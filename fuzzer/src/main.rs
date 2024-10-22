mod direction;
mod executor;
mod layers;
mod packets;
mod pcap;
mod smoltcp;

#[allow(unused_imports)]
use std::{
    io::{self, Write},
    ops::Deref as _,
    sync::LazyLock,
};

use ::smoltcp::wire::IpAddress;

#[allow(unused_imports)]
use crate::{
    direction::DirectionIteratorExt as _,
    executor::run_zephyr_manual,
    layers::data_link::parse_eth,
    packets::get_packets,
    pcap::{add_packet_to_pcap_file, add_packet_to_pcap_file_owned, dump_to_pcap_file},
};

pub static SHMEM_SIZE: usize = 1 << 16;
pub static FUZZER_MAC_ADDR: [u8; 6] = [0x00, 0x00, 0x5e, 0x00, 0x53, 0xff];
pub static PCAP_PATH: &str = "./pcap.pcap";
pub static IPV6_LINK_LOCAL_ADDR: LazyLock<IpAddress> = LazyLock::new(|| {
    IpAddress::v6(
        0xfe80, 0x0000, 0x0000, 0x0000, 0x0200, 0x5eff, 0xfe00, 0x53ff,
    )
});

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
    run_zephyr_manual(executor::RunType::Default);
    dump_to_pcap_file(PCAP_PATH).unwrap();
}

#[allow(unused)]
fn wait_for_newline() {
    let mut input = String::new();
    io::stdout().flush().unwrap();
    io::stdin().read_line(&mut input).unwrap();
}
