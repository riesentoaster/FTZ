use std::ops::Deref;

use crate::layers::{data_link::parse_eth, upper::UpperLayerPacket, PacketParseError};

pub enum PacketState {
    ParseError(PacketParseError),
    NoUpper,
    Icmpv6,
    Tcp(u8),
}

impl From<&UpperLayerPacket> for PacketState {
    fn from(p: &UpperLayerPacket) -> Self {
        match p {
            UpperLayerPacket::Tcp(tcp, _) => Self::Tcp(tcp.flags),
            UpperLayerPacket::Icmpv6(_) => Self::Icmpv6,
            UpperLayerPacket::Hopopt(_, u) => u.deref().into(),
        }
    }
}

impl From<&[u8]> for PacketState {
    fn from(value: &[u8]) -> Self {
        match parse_eth(value) {
            Ok(p) => match p.upper() {
                Some(u) => u.into(),
                None => Self::NoUpper,
            },
            Err(err) => Self::ParseError(err),
        }
    }
}

impl From<&PacketState> for u16 {
    fn from(val: &PacketState) -> Self {
        match val {
            PacketState::NoUpper => 0x1,
            PacketState::Icmpv6 => 0x2,
            PacketState::ParseError(packet_parse_error) => match packet_parse_error {
                PacketParseError::MalformedEthernet(_) => 0x3,
                PacketParseError::MalformedIpv4(_) => 0x4,
                PacketParseError::MalformedIpv6(_) => 0x5,
                PacketParseError::MalformedArp(_) => 0x6,
                PacketParseError::MalformedTcp(_) => 0x7,
                PacketParseError::MalformedIcmpv6(_) => 0x8,
                PacketParseError::MalformedHopopt(_) => 0x9,
                PacketParseError::UnknownLayer3(_) => 0xa,
                PacketParseError::UnknownLayer4(_) => 0xb,
            },
            PacketState::Tcp(flags) => (*flags as u16) << 8,
        }
    }
}
