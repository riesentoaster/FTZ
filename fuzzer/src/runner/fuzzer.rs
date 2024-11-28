use crate::{
    cli::Cli,
    runner::{
        objective::CrashLoggingFeedback, PacketFeedback, PacketObserver,
        ZephyrInteractionGenerator, ZepyhrExecutor,
    },
    shmem::get_shmem,
    COV_SHMEM_SIZE, NETWORK_SHMEM_SIZE,
};
use clap::Parser as _;
use libafl::{
    corpus::{Corpus, OnDiskCorpus},
    events::{CentralizedEventManager, CentralizedLauncher, EventConfig, EventRestarter},
    feedback_or_fast,
    feedbacks::{MaxMapFeedback, TimeFeedback},
    monitors::OnDiskTomlMonitor,
    mutators::{havoc_mutations, StdMOptMutator},
    observers::{CanTrack, ConstMapObserver, HitcountsMapObserver, TimeObserver},
    schedulers::StdScheduler,
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

#[cfg(feature = "monitor_tui")]
use libafl::monitors::tui::TuiMonitor;
#[cfg(feature = "monitor_stdout")]
use libafl::monitors::MultiMonitor;
#[cfg(feature = "monitor_none")]
use libafl::monitors::NopMonitor;

#[cfg(not(feature = "ondisk_corpus"))]
use libafl::corpus::InMemoryCorpus;

#[cfg(feature = "stability")]
use libafl::stages::CalibrationStage;

pub fn fuzz() {
    log::info!("Initializing fuzzer");
    let opt = Cli::parse();

    let zephyr_exec_path = opt.zephyr_exec_dir();

    let mut run_client = |state: Option<_>,
                          mut manager: CentralizedEventManager<_, _, _, _>,
                          _core_id| {
        log::info!("Initializing fuzzing client");
        let mut rand = StdRand::new();

        let mut cov_shmem = get_shmem(COV_SHMEM_SIZE, &mut rand)?;
        let cov_shmem_description = cov_shmem.description();

        let cov_raw_observer = unsafe {
            ConstMapObserver::from_mut_ptr(
                "coverage_observer",
                NonNull::new(cov_shmem.as_mut_ptr())
                    .expect("map ptr is null")
                    .cast::<[u8; COV_SHMEM_SIZE]>(),
            )
        };

        let cov_observer = HitcountsMapObserver::new(cov_raw_observer).track_indices();
        let time_observer = TimeObserver::new("time_observer");
        let packet_observer = PacketObserver::new();
        let packet_observer_handle = packet_observer.handle();
        let cov_feedback = MaxMapFeedback::new(&cov_observer);
        #[cfg(feature = "stability")]
        let calibration_stage = CalibrationStage::new(&cov_feedback);

        let mut feedback = feedback_or_fast!(
            TimeFeedback::new(&time_observer),
            PacketFeedback::new(&packet_observer),
            cov_feedback,
        );

        let mut objective = feedback_or_fast!(
            TimeFeedback::new(&time_observer),
            PacketFeedback::new(&packet_observer),
            CrashLoggingFeedback::new(),
        );

        let mut observers = tuple_list!(cov_observer, time_observer, packet_observer);

        let solutions = OnDiskCorpus::new("./solutions")?;

        #[cfg(feature = "ondisk_corpus")]
        let corpus = OnDiskCorpus::new("./corpus")?;
        #[cfg(not(feature = "ondisk_corpus"))]
        let corpus = InMemoryCorpus::new();

        let mut state = state.unwrap_or_else(|| {
            StdState::new(rand, corpus, solutions, &mut feedback, &mut objective)
                .expect("Could not create state")
        });

        let mutations = havoc_mutations();
        let mutator = StdMutationalStage::new(StdMOptMutator::new(&mut state, mutations, 7, 5)?);

        #[cfg(not(feature = "stability"))]
        let mut stages = tuple_list!(mutator);
        #[cfg(feature = "stability")]
        let mut stages = tuple_list!(mutator, calibration_stage);

        let scheduler = StdScheduler::new();

        let mut fuzzer = StdFuzzer::new(scheduler, feedback, objective);

        let mut generator = ZephyrInteractionGenerator::new();

        let mut executor = ZepyhrExecutor::new(
            &mut observers,
            packet_observer_handle,
            &cov_shmem_description,
            zephyr_exec_path.to_path_buf(),
            opt.zephyr_out_dir().map(PathBuf::to_owned),
            NETWORK_SHMEM_SIZE,
            &mut rand,
        )?;

        if state.must_load_initial_inputs() {
            log::debug!("Generating inputs");
            state.generate_initial_inputs(
                &mut fuzzer,
                &mut executor,
                &mut generator,
                &mut manager,
                1,
            )?;
            log::info!("Generated {} inputs", state.corpus().count());
        } else {
            log::info!("Did not need to load initial inputs");
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
    };

    #[cfg(feature = "monitor_tui")]
    let base_monitor = {
        TuiMonitor::builder()
            .title("Zephyr TCP/IP Stack Fuzzer")
            .build()
    };

    #[cfg(feature = "monitor_stdout")]
    let base_monitor = MultiMonitor::new(|m| println!("{m}"));
    #[cfg(feature = "monitor_none")]
    let base_monitor = NopMonitor::new();

    let monitor = OnDiskTomlMonitor::with_update_interval(
        "./monitor.toml",
        base_monitor,
        Duration::from_secs(10),
    );

    let cores = if opt.fuzz_one() {
        Cores::from_cmdline("1").unwrap()
    } else {
        opt.cores().clone()
    };

    let overcommit = if opt.fuzz_one() { 1 } else { opt.overcommit() };

    match CentralizedLauncher::builder()
        .shmem_provider(StdShMemProvider::new().expect("Failed to init shared memory"))
        .configuration(EventConfig::from_name("default"))
        .monitor(monitor)
        .main_run_client(&mut run_client.clone())
        .secondary_run_client(&mut run_client)
        .cores(&cores)
        .overcommit(overcommit)
        .stdout_file(opt.stdout().and_then(|e| e.as_os_str().to_str()))
        .stderr_file(opt.stderr().and_then(|e| e.as_os_str().to_str()))
        .launch_delay(200)
        .build()
        .launch()
    {
        Ok(()) => (),
        Err(Error::ShuttingDown) => log::info!("Fuzzing stopped by user. Good bye."),
        Err(err) => panic!("Failed to run launcher: {}", err),
    }
}
