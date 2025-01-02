use crate::layers::{data_link::parse_eth, PacketParseError};
use libafl::{corpus::CorpusId, inputs::Input};
use pnet::packet::{
    ethernet::{Ethernet, MutableEthernetPacket},
    ipv4::{Ipv4, MutableIpv4Packet},
    tcp::{MutableTcpPacket, Tcp, TcpOptionPacket},
};
use serde::{Deserialize, Serialize, Serializer};
use std::hash::Hash;

#[derive(Clone, Debug)]
pub struct ParsedZephyrInput {
    eth: Ethernet,
    ip: Ipv4,
    tcp: Tcp,
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
        let tcp = val.tcp;
        // MutableTcpPacket::packet_size is broken, see https://github.com/libpnet/libpnet/issues/726
        let tcp_len = MutableTcpPacket::minimum_packet_size()
            + tcp
                .options
                .iter()
                .map(TcpOptionPacket::packet_size)
                .sum::<usize>()
            + tcp.payload.len();

        let mut tcp_buf = vec![0; tcp_len];
        let mut packet = MutableTcpPacket::new(&mut tcp_buf).unwrap();
        packet.populate(&tcp);

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
