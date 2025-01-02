use std::ops::Deref;

use serde::{Deserialize, Serialize};

use crate::layers::{data_link::parse_eth, upper::UpperLayerPacket, PacketParseError};

#[derive(Debug, Serialize, Deserialize, Clone, Hash)]
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
            PacketState::Tcp(flags) => (*flags).into(),
            PacketState::NoUpper => 0x1 << 8,
            PacketState::Icmpv6 => 0x2 << 8,
            PacketState::ParseError(packet_parse_error) => match packet_parse_error {
                PacketParseError::MalformedEthernet(_) => 0x3 << 8,
                PacketParseError::MalformedIpv4(_) => 0x4 << 8,
                PacketParseError::MalformedIpv6(_) => 0x5 << 8,
                PacketParseError::MalformedArp(_) => 0x6 << 8,
                PacketParseError::MalformedTcp(_) => 0x7 << 8,
                PacketParseError::MalformedIcmpv6(_) => 0x8 << 8,
                PacketParseError::MalformedHopopt(_) => 0x9 << 8,
                PacketParseError::UnknownLayer3(_) => 0xa << 8,
                PacketParseError::UnknownLayer4(_) => 0xb << 8,
            },
        }
    }
}

impl PacketState {
    pub const fn max_numeric_value() -> u16 {
        0xb << 8
    }
}
