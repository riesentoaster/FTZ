use etherparse::err::packet::SliceError;
use serde::{Deserialize, Serialize};

pub mod data_link;
pub mod interactive;
pub mod network;
pub mod upper;

#[repr(u8)]
#[derive(Debug, Clone, Serialize, Deserialize, Hash)]
pub enum PacketParseError {
    MalformedEthernet(Vec<u8>),
    MalformedIpv4(Vec<u8>),
    MalformedIpv6(Vec<u8>),
    MalformedArp(Vec<u8>),
    MalformedTcp(Vec<u8>),
    MalformedIcmpv6(Vec<u8>),
    MalformedHopopt(Vec<u8>),
    UnknownLayer3(Vec<u8>),
    UnknownLayer4(Vec<u8>),
}

impl PacketParseError {
    #[allow(unused)]
    fn get_packet(&self) -> &[u8] {
        match self {
            PacketParseError::MalformedEthernet(vec) => vec,
            PacketParseError::MalformedIpv4(vec) => vec,
            PacketParseError::MalformedIpv6(vec) => vec,
            PacketParseError::MalformedArp(vec) => vec,
            PacketParseError::MalformedTcp(vec) => vec,
            PacketParseError::MalformedIcmpv6(vec) => vec,
            PacketParseError::MalformedHopopt(vec) => vec,
            PacketParseError::UnknownLayer3(vec) => vec,
            PacketParseError::UnknownLayer4(vec) => vec,
        }
    }

    pub fn from_slice_error(e: SliceError, value: &[u8]) -> Self {
        match e {
            SliceError::Len(_) => PacketParseError::MalformedEthernet(value.to_vec()),
            SliceError::Ipv4(_) => PacketParseError::MalformedIpv4(value.to_vec()),
            SliceError::LinuxSll(_) => PacketParseError::MalformedEthernet(value.to_vec()),
            SliceError::Ip(_) => PacketParseError::MalformedIpv4(value.to_vec()),
            SliceError::Ipv4Exts(_) => PacketParseError::MalformedIpv4(value.to_vec()),
            SliceError::Ipv6(_) => PacketParseError::MalformedIpv6(value.to_vec()),
            SliceError::Ipv6Exts(_) => PacketParseError::MalformedIpv6(value.to_vec()),
            SliceError::Tcp(_) => PacketParseError::MalformedTcp(value.to_vec()),
        }
    }
}
