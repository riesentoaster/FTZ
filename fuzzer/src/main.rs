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

pub static SHMEM_SIZE: usize = 1 << 16;
pub static PCAP_PATH: &str = "./pcap.pcap";

fn main() {
    env_logger::init();
    // get_packets()
    //     .into_iter()
    //     .map_content(|e| parse_eth(e).unwrap())
    //     .enumerate()
    //     // .filter_content(|e| e.upper().is_some_and(UpperLayerPacket::is_tcp))
    //     .for_each(|(i, e)| println!("{i}\n{:?}\n", e.deref()));
    // get_packets()
    //     .into_iter()
    //     .for_each(add_packet_to_pcap_file_owned);
    let zephyr_dir = args()
    .nth(1)
    .unwrap_or_else(|| {
        let res = format!("{}/zephyrproject/zephyr", env!("HOME"));
        println!("Did not receive zephyr's working directory as a command line argument, using '{}' instead", res);
        res
    });

    run_zephyr(&zephyr_dir, runner::RunType::Default);
    dump_to_pcap_file(PCAP_PATH).unwrap();
}

#[allow(unused)]
fn wait_for_newline() {
    let mut input = String::new();
    io::stdout().flush().unwrap();
    io::stdin().read_line(&mut input).unwrap();
}
