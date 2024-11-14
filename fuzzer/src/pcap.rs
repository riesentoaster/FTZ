use std::{
    fs::OpenOptions,
    io::Write,
    ops::Deref,
    path::Path,
    sync::{LazyLock, Mutex},
    time::{Duration, Instant},
};

use pcap_file::{
    pcap::{PcapPacket, PcapWriter},
    PcapError,
};

use libafl::Error;

use crate::direction::Direction;

#[allow(clippy::type_complexity)]
static PACKETS: LazyLock<Mutex<Vec<Direction<(Duration, Vec<u8>)>>>> =
    LazyLock::new(|| Mutex::new(vec![]));
static START_TIME: LazyLock<Instant> = LazyLock::new(Instant::now);

#[allow(unused)]
fn add_packet_to_global_pcap_file(raw: Direction<&[u8]>) {
    add_packet_to_global_pcap_file_owned(raw.map(|e| e.to_vec()));
}
#[allow(unused)]
fn add_packet_to_global_pcap_file_owned(raw: Direction<Vec<u8>>) {
    PACKETS
        .lock()
        .unwrap()
        .push(raw.map(|e| (Instant::now().duration_since(*START_TIME), e)));
}

pub fn dump_global_packets_to_pcap_file<P: AsRef<Path>>(
    path: P,
    append: bool,
) -> Result<usize, Error> {
    let packets = PACKETS.lock().unwrap();
    let packets = packets
        .iter()
        .map(Deref::deref)
        .map(|(d, p)| (d, p))
        .collect::<Vec<_>>();
    dump_packets_to_pcap_file(&packets, path, append)
}

#[allow(unused)]
pub fn dump_packets_to_pcap_file<P: AsRef<Path>>(
    packets: &[(&Duration, &Vec<u8>)],
    path: P,
    append: bool,
) -> Result<usize, Error> {
    let mut file = OpenOptions::new()
        .create(true)
        .append(append)
        .open(path)
        .map_err(|e| Error::os_error(e, "Could not create .pcap file"))?;

    write_pcap(packets, &mut file)
}

pub fn write_pcap<W: Write>(
    packets: &[(&Duration, &Vec<u8>)],
    file: &mut W,
) -> Result<usize, Error> {
    let mut pcap_writer =
        PcapWriter::new(file).map_err(map_pcap_err("Could not create pcap writer"))?;
    let lens = packets
        .iter()
        .enumerate()
        .map(|(i, (duration, packet))| {
            let len = packet.len();

            log::debug!(
                "#{} Attempting to write pcap entry at time: {:?}, len: {}",
                i + 1,
                duration,
                len
            );

            PcapPacket::new(
                **duration,
                len.try_into().expect("Could not parse usize to u64"),
                &packet[..len],
            )
        })
        .map(|p| {
            pcap_writer
                .write_packet(&p)
                .map_err(map_pcap_err("Could not write pcap entry"))
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(lens.iter().sum())
}

fn map_pcap_err(message: &str) -> impl Fn(PcapError) -> Error + use<'_> {
    move |e| match e {
        PcapError::IoError(io_error) => Error::os_error(io_error, message),
        e => Error::unknown(format!("{}: {:?}", message, e)),
    }
}
