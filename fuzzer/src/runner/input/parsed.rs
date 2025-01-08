use crate::layers::{data_link::parse_eth, PacketParseError};
use libafl::{corpus::CorpusId, inputs::Input};

use pnet::packet::{
    ethernet::{Ethernet, MutableEthernetPacket},
    ipv4::{Ipv4, MutableIpv4Packet},
    tcp::{MutableTcpPacket, Tcp, TcpOptionNumbers},
};
use serde::{Deserialize, Serialize, Serializer};
use std::{hash::Hash, panic::catch_unwind};

#[derive(Clone, Debug)]
pub struct ParsedZephyrInput {
    eth: Ethernet,
    ip: Ipv4,
    tcp: Tcp,
}

impl ParsedZephyrInput {
    pub fn eth_mut(&mut self) -> &mut Ethernet {
        &mut self.eth
    }

    pub fn eth(&self) -> &Ethernet {
        &self.eth
    }

    pub fn ip_mut(&mut self) -> &mut Ipv4 {
        &mut self.ip
    }

    pub fn ip(&self) -> &Ipv4 {
        &self.ip
    }

    pub fn tcp_mut(&mut self) -> &mut Tcp {
        &mut self.tcp
    }

    pub fn tcp(&self) -> &Tcp {
        &self.tcp
    }
}

impl ParsedZephyrInput {
    pub fn ipv4_version(&self) -> &u8 {
        &self.ip.version
    }

    pub fn ipv4_version_mut(&mut self) -> &mut u8 {
        &mut self.ip.version
    }

    pub fn ipv4_header_length(&self) -> &u8 {
        &self.ip.header_length
    }

    pub fn ipv4_header_length_mut(&mut self) -> &mut u8 {
        &mut self.ip.header_length
    }

    pub fn ipv4_dscp(&self) -> &u8 {
        &self.ip.dscp
    }

    pub fn ipv4_dscp_mut(&mut self) -> &mut u8 {
        &mut self.ip.dscp
    }

    pub fn ipv4_ecn(&self) -> &u8 {
        &self.ip.ecn
    }

    pub fn ipv4_ecn_mut(&mut self) -> &mut u8 {
        &mut self.ip.ecn
    }

    pub fn ipv4_total_length(&self) -> &u16 {
        &self.ip.total_length
    }

    pub fn ipv4_total_length_mut(&mut self) -> &mut u16 {
        &mut self.ip.total_length
    }

    pub fn ipv4_identification(&self) -> &u16 {
        &self.ip.identification
    }

    pub fn ipv4_identification_mut(&mut self) -> &mut u16 {
        &mut self.ip.identification
    }

    pub fn ipv4_flags(&self) -> &u8 {
        &self.ip.flags
    }

    pub fn ipv4_flags_mut(&mut self) -> &mut u8 {
        &mut self.ip.flags
    }

    pub fn ipv4_fragment_offset(&self) -> &u16 {
        &self.ip.fragment_offset
    }

    pub fn ipv4_fragment_offset_mut(&mut self) -> &mut u16 {
        &mut self.ip.fragment_offset
    }

    pub fn ipv4_ttl(&self) -> &u8 {
        &self.ip.ttl
    }

    pub fn ipv4_ttl_mut(&mut self) -> &mut u8 {
        &mut self.ip.ttl
    }

    pub fn tcp_source(&self) -> &u16 {
        &self.tcp.source
    }

    pub fn tcp_source_mut(&mut self) -> &mut u16 {
        &mut self.tcp.source
    }

    pub fn tcp_destination(&self) -> &u16 {
        &self.tcp.destination
    }

    pub fn tcp_destination_mut(&mut self) -> &mut u16 {
        &mut self.tcp.destination
    }

    pub fn tcp_sequence(&self) -> &u32 {
        &self.tcp.sequence
    }

    pub fn tcp_sequence_mut(&mut self) -> &mut u32 {
        &mut self.tcp.sequence
    }

    pub fn tcp_acknowledgement(&self) -> &u32 {
        &self.tcp.acknowledgement
    }

    pub fn tcp_acknowledgement_mut(&mut self) -> &mut u32 {
        &mut self.tcp.acknowledgement
    }

    pub fn tcp_data_offset(&self) -> &u8 {
        &self.tcp.data_offset
    }

    pub fn tcp_data_offset_mut(&mut self) -> &mut u8 {
        &mut self.tcp.data_offset
    }

    pub fn tcp_reserved(&self) -> &u8 {
        &self.tcp.reserved
    }

    pub fn tcp_reserved_mut(&mut self) -> &mut u8 {
        &mut self.tcp.reserved
    }

    pub fn tcp_flags(&self) -> &u8 {
        &self.tcp.flags
    }

    pub fn tcp_flags_mut(&mut self) -> &mut u8 {
        &mut self.tcp.flags
    }

