use etherparse::err::packet::SliceError;
use serde::{Deserialize, Serialize};

pub mod data_link;
pub mod interactive;
pub mod network;
pub mod upper;

#[repr(u8)]
#[derive(Debug, Clone, Serialize, Deserialize, Hash)]
pub enum PacketParseError {
    MalformedEthernet,
    MalformedIpv4,
    MalformedIpv6,
    MalformedArp,
    MalformedTcp,
    MalformedIcmpv6,
    MalformedHopopt,
    UnknownLayer3,
    UnknownLayer4,
}

impl PacketParseError {
    pub fn from_slice_error(e: SliceError) -> Self {
        match e {
            SliceError::Len(_) => PacketParseError::MalformedEthernet,
            SliceError::Ipv4(_) => PacketParseError::MalformedIpv4,
            SliceError::LinuxSll(_) => PacketParseError::MalformedEthernet,
            SliceError::Ip(_) => PacketParseError::MalformedIpv4,
            SliceError::Ipv4Exts(_) => PacketParseError::MalformedIpv4,
            SliceError::Ipv6(_) => PacketParseError::MalformedIpv6,
            SliceError::Ipv6Exts(_) => PacketParseError::MalformedIpv6,
            SliceError::Tcp(_) => PacketParseError::MalformedTcp,
        }
    }
}
