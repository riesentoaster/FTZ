use crate::{
    cli::Cli,
    packets::outgoing_tcp_packets,
    runner::{
        input::{FixedZephyrInputGenerator, ZephyrInput, ZephyrInputType},
        objective::CrashLoggingFeedback,
        observer::state::PacketState,
        PacketMetadataFeedback, PacketObserver, ZepyhrExecutor,
    },
    shmem::get_shmem,
    COV_SHMEM_SIZE, NETWORK_SHMEM_SIZE,
};
use clap::Parser as _;
use libafl::{
    corpus::{Corpus, OnDiskCorpus},
    events::{CentralizedEventManager, CentralizedLauncher, EventConfig, EventRestarter},
    feedback_and, feedback_or_fast,
    feedbacks::{ConstFeedback, MaxMapFeedback, TimeFeedback},
    monitors::OnDiskTomlMonitor,
    mutators::StdMOptMutator,
    observers::{CanTrack, ConstMapObserver, HitcountsMapObserver, TimeObserver},
    schedulers::{powersched::PowerSchedule, StdWeightedScheduler},
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

pub fn fuzz() {
    log::info!("Initializing fuzzer");
    let opt = Cli::parse();

    let zephyr_exec_path = opt.zephyr_exec_dir();

    let run_client = |_primary| {
        let opt = &opt;
        move |state: Option<_>,
              mut manager: CentralizedEventManager<_, _, _, _>,
              client_description| {
            log::info!("Initializing fuzzing client");

            let mut cov_shmem = get_shmem(COV_SHMEM_SIZE, &client_description, "cov")?;
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

            let mut packet_observer = PacketObserver::new();
            let packet_observer_handle = packet_observer.handle().clone();

            let state_map = packet_observer.get_states_mut();
            let state_observer_raw = unsafe {
                ConstMapObserver::from_mut_ptr(
                    "state-observer",
                    NonNull::new(state_map.as_mut_ptr())
                        .expect("map ptr is null")
                        .cast::<[u8; PacketState::max_numeric_value() as usize + 1]>(),
                )
            };

            let state_observer = HitcountsMapObserver::new(state_observer_raw).track_indices();
            let state_feedback = MaxMapFeedback::new(&state_observer);

            let cov_feedback = MaxMapFeedback::new(&cov_observer);

            let mut feedback = feedback_or_fast!(
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
            // let mutations = tuple_list!()
            //     .merge(
            //         int_mutators_no_crossover()
            //             .map(ToMappingMutator::new(ParsedZephyrInput::ipv4_version_mut)),
            //     )
            //     .merge(int_mutators_no_crossover().map(ToMappingMutator::new(
            //         ParsedZephyrInput::ipv4_header_length_mut,
            //     )))
            //     .merge(
            //         int_mutators_no_crossover()
            //             .map(ToMappingMutator::new(ParsedZephyrInput::ipv4_dscp_mut)),
            //     )
            //     .merge(
            //         int_mutators_no_crossover()
            //             .map(ToMappingMutator::new(ParsedZephyrInput::ipv4_ecn_mut)),
            //     )
            //     .merge(int_mutators_no_crossover().map(ToMappingMutator::new(
            //         ParsedZephyrInput::ipv4_total_length_mut,
            //     )))
            //     .merge(int_mutators_no_crossover().map(ToMappingMutator::new(
            //         ParsedZephyrInput::ipv4_identification_mut,
            //     )))
            //     .merge(
            //         int_mutators_no_crossover()
            //             .map(ToMappingMutator::new(ParsedZephyrInput::ipv4_flags_mut)),
            //     )
            //     .merge(int_mutators_no_crossover().map(ToMappingMutator::new(
            //         ParsedZephyrInput::ipv4_fragment_offset_mut,
            //     )))
            //     .merge(
            //         int_mutators_no_crossover()
            //             .map(ToMappingMutator::new(ParsedZephyrInput::ipv4_ttl_mut)),
            //     )
            //     .merge(
            //         int_mutators_no_crossover()
            //             .map(ToMappingMutator::new(ParsedZephyrInput::tcp_source_mut)),
            //     )
            //     .merge(int_mutators_no_crossover().map(ToMappingMutator::new(
            //         ParsedZephyrInput::tcp_destination_mut,
            //     )))
            //     .merge(
            //         int_mutators_no_crossover()
            //             .map(ToMappingMutator::new(ParsedZephyrInput::tcp_sequence_mut)),
            //     )
            //     .merge(int_mutators_no_crossover().map(ToMappingMutator::new(
            //         ParsedZephyrInput::tcp_acknowledgement_mut,
            //     )))
            //     .merge(int_mutators_no_crossover().map(ToMappingMutator::new(
            //         ParsedZephyrInput::tcp_data_offset_mut,
            //     )))
            //     .merge(
            //         int_mutators_no_crossover()
            //             .map(ToMappingMutator::new(ParsedZephyrInput::tcp_reserved_mut)),
            //     )
            //     .merge(
            //         int_mutators_no_crossover()
            //             .map(ToMappingMutator::new(ParsedZephyrInput::tcp_flags_mut)),
            //     )
            //     .merge(
            //         int_mutators_no_crossover()
            //             .map(ToMappingMutator::new(ParsedZephyrInput::tcp_window_mut)),
            //     )
            //     .merge(
            //         int_mutators_no_crossover()
            //             .map(ToMappingMutator::new(ParsedZephyrInput::tcp_urgent_ptr_mut)),
            //     )
            //     .merge(
            //         havoc_mutations_no_crossover()
            //             .map(ToMappingMutator::new(ParsedZephyrInput::tcp_payload_mut)),
            //     );

            let mutator =
                StdMutationalStage::new(StdMOptMutator::new(&mut state, mutations, 7, 5)?);

            let scheduler = StdWeightedScheduler::with_schedule(
                &mut state,
                &state_observer,
                Some(PowerSchedule::fast()),
            );

            let mut fuzzer = StdFuzzer::new(scheduler, feedback, objective);

            let outgoing_packets = outgoing_tcp_packets();
            let mut generator = FixedZephyrInputGenerator::new(outgoing_packets, false);

            let mut observers =
                tuple_list!(cov_observer, time_observer, packet_observer, state_observer);

            let mut executor = ZepyhrExecutor::new(
                &mut observers,
                packet_observer_handle,
                &cov_shmem_description,
                zephyr_exec_path.to_path_buf(),
                opt.zephyr_out_dir().map(PathBuf::to_owned),
                NETWORK_SHMEM_SIZE,
                &client_description,
            )?;

            if state.must_load_initial_inputs() {
                log::debug!("Generating inputs");
                state.generate_initial_inputs(
                    &mut fuzzer,
                    &mut executor,
                    &mut generator,
                    &mut manager,
                    outgoing_tcp_packets().len() - 1,
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
                let mut stages = tuple_list!(mutator);
                fuzzer.fuzz_one(&mut stages, &mut executor, &mut state, &mut manager)?;
                manager.send_exiting()?;
                return Err(Error::shutting_down());
            } else {
                let mut stages = tuple_list!(mutator);
                fuzzer.fuzz_loop(&mut stages, &mut executor, &mut state, &mut manager)?;
            }
            Ok(())
        }
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
        .main_run_client(&mut run_client(true))
        .secondary_run_client(&mut run_client(false))
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
