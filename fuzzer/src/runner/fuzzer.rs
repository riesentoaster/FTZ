use crate::{
    cli::Cli,
    packets::outgoing_tcp_packets,
    runner::{
        feedback::sparse::SparseMapFeedback,
        input::{FixedZephyrInputGenerator, ZephyrInput, ZephyrInputType},
        objective::CrashLoggingFeedback,
        PacketMetadataFeedback, PacketObserver, ZepyhrExecutor,
    },
    shmem::get_shmem,
    COV_SHMEM_SIZE, NETWORK_SHMEM_SIZE,
};
use clap::Parser as _;
use libafl::{
    corpus::{Corpus, OnDiskCorpus},
    events::{
        CentralizedEventManager, CentralizedLauncher, ClientDescription, EventConfig, ManagerExit,
    },
    feedback_and, feedback_or_fast,
    feedbacks::{ConstFeedback, MaxMapFeedback, TimeFeedback},
    monitors::OnDiskJsonAggregateMonitor,
    mutators::StdMOptMutator,
    observers::{CanTrack, ConstMapObserver, HitcountsMapObserver, TimeObserver},
    stages::StdMutationalStage,
    state::{HasCorpus as _, StdState},
    Error, Fuzzer as _, StdFuzzer,
};
use libafl_bolts::{
    core_affinity::Cores,
    rands::StdRand,
    shmem::{ShMem, ShMemProvider as _, StdShMemProvider},
    tuples::{tuple_list, Handled},
};
use std::{path::PathBuf, ptr::NonNull, time::Duration};

#[cfg(feature = "monitor_memory")]
use crate::runner::feedback::memory::MemoryPseudoFeedback;
#[cfg(feature = "monitor_tui")]
use libafl::monitors::tui::TuiMonitor;
#[cfg(feature = "monitor_stdout")]
use libafl::monitors::MultiMonitor;
#[cfg(feature = "monitor_none")]
use libafl::monitors::NopMonitor;
#[cfg(not(feature = "coverage_stability"))]
use libafl::schedulers::{powersched::PowerSchedule, StdWeightedScheduler};
#[cfg(feature = "coverage_stability")]
use {
    crate::runner::calibration_log_stage::CalibrationLogStage,
    libafl::{schedulers::StdScheduler, stages::CalibrationStage},
};

