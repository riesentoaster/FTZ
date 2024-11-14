use crate::{
    cli::Cli,
    runner::{
        CorpusEnum, PacketFeedback, PacketObserver, ZephyrInteractionGenerator, ZepyhrExecutor,
    },
    COV_SHMEM_SIZE, NETWORK_SHMEM_SIZE,
};
use clap::Parser as _;
use libafl::{
    corpus::{Corpus, OnDiskCorpus},
    events::{EventConfig, EventRestarter, Launcher, LlmpRestartingEventManager},
    feedback_or, feedback_or_fast,
    feedbacks::{CrashFeedback, MaxMapFeedback, TimeFeedback, TimeoutFeedback},
    monitors::OnDiskTomlMonitor,
    mutators::{havoc_mutations, StdScheduledMutator},
    observers::{CanTrack, HitcountsMapObserver, StdMapObserver, TimeObserver},
    schedulers::StdScheduler,
    stages::StdMutationalStage,
    state::{HasCorpus as _, StdState},
    Error, Fuzzer as _, StdFuzzer,
};
use libafl_bolts::{
    core_affinity::Cores,
    rands::{Rand as _, StdRand},
    shmem::{MmapShMemProvider, ShMem, ShMemProvider as _, StdShMemProvider},
    tuples::{tuple_list, Handled},
    AsSliceMut,
};
use std::{fs, time::Duration};

#[cfg(feature = "tui")]
use libafl::monitors::tui::TuiMonitor;
#[cfg(not(feature = "tui"))]
use libafl::monitors::MultiMonitor;

pub fn fuzz() {
    log::info!("Initializing fuzzer");
    let opt = Cli::parse();

    if let Some(corpus_dir) = opt.corpus_dir() {
        if corpus_dir.exists() {
            log::warn!("Removing previous corpus entries");
            fs::remove_dir_all(corpus_dir).unwrap();
        }
    }

    let zephyr_exec_path = opt.zephyr_exec_dir();

    let mut shmem_provider = MmapShMemProvider::default();

    let mut run_client = |state: Option<_>,
                          mut manager: LlmpRestartingEventManager<_, _, _>,
                          _core_id| {
        log::info!("Initializing fuzzing client");
        let mut cov_shmem = shmem_provider.new_shmem_persistent(COV_SHMEM_SIZE)?;
        let cov_shmem_description = cov_shmem.description();

        let cov_raw_observer =
            StdMapObserver::from_mut_slice("coverage_observer", cov_shmem.as_slice_mut().into());

        let cov_observer = HitcountsMapObserver::new(cov_raw_observer).track_indices();
        let time_observer = TimeObserver::new("time_observer");
        let packet_observer = PacketObserver::new();
        let packet_observer_handle = packet_observer.handle();

        let mut feedback = feedback_or!(
            MaxMapFeedback::new(&cov_observer),
            TimeFeedback::new(&time_observer),
            PacketFeedback::new(&packet_observer),
        );

        let mut objective = feedback_or_fast!(CrashFeedback::new(), TimeoutFeedback::new());
        let observers = tuple_list!(cov_observer, time_observer, packet_observer);

        let solutions = OnDiskCorpus::new(opt.solutions_dir())?;
        let corpus = CorpusEnum::new(opt.corpus_dir())?;

        let mut state = state.unwrap_or_else(|| {
            StdState::new(
                StdRand::new(),
                corpus,
                solutions,
                &mut feedback,
                &mut objective,
            )
            .expect("Could not create state")
        });

        let mut stages = tuple_list!(StdMutationalStage::new(StdScheduledMutator::new(
            havoc_mutations()
        )));

        let scheduler = StdScheduler::new();

        let mut fuzzer = StdFuzzer::new(scheduler, feedback, objective);

        let mut generator = ZephyrInteractionGenerator::new(
            NETWORK_SHMEM_SIZE,
            &cov_shmem_description,
            zephyr_exec_path,
        )?;

        let mut executor = ZepyhrExecutor::new(
            observers,
            packet_observer_handle,
            &cov_shmem_description,
            zephyr_exec_path.to_path_buf(),
            NETWORK_SHMEM_SIZE,
        )?;

        if state.must_load_initial_inputs() {
            if opt.resume() {
                let corpus_dir = opt
                    .corpus_dir()
                    .expect("The corpus directory needs to be specified when resuming")
                    .to_path_buf();

                log::info!("Loading inputs from disk at {:#?}", corpus_dir);

                state.load_initial_inputs(
                    &mut fuzzer,
                    &mut executor,
                    &mut manager,
                    &[corpus_dir],
                )?;

                log::info!("Loaded {} inputs from disk", state.corpus().count());
            } else {
                log::info!("Generating inputs");
                state.generate_initial_inputs(
                    &mut fuzzer,
                    &mut executor,
                    &mut generator,
                    &mut manager,
                    1,
                )?;
                log::info!("Generated {} inputs", state.corpus().count());
            }
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

    #[cfg(feature = "tui")]
    let base_monitor = TuiMonitor::builder()
        .title("Zephyr TCP/IP Stack Fuzzer")
        .build();

    #[cfg(not(feature = "tui"))]
    let base_monitor = {
        let mut rand = StdRand::new();
        MultiMonitor::new(move |m| {
            if rand.next() % 10 == 0 && m.contains("GLOBAL") {
                log::info!("{m}")
            }
        })
    };

    let monitor = OnDiskTomlMonitor::with_update_interval(
        opt.monitor_path(),
        base_monitor,
        Duration::from_secs(10),
    );

    let cores = if opt.fuzz_one() {
        Cores::from_cmdline("1").unwrap()
    } else {
        opt.cores().clone()
    };

    let overcommit = if opt.fuzz_one() { 1 } else { opt.overcommit() };

    match Launcher::builder()
        .shmem_provider(StdShMemProvider::new().expect("Failed to init shared memory"))
        .configuration(EventConfig::from_name("default"))
        .monitor(monitor)
        .run_client(&mut run_client)
        .cores(&cores)
        .overcommit(overcommit)
        .broker_port(opt.broker_port())
        .remote_broker_addr(opt.remote_broker_addr())
        .stdout_file(opt.stdout().and_then(|e| e.as_os_str().to_str()))
        .stderr_file(opt.stderr().and_then(|e| e.as_os_str().to_str()))
        .build()
        .launch()
    {
        Ok(()) => (),
        Err(Error::ShuttingDown) => log::info!("Fuzzing stopped by user. Good bye."),
        Err(err) => panic!("Failed to run launcher: {err:?}"),
    }
}
