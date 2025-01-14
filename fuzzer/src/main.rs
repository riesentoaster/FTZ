#![recursion_limit = "1024"] // too complex types in mutators

#[allow(unused_imports)]
use runner::{connect_to_zephyr, fuzz};

mod cli;
mod direction;
mod layers;
mod packets;
mod pcap;
mod runner;
mod shmem;
mod smoltcp;

pub const NETWORK_SHMEM_SIZE: usize = 1600;
pub const COV_SHMEM_SIZE: usize = 26612; // manually extracted
pub const PCAP_PATH: &str = "./pcap.pcap";

fn main() {
    env_logger::builder()
        .target(env_logger::Target::Stdout)
        .init();

    fuzz();
    // let opt = Cli::parse();
    // let packets = connect_to_zephyr(
    //     b"Hello, World!",
    //     opt.zephyr_exec_dir(),
    //     opt.zephyr_out_dir(),
    //     0,
    //     NETWORK_SHMEM_SIZE,
    //     Duration::from_secs(10),
    // )
    // .unwrap();

    // let mut pcap_file = File::create(PCAP_PATH).unwrap();
    // write_pcap(
    //     &packets.iter().map(|(d, p)| (d, p)).collect::<Vec<_>>(),
    //     &mut pcap_file,
    // )
    // .unwrap();
}

#[allow(unused)]
fn wait_for_newline() {
    std::io::stdin().read_line(&mut String::new()).unwrap();
}
