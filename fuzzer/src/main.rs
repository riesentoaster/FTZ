#![recursion_limit = "1024"]
use std::io::{self, Write};

use runner::fuzz;

mod cli;
mod direction;
mod layers;
mod packets;
mod pcap;
mod runner;
mod shmem;
mod smoltcp;

pub const NETWORK_SHMEM_SIZE: usize = 1600;
pub const COV_SHMEM_SIZE: usize = 25632; // manually extracted
pub const PCAP_PATH: &str = "./pcap.pcap";

fn main() {
    env_logger::builder()
        .target(env_logger::Target::Stdout)
        .init();
    fuzz();

    // let opt = Opt::parse();
    // let mut cov_shmem = MmapShMemProvider::new()
    //     .unwrap()
    //     .new_shmem(COV_SHMEM_SIZE)
    //     .unwrap();
    // let cov_raw_observer = unsafe {
    //     StdMapObserver::from_mut_ptr("coverage_observer", cov_shmem.as_mut_ptr(), cov_shmem.len())
    // };
    // let cov_observer = HitcountsMapObserver::new(cov_raw_observer).track_indices();
    // let time_observer = TimeObserver::new("time");
    // let time_feedback = TimeFeedback::new(&time_observer);
    // let cov_feedback = MaxMapFeedback::new(&cov_observer);
    // let cov_observer_handle = cov_observer.handle();
    // let time_observer_handle = time_observer.handle();
    // let observers = tuple_list!(cov_observer, time_observer);
    // cov_shmem.persist_for_child_processes().unwrap();
    // let mut executor = ZepyhrExecutor::new(
    //     observers,
    //     cov_shmem,
    //     opt.zephyr_exec_dir().to_path_buf(),
    //     NETWORK_SHMEM_SIZE,
    // );

    // let mut state = StdState::new(
    //     StdRand::new(),
    //     InMemoryCorpus::new(),
    //     InMemoryCorpus::new(),
    //     &mut feedback_or!(time_feedback, cov_feedback),
    //     &mut (),
    // )
    // .unwrap();
    // let mut input = MultipartInput::new();
    // input.add_part("test".to_owned(), BytesInput::from(vec![1, 2, 3]));
    // let res = executor
    //     .run_target(
    //         &mut NopFuzzer::new(),
    //         &mut state,
    //         &mut NopEventManager::new(),
    //         &input,
    //     )
    //     .unwrap();
    // println!("{:?}", res);
    // println!("{:?}", input);
    // println!(
    //     "{:?}",
    //     executor
    //         .observers()
    //         .get(&time_observer_handle)
    //         .unwrap()
    //         .last_runtime()
    // );

    // dump_global_packets_to_pcap_file(PCAP_PATH, true).unwrap();
}

#[allow(unused)]
fn wait_for_newline() {
    let mut input = String::new();
    io::stdout().flush().unwrap();
    io::stdin().read_line(&mut input).unwrap();
}
