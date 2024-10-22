use std::{net::Ipv6Addr, ops::Deref};

use pnet::{
    packet::{
        arp::{Arp, ArpOperations, MutableArpPacket},
        ethernet::{EtherTypes, Ethernet, MutableEthernetPacket},
        icmpv6::{
            self, echo_reply::Icmpv6Codes, Icmpv6, Icmpv6Type, Icmpv6Types, MutableIcmpv6Packet,
        },
        ip::IpNextHeaderProtocols,
        ipv6::{HopByHop, Ipv6, MutableHopByHopPacket, MutableIpv6Packet},
    },
    util::MacAddr,
};
use smoltcp::wire::Ipv6Address;

use crate::{
    layers::data_link::parse_eth, packets::get_packets, pcap::add_packet_to_pcap_file_owned,
    FUZZER_MAC_ADDR, IPV6_LINK_LOCAL_ADDR,
};

use super::{
    data_link::DataLinkLayerPacket, network::NetworkLayerPacketType, upper::UpperLayerPacket,
};

#[allow(unused)]
pub fn respond_to_arp(incoming: &DataLinkLayerPacket) -> Vec<u8> {
    let arp = match incoming.net() {
        NetworkLayerPacketType::Arp(p) => p,
        _ => panic!("Can not create an ARP response to {:?}", incoming),
    };

    let res_arp = Arp {
        operation: ArpOperations::Reply,
        sender_hw_addr: MacAddr::new(0x02, 0x00, 0x5e, 0x00, 0x53, 0x31),
        sender_proto_addr: arp.target_proto_addr,
        target_hw_addr: arp.sender_hw_addr,
        target_proto_addr: arp.sender_proto_addr,
        ..arp.clone()
    };

    let res_arp_len = MutableArpPacket::packet_size(&res_arp);
    let mut res_arp_buf = vec![0; res_arp_len];
    MutableArpPacket::new(&mut res_arp_buf)
        .unwrap()
        .populate(&res_arp);

    let eth = incoming.eth();
    let res_eth = Ethernet {
        destination: eth.source,
        source: FUZZER_MAC_ADDR.into(),
        ethertype: eth.ethertype,
        payload: res_arp_buf,
    };

    let res_eth_len = MutableEthernetPacket::packet_size(&res_eth);
    let mut res_eth_buf = vec![0; res_eth_len];
    MutableEthernetPacket::new(&mut res_eth_buf)
        .unwrap()
        .populate(&res_eth);
    res_eth_buf
}

pub fn respond_to_icmpv6_multicast_listener_report_message(
    incoming: &DataLinkLayerPacket,
) -> Option<Vec<u8>> {
    let icmp = match incoming.upper().unwrap() {
        UpperLayerPacket::Icmpv6(p) => p,
        UpperLayerPacket::Hopopt(_, p) => match p.as_ref() {
            UpperLayerPacket::Icmpv6(ref p2) => p2,
            _ => panic!(
                "Did not receive ICMPv6 within HopOpt packet: {:?}",
                incoming
            ),
        },
        _ => panic!("Did not receive straight ICMPv6 packet: {:?}", incoming),
    };

    if Icmpv6Types::NeighborSolicit != icmp.icmpv6_type {
        return None;
    }

    let res_icmpv6 = Icmpv6 {
        icmpv6_type: Icmpv6Type(143),
        icmpv6_code: Icmpv6Codes::NoCode,
        payload: vec![
            0, 0, 0, 2, 4, 0, 0, 0, 255, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 255, 0, 83, 255, 4, 0, 0,
            0, 255, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 255, 0, 0, 2,
        ],
        checksum: 0,
    };

    let res_icmpv6_len = MutableIcmpv6Packet::packet_size(&res_icmpv6);
    let mut res_icmpv6_buf = vec![0; res_icmpv6_len];
    let mut res_icmpv6_packet = MutableIcmpv6Packet::new(&mut res_icmpv6_buf).unwrap();
    res_icmpv6_packet.populate(&res_icmpv6);

    let target_ip = [0xff, 0x02, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x16];
    let source_ip = *IPV6_LINK_LOCAL_ADDR;

    let checksum = icmpv6::checksum(
        &res_icmpv6_packet.to_immutable(),
        // &Ipv6Address::from_bytes(source_ip.as_bytes()).into(),
        &Ipv6Address::from_bytes(source_ip.as_bytes()).into(),
        &Ipv6Address::from_bytes(&target_ip).into(),
    );

    res_icmpv6_packet.set_checksum(checksum);

    let res_hopopt = HopByHop {
        next_header: IpNextHeaderProtocols::Icmpv6,
        hdr_ext_len: 0,
        options: vec![0x05, 0x02, 0x00, 0x00, 0x01, 0x00], // from dump
    };

    let res_hopopt_len = MutableHopByHopPacket::packet_size(&res_hopopt);
    assert_eq!(8, res_hopopt_len, "Hopopt length is wrong");
    let mut res_hopopt_buf = vec![0; res_hopopt_len];
    let mut res_hopopt_packet = MutableHopByHopPacket::new(&mut res_hopopt_buf).unwrap();
    res_hopopt_packet.populate(&res_hopopt);

    res_hopopt_buf.append(&mut res_icmpv6_buf);

    assert!(
        incoming.net().is_ipv6(),
        "Did not expect non-ipv6 packets for icmpv6"
    );

    let res_net = Ipv6 {
        version: 6,
        traffic_class: 0x00,
        flow_label: 0x00,
        payload_length: res_hopopt_buf.len().try_into().unwrap(),
        next_header: IpNextHeaderProtocols::Hopopt,
        hop_limit: 0x1,
        source: Ipv6Addr::from_bits(
            source_ip
                .as_bytes()
                .iter()
                .rev()
                .enumerate()
                .map(|(i, &b)| (b as u128) << (i * 8))
                .sum(),
        ),
        destination: Ipv6Addr::from_bits(
            target_ip
                .into_iter()
                .rev()
                .enumerate()
                .map(|(i, b)| (b as u128) << (i * 8))
                .sum(),
        ),
        payload: res_hopopt_buf,
    };

    let res_net_len = MutableIpv6Packet::packet_size(&res_net);
    let mut res_net_buf = vec![0; res_net_len];
    let mut res_net_packet = MutableIpv6Packet::new(&mut res_net_buf).unwrap();
    res_net_packet.populate(&res_net);

    let eth_res = Ethernet {
        destination: MacAddr(0x33, 0x33, 0x00, 0x00, 0x00, 0x16), // IPv6mcast_16
        source: FUZZER_MAC_ADDR.into(),
        ethertype: EtherTypes::Ipv6,
        payload: res_net_buf,
    };

    let eth_res_len = MutableEthernetPacket::packet_size(&eth_res);
    let mut eth_res_buf = vec![0; eth_res_len];
    let mut eth_res_packet = MutableEthernetPacket::new(&mut eth_res_buf).unwrap();
    eth_res_packet.populate(&eth_res);

    let re_parsed = parse_eth(&eth_res_buf).unwrap(); // to check
    let should_raw = &get_packets()[6];
    let should = parse_eth(should_raw).unwrap();

    add_packet_to_pcap_file_owned(get_packets()[6].clone());
    log::debug!(
        "Comparing generated and captured packet:\ngen: {:02x?}\ncap: {:02x?}\ngen: {:02x?}\ncap: {:02x?}",
        re_parsed,
        should,
        eth_res_buf,
        get_packets()[6].deref()
    );

    Some(eth_res_buf)
}
