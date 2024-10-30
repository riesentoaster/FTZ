mod direction;

mod layers;
mod packets;
mod pcap;
mod runner;
mod smoltcp;

use std::env::args;
#[allow(unused_imports)]
use std::{
    io::{self, Write},
    ops::Deref as _,
    sync::LazyLock,
};

#[allow(unused_imports)]
use crate::{
    direction::DirectionIteratorExt as _,
    layers::data_link::parse_eth,
    packets::get_packets,
    pcap::{add_packet_to_pcap_file, add_packet_to_pcap_file_owned, dump_to_pcap_file},
    runner::run_zephyr,
};

pub static NETWORK_SHMEM_SIZE: usize = 1 << 16;
pub static COV_SHMEM_SIZE: usize = 25632; // manually extracted
pub static PCAP_PATH: &str = "./pcap.pcap";

fn main() {
    env_logger::init();
    let zephyr_dir = args()
        .nth(1)
        .expect("Did not receive the path to the Zephyr executable as a command line argument");

    run_zephyr(&zephyr_dir);
    dump_to_pcap_file(PCAP_PATH).unwrap();
}

#[allow(unused)]
fn wait_for_newline() {
    let mut input = String::new();
    io::stdout().flush().unwrap();
    io::stdin().read_line(&mut input).unwrap();
}
