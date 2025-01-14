use criterion::{black_box, criterion_group, criterion_main, Criterion};
use etherparse::{Packet, PacketHeaders, SlicedPacket};
use fuzzer::{layers::data_link::parse_eth, packets::get_packets};

use std::ops::Deref;

fn criterion_benchmark(c: &mut Criterion) {
    let packets: Vec<Vec<u8>> = get_packets().iter().map(|p| p.deref().to_vec()).collect();

    let mut group = c.benchmark_group("ethernet_parsing");

    group.bench_function("parse_eth", |b| {
        b.iter(|| {
            for packet in packets.iter() {
                let _ = parse_eth(black_box(packet)).unwrap();
            }
        })
    });

    group.bench_function("etherparse::SlicedPacket", |b| {
        b.iter(|| {
            for packet in packets.iter() {
                let _ = SlicedPacket::from_ethernet(black_box(packet)).unwrap();
            }
        })
    });

    group.bench_function("etherparse::Packet", |b| {
        b.iter(|| {
            for packet in packets.iter() {
                let _: Packet = PacketHeaders::from_ethernet_slice(black_box(packet))
                    .unwrap()
                    .into();
            }
        })
    });

    group.bench_function("etherparse::PacketHeaders", |b| {
        b.iter(|| {
            for packet in packets.iter() {
                let _ = PacketHeaders::from_ethernet_slice(black_box(packet)).unwrap();
            }
        })
    });

    group.finish();
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