pub fn fuzz() {
    log::info!("Initializing fuzzer");
    let opt = Cli::parse();

    let zephyr_exec_path = opt.zephyr_exec_dir();

    let run_client = |_primary| {
        let opt = &opt;
        move |state: Option<_>,
              mut manager: CentralizedEventManager<_, _, _, _>,
              client_description: ClientDescription| {
            log::info!("Initializing fuzzing client");

            let mut cov_shmem = get_shmem(COV_SHMEM_SIZE, client_description.id(), "cov")?;
            let cov_shmem_description = cov_shmem.description();

            let cov_raw_observer = unsafe {
                ConstMapObserver::from_mut_ptr(
                    "coverage-observer",
                    NonNull::new(cov_shmem.as_mut_ptr())
                        .expect("map ptr is null")
                        .cast::<[u8; COV_SHMEM_SIZE]>(),
                )
            };

            let cov_observer = HitcountsMapObserver::new(cov_raw_observer).track_indices();
            let time_observer = TimeObserver::new("time-observer");

            let packet_observer = PacketObserver::new(opt.state_diff());
            let packet_observer_handle = packet_observer.handle().clone();

            let state_feedback = SparseMapFeedback::new(&packet_observer, "state-observer");

            let cov_feedback = MaxMapFeedback::new(&cov_observer);
            #[cfg(feature = "coverage_stability")]
            let stability = CalibrationStage::new(&cov_feedback);

            #[cfg(not(feature = "monitor_memory"))]
            let mut feedback = feedback_or_fast!(
                TimeFeedback::new(&time_observer),
                PacketMetadataFeedback::new(packet_observer_handle.clone()),
                // CovLogFeedback::new(cov_observer.handle(), client_description.id()),
                feedback_and!(cov_feedback, ConstFeedback::new(false)),
                state_feedback
            );

            #[cfg(feature = "monitor_memory")]
            let mut feedback = feedback_or_fast!(
                MemoryPseudoFeedback,
                TimeFeedback::new(&time_observer),
                PacketMetadataFeedback::new(packet_observer_handle.clone()),
                // CovLogFeedback::new(cov_observer.handle(), client_description.id()),
                feedback_and!(cov_feedback, ConstFeedback::new(false)),
                state_feedback
            );

            let mut objective = feedback_or_fast!(
                TimeFeedback::new(&time_observer),
                CrashLoggingFeedback::new(),
            );

            let solutions = OnDiskCorpus::new("./solutions")?;

            let corpus = OnDiskCorpus::new("corpus")?;

            let mut state: StdState<ZephyrInputType, _, _, _> = state.unwrap_or_else(|| {
                StdState::new(
                    StdRand::new(),
                    corpus,
                    solutions,
                    &mut feedback,
                    &mut objective,
                )
                .expect("Could not create state")
            });

            let mutations = ZephyrInputType::mutators();

            let mutator =
                StdMutationalStage::new(StdMOptMutator::new(&mut state, mutations, 7, 5)?);

            #[cfg(feature = "coverage_stability")]
            let unstable_coverage_log_stage = CalibrationLogStage::new("unstable-coverage.txt");
            #[cfg(feature = "coverage_stability")]
            let mut stages = tuple_list!(stability, unstable_coverage_log_stage, mutator);
            #[cfg(not(feature = "coverage_stability"))]
            let mut stages = tuple_list!(mutator);

            #[cfg(not(feature = "coverage_stability"))]
            let scheduler = StdWeightedScheduler::with_schedule(
                &mut state,
                &packet_observer,
                Some(PowerSchedule::fast()),
            );

            // StdWeightedScheduler is not compatible with CalibrationStage
            #[cfg(feature = "coverage_stability")]
            let scheduler = StdScheduler::new();

            let mut fuzzer = StdFuzzer::new(scheduler, feedback, objective);

            let mut observers = tuple_list!(cov_observer, time_observer, packet_observer);

            let mut executor = ZepyhrExecutor::new(
                &mut observers,
                packet_observer_handle,
                &cov_shmem_description,
                zephyr_exec_path.to_path_buf(),
                opt.zephyr_out_dir().map(PathBuf::to_owned),
                NETWORK_SHMEM_SIZE,
                client_description.id(),
            )?;

            if state.must_load_initial_inputs() {
                let outgoing_packets = outgoing_tcp_packets();
                let outgoing_packets_len = outgoing_packets.len();
                let mut generator = FixedZephyrInputGenerator::new(outgoing_packets, false);

                log::debug!(
                    "Generating inputs from fixed trace, expecting {} packets",
                    outgoing_packets_len
                );

                state.generate_initial_inputs_forced(
                    &mut fuzzer,
                    &mut executor,
                    &mut generator,
                    &mut manager,
                    outgoing_packets_len,
                )?;

                log::info!("Generated {} inputs", state.corpus().count());
            } else {
                log::warn!("Did not need to load initial inputs");
            }

            log::info!("Starting Fuzzing");

            if opt.load_only() {
                manager.send_exiting()?;
                return Err(Error::shutting_down());
            } else if opt.fuzz_one() {
                fuzzer.fuzz_one(&mut stages, &mut executor, &mut state, &mut manager)?;
                manager.send_exiting()?;
                return Err(Error::shutting_down());
            } else {
                fuzzer.fuzz_loop(&mut stages, &mut executor, &mut state, &mut manager)?;
            }
            Ok(())
        }
    };

    #[cfg(feature = "monitor_tui")]
    let monitor = {
        TuiMonitor::builder()
            .title("Zephyr TCP/IP Stack Fuzzer")
            .build()
    };

    #[cfg(feature = "monitor_stdout")]
    let monitor = MultiMonitor::new(|m| println!("{m}"));

    #[cfg(feature = "monitor_none")]
    let monitor = NopMonitor::new();

    let json_path = format!("{}.json", opt.monitor());
    if std::path::Path::new(&json_path).exists() {
        println!("Monitor file already exists: {}, exiting", json_path);
        return;
    }
    let monitor =
        OnDiskJsonAggregateMonitor::with_interval(json_path, monitor, Duration::from_secs(1));

    let (cores, overcommit) = if opt.fuzz_one() || opt.load_only() {
        (Cores::from_cmdline("1").unwrap(), 1)
    } else {
        (opt.cores().clone(), opt.overcommit())
    };

    match CentralizedLauncher::builder()
        .shmem_provider(StdShMemProvider::new().expect("Failed to init shared memory"))
        .configuration(EventConfig::from_name("default"))
        .monitor(monitor)
        .main_run_client(&mut run_client(true))
        .secondary_run_client(&mut run_client(false))
        .cores(&cores)
        .overcommit(overcommit)
        .stdout_file(opt.stdout().and_then(|e| e.as_os_str().to_str()))
        .stderr_file(opt.stderr().and_then(|e| e.as_os_str().to_str()))
        .launch_delay(20)
        .build()
        .launch()
    {
        Ok(()) => (),
        Err(Error::ShuttingDown) => log::info!("Fuzzing stopped by user. Good bye."),
        Err(err) => panic!("Failed to run launcher: {}", err),
    }
}
