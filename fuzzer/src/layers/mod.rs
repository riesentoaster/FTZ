pub mod data_link;
pub mod interactive;
pub mod network;
pub mod upper;

#[repr(u8)]
#[derive(Debug, Clone)]
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
}
