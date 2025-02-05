#![recursion_limit = "1024"] // too complex types in mutators

#[allow(unused_imports)]
use runner::{connect_to_zephyr, fuzz};

pub mod cli;
pub mod direction;
pub mod layers;
pub mod packets;
pub mod pcap;
pub mod runner;
pub mod shmem;
pub mod smoltcp;

pub const NETWORK_SHMEM_SIZE: usize = 1600;
pub const COV_SHMEM_SIZE: usize = 26860; // manually extracted
pub const PCAP_PATH: &str = "./pcap.pcap";

#[allow(unused)]
fn wait_for_newline() {
    std::io::stdin().read_line(&mut String::new()).unwrap();
}
