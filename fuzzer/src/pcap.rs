use std::{
    fs::{File, OpenOptions},
    ops::Deref,
    path::Path,
    sync::{LazyLock, Mutex},
    time::{Duration, Instant},
};

use pcap_file::{
    pcap::{PcapPacket, PcapWriter},
    PcapError,
};

use crate::direction::Direction;

#[allow(clippy::type_complexity)]
static PACKETS: LazyLock<Mutex<Vec<Direction<(Duration, Vec<u8>)>>>> =
    LazyLock::new(|| Mutex::new(vec![]));
static START_TIME: LazyLock<Instant> = LazyLock::new(Instant::now);

pub fn add_packet_to_pcap_file(raw: Direction<&[u8]>) {
    PACKETS
        .lock()
        .unwrap()
        .push(raw.map(|e| (Instant::now().duration_since(*START_TIME), e.to_vec())));
}
pub fn add_packet_to_pcap_file_owned(raw: Direction<Vec<u8>>) {
    PACKETS
        .lock()
        .unwrap()
        .push(raw.map(|e| (Instant::now().duration_since(*START_TIME), e)));
}

#[allow(unused)]
pub fn dump_to_pcap_file<P: AsRef<Path>>(path: P) -> Result<usize, PcapError> {
    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(PcapError::IoError)?;
    write_to_file(file)
}

fn write_to_file(file: File) -> Result<usize, PcapError> {
    let mut pcap_writer = PcapWriter::new(file)?;
    let lens = PACKETS
        .lock()
        .unwrap()
        .iter()
        .enumerate()
        .map(|(i, e)| {
            let (duration, packet) = e.deref();
            let len = packet.len();

            log::info!(
                "#{} Attempting to write pcap entry dir: {:?}, time: {:?}, len: {}",
                i + 1,
                e.outer_to_string(),
                duration,
                len
            );
            PcapPacket::new(
                *duration,
                len.try_into().expect("Could not parse usize to u64"),
                &packet[..len],
            )
        })
        .map(|p| pcap_writer.write_packet(&p))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(lens.iter().sum())
}
