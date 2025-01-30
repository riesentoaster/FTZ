use crate::{
    cli::Cli,
    packets::outgoing_tcp_packets,
    runner::{
        feedback::{
            corpus_dir_count::CorpusDirCountFeedback, input_len::InputLenFeedback,
            memory::MemoryPseudoFeedback,
        },
        generator::{
            fixed::{FixedZephyrInputGenerator, FixedZephyrInputPartGenerator},
            random::RandomTcpZephyrInputPartGenerator,
        },
        input::{appending::ToAppendingMutatorWrapper, ZephyrInput, ZephyrInputType},
        objective::CrashLoggingFeedback,
        PacketMetadataFeedback, PacketObserver, ZepyhrExecutor,
    },
    shmem::get_shmem,
    COV_SHMEM_SIZE, NETWORK_SHMEM_SIZE,
};
use clap::Parser as _;
use libafl::{
    corpus::{Corpus, InMemoryCorpus, OnDiskCorpus},
    events::{
        CentralizedEventManager, CentralizedLauncher, ClientDescription, EventConfig,
        SendExiting as _,
    },
    feedback_and, feedback_and_fast, feedback_or_fast,
    feedbacks::{ConstFeedback, MaxMapFeedback, TimeFeedback},
    fuzzer::{replaying::ReplayingFuzzer, Evaluator as _, Fuzzer as _},
    generators::Generator as _,
    monitors::OnDiskJsonAggregateMonitor,
    mutators::StdMOptMutator,
    observers::{CanTrack, ConstMapObserver, HitcountsMapObserver, StdMapObserver, TimeObserver},
    stages::StdMutationalStage,
    state::{HasCorpus as _, StdState},
    Error,
};
use libafl_bolts::{
    core_affinity::Cores,
    rands::StdRand,
    shmem::{ShMem, ShMemProvider as _, StdShMemProvider},
    tuples::{tuple_list, Handled as _, Map as _, Merge as _},
};
use std::{path::PathBuf, ptr::NonNull, time::Duration};

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

    let run_client = |_primary: bool| {
        let opt = &opt;
        move |state: Option<_>,
              mut manager: CentralizedEventManager<_, _, _, _, _, _>,
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

            let mut packet_observer = PacketObserver::new(opt.state_diff());
            let state_map = packet_observer.get_state_map();
            let state_map_observer = unsafe {
                let state_map_len = state_map.len();
                StdMapObserver::from_mut_ptr(
                    "state-map-observer",
                    state_map.as_mut_ptr(),
                    state_map_len,
                )
            };
            let state_feedback = MaxMapFeedback::new(&state_map_observer);
            let packet_observer_handle = packet_observer.handle();

            let cov_feedback = MaxMapFeedback::new(&cov_observer);
            #[cfg(feature = "coverage_stability")]
            let stability = CalibrationStage::new(&cov_feedback);

            let should_have_gated_feedbacks =
                opt.cores().ids.len() * opt.overcommit() == client_description.id();

            if should_have_gated_feedbacks {
                log::info!("Client {:?} gets gated feedbacks", client_description);
            }

            let gated_feedbacks = feedback_and_fast!(
                ConstFeedback::new(should_have_gated_feedbacks),
                feedback_or_fast!(
                    MemoryPseudoFeedback::new(Duration::from_secs(10)),
                    CorpusDirCountFeedback::new(opt.corpus_dir(), Duration::from_secs(10))
                )
            );

            let mut feedback = feedback_or_fast!(
                gated_feedbacks,
                TimeFeedback::new(&time_observer),
                PacketMetadataFeedback::new(packet_observer_handle.clone()),
                InputLenFeedback,
                // only log coverage
                feedback_and!(cov_feedback, ConstFeedback::new(false)),
                state_feedback
            );

            let mut objective = feedback_or_fast!(
                TimeFeedback::new(&time_observer),
                CrashLoggingFeedback::new(),
            );

            let solutions = OnDiskCorpus::new(opt.solutions_dir())?;

            let corpus = InMemoryCorpus::new();

            let mut state: StdState<_, _, _, _> = state.unwrap_or_else(|| {
                StdState::new(
                    StdRand::new(),
                    corpus,
                    solutions,
                    &mut feedback,
                    &mut objective,
                )
                .expect("Could not create state")
            });

            let mutations = ZephyrInputType::mutators().merge(
                tuple_list!(
                    FixedZephyrInputPartGenerator::new(outgoing_tcp_packets(), true),
                    RandomTcpZephyrInputPartGenerator
                )
                .map(ToAppendingMutatorWrapper),
            );

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
                &state_map_observer,
                Some(PowerSchedule::fast()),
            );

            // StdWeightedScheduler is not compatible with CalibrationStage
            #[cfg(feature = "coverage_stability")]
            let scheduler = StdScheduler::new();

            let mut fuzzer = ReplayingFuzzer::new(
                3,
                2.1,
                10,
                true,
                state_map_observer.handle(),
                scheduler,
                feedback,
                objective,
            );

            let mut observers = tuple_list!(
                cov_observer,
                time_observer,
                packet_observer,
                state_map_observer
            );

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
                let mut generator = FixedZephyrInputGenerator::new(outgoing_packets, true);

                log::debug!(
                    "Generating inputs from fixed trace, expecting {} packets",
                    outgoing_packets_len
                );

                state.generate_initial_inputs_forced(
                    &mut fuzzer,
                    &mut executor,
                    &mut generator,
                    &mut manager,
                    outgoing_packets_len + 1,
                )?;
                log::info!(
                    "Added {} inputs to corpus, now evaluating them to seed rest of fuzzer",
                    state.corpus().count()
                );

                for _i in 0..=outgoing_packets_len {
                    let input = generator.generate(&mut state)?;
                    fuzzer.evaluate_input(&mut state, &mut executor, &mut manager, input)?;
                }

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
            // } else if manager.is_main() {
            //     fuzzer.fuzz_loop(&mut tuple_list!(), &mut executor, &mut state, &mut manager)?;
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
        .launch_delay(89)
        .build()
        .launch()
    {
        Ok(()) => (),
        Err(e) => match e {
            Error::ShuttingDown => log::info!("Fuzzing stopped by user. Good bye."),
            _ => log::warn!("--------------------------------\nFailed to run launcher:\n{}\n--------------------------------", e),
        },
    }
}
