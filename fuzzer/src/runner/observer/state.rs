use std::ops::Deref;

use serde::{Deserialize, Serialize};

use crate::layers::{data_link::parse_eth, upper::UpperLayerPacket, PacketParseError};

#[derive(Debug, Serialize, Deserialize, Clone, Hash)]
pub enum PacketState {
    ParseError(PacketParseError),
    NoUpper,
    Icmpv6,
    Tcp(u8),
    // No previous state
    Nothing,
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
            PacketState::NoUpper => 0x100,
            PacketState::Icmpv6 => 0x101,
            PacketState::ParseError(packet_parse_error) => match packet_parse_error {
                PacketParseError::MalformedEthernet => 0x102,
                PacketParseError::MalformedIpv4 => 0x103,
                PacketParseError::MalformedIpv6 => 0x104,
                PacketParseError::MalformedArp => 0x105,
                PacketParseError::MalformedTcp => 0x106,
                PacketParseError::MalformedIcmpv6 => 0x107,
                PacketParseError::MalformedHopopt => 0x108,
                PacketParseError::UnknownLayer3 => 0x109,
                PacketParseError::UnknownLayer4 => 0x10a,
            },
            PacketState::Nothing => 0x10b,
        }
    }
}

impl PacketState {
    pub const fn array_size() -> usize {
        // max value + 1
        0x10c
    }
}