    pub fn tcp_window(&self) -> &u16 {
        &self.tcp.window
    }

    pub fn tcp_window_mut(&mut self) -> &mut u16 {
        &mut self.tcp.window
    }

    pub fn tcp_urgent_ptr(&self) -> &u16 {
        &self.tcp.urgent_ptr
    }

    pub fn tcp_urgent_ptr_mut(&mut self) -> &mut u16 {
        &mut self.tcp.urgent_ptr
    }

    pub fn tcp_payload(&self) -> &Vec<u8> {
        &self.tcp.payload
    }

    pub fn tcp_payload_mut(&mut self) -> &mut Vec<u8> {
        &mut self.tcp.payload
    }
}

impl Hash for ParsedZephyrInput {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        let vec: Vec<u8> = self.clone().into();
        vec.hash(state)
    }
}

impl Serialize for ParsedZephyrInput {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let vec: Vec<_> = self.clone().into();
        vec.serialize(serializer)
    }
}

impl<'a> Deserialize<'a> for ParsedZephyrInput {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'a>,
    {
        <Vec<u8>>::deserialize(deserializer)
            .map(|e| Self::try_from(&e as &[u8]))
            .map(Result::unwrap)
    }
}

impl Input for ParsedZephyrInput {
    fn generate_name(&self, id: Option<CorpusId>) -> String {
        format!("{:16x}", id.map(|e| e.0).unwrap_or(0))
    }
}

impl From<ParsedZephyrInput> for Vec<u8> {
    fn from(val: ParsedZephyrInput) -> Self {
        let mut tcp = val.tcp;
        // MutableTcpPacket::packet_size is broken, see https://github.com/libpnet/libpnet/issues/726
        tcp.options = tcp
            .options
            .iter()
            .filter(|e| e.number != TcpOptionNumbers::MSS)
            .cloned()
            .collect();
        let tcp_len = MutableTcpPacket::minimum_packet_size()
            + tcp.options.iter().map(|e| e.length[0]).sum::<u8>() as usize
            + tcp.payload.len();
        let tcp_buf = catch_unwind(|| {
            let mut tcp_buf = vec![0; tcp_len];
            MutableTcpPacket::new(&mut tcp_buf).unwrap().populate(&tcp);
            tcp_buf
        })
        .unwrap_or_else(|_e| panic!("tcp:\n{:?}\nbuffer len: {}", tcp, tcp_len));

        let mut ip = val.ip;
        ip.payload = tcp_buf;

        let ip_len = MutableIpv4Packet::packet_size(&ip);

        let mut ip_buf = vec![0; ip_len];
        MutableIpv4Packet::new(&mut ip_buf).unwrap().populate(&ip);

        let mut eth = val.eth;
        eth.payload = ip_buf;

        let eth_len = MutableEthernetPacket::packet_size(&eth);
        let mut eth_buf = vec![0; eth_len];
        MutableEthernetPacket::new(&mut eth_buf)
            .unwrap()
            .populate(&eth);

        eth_buf
    }
}

impl TryFrom<&[u8]> for ParsedZephyrInput {
    type Error = PacketParseError;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        let p = parse_eth(value)?;
        let (eth, net, upper) = p.contents_owned();

        let ip = net
            .get_ipv4_owned()
            .ok_or(PacketParseError::UnknownLayer3(value.to_vec()))?;

        let tcp = upper
            .ok_or(PacketParseError::UnknownLayer4(value.to_vec()))?
            .get_tcp_owned()
            .ok_or(PacketParseError::UnknownLayer4(value.to_vec()))?;
        Ok(Self { eth, ip, tcp })
    }
}

#[cfg(test)]
mod tests {
    use crate::{layers::data_link::parse_eth, packets::outgoing_tcp_packets};

    use super::ParsedZephyrInput;

    #[test]
    fn parse() {
        for (i, p) in outgoing_tcp_packets().iter().enumerate() {
            let parsed = ParsedZephyrInput::try_from(p as &[u8]).unwrap_or_else(|_| {
                panic!("Parsing packet {} with contents {:?}", i, parse_eth(p))
            });
            let vec: Vec<u8> = parsed.into();
            assert_eq!(*p, vec, "{}", i);
        }
    }

    #[test]
    fn serde() {
        for (i, p) in outgoing_tcp_packets().iter().enumerate() {
            let parsed = ParsedZephyrInput::try_from(p as &[u8]).unwrap();
            let serialized = serde_json::to_string(&parsed).unwrap();
            let deserialized: ParsedZephyrInput = serde_json::from_str(&serialized).unwrap();
            assert_eq!(
                <Vec<u8>>::from(parsed),
                <Vec<u8>>::from(deserialized),
                "{}",
                i
            );
        }
    }
}
