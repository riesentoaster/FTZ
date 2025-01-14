use pnet::packet::{
    arp::{Arp, ArpPacket},
    ip::IpNextHeaderProtocols,
    ipv4::{Ipv4, Ipv4Packet},
    ipv6::{Ipv6, Ipv6Packet},
    FromPacket,
};

use super::{
    upper::{parse_hopopt, parse_icmpv6, parse_tcp, UpperLayerPacket},
    PacketParseError,
};

#[derive(Debug)]
pub enum NetworkLayerPacketType {
    Ipv4(Ipv4),
    Ipv6(Ipv6),
    Arp(Arp),
}

#[allow(unused)]
impl NetworkLayerPacketType {
    #[must_use]
    pub fn is_ipv4(&self) -> bool {
        matches!(self, Self::Ipv4(..))
    }
    #[must_use]
    pub fn is_ipv6(&self) -> bool {
        matches!(self, Self::Ipv6(..))
    }
    #[must_use]
    pub fn is_arp(&self) -> bool {
        matches!(self, Self::Arp(..))
    }

    pub fn get_ipv4(&self) -> Option<&Ipv4> {
        match self {
            NetworkLayerPacketType::Ipv4(ipv4) => Some(ipv4),
            _ => None,
        }
    }
    pub fn get_ipv4_owned(self) -> Option<Ipv4> {
        match self {
            NetworkLayerPacketType::Ipv4(ipv4) => Some(ipv4),
            _ => None,
        }
    }

    pub fn get_arp(&self) -> Option<&Arp> {
        match self {
            NetworkLayerPacketType::Arp(arp) => Some(arp),
            _ => None,
        }
    }

    pub fn types_to_string(&self) -> &str {
        match self {
            NetworkLayerPacketType::Ipv4(ipv4) => "ipv4",
            NetworkLayerPacketType::Ipv6(ipv6) => "ipv6",
            NetworkLayerPacketType::Arp(arp) => "arp",
        }
    }
}

#[derive(Debug)]
pub struct NetworkLayerPacket {
    upper: Option<UpperLayerPacket>,
    net: NetworkLayerPacketType,
}

#[allow(unused)]
impl NetworkLayerPacket {
    pub fn upper(&self) -> Option<&UpperLayerPacket> {
        self.upper.as_ref()
    }

    pub fn net(&self) -> &NetworkLayerPacketType {
        &self.net
    }

    pub fn contents(self) -> (NetworkLayerPacketType, Option<UpperLayerPacket>) {
        (self.net, self.upper)
    }
}

pub fn parse_ipv6(packet: &[u8]) -> Result<NetworkLayerPacket, PacketParseError> {
    let ipv6 = Ipv6Packet::new(packet)
        .ok_or(PacketParseError::MalformedIpv6(packet.to_vec()))?
        .from_packet();
    let upper = match ipv6.next_header {
        IpNextHeaderProtocols::Icmpv6 => parse_icmpv6(&ipv6.payload),
        IpNextHeaderProtocols::Hopopt => parse_hopopt(&ipv6.payload), // not sure if this is correct?
        _ => Err(PacketParseError::UnknownLayer4(packet.to_vec())),
    }?;
    Ok(NetworkLayerPacket {
        net: NetworkLayerPacketType::Ipv6(ipv6),
        upper: Some(upper),
    })
}
pub fn parse_ipv4(packet: &[u8]) -> Result<NetworkLayerPacket, PacketParseError> {
    let ipv4 = Ipv4Packet::new(packet)
        .ok_or(PacketParseError::MalformedIpv4(packet.to_vec()))?
        .from_packet();
    let upper = match ipv4.next_level_protocol {
        IpNextHeaderProtocols::Tcp => parse_tcp(&ipv4.payload),
        _ => Err(PacketParseError::UnknownLayer4(packet.to_vec())),
    }?;
    Ok(NetworkLayerPacket {
        net: NetworkLayerPacketType::Ipv4(ipv4),
        upper: Some(upper),
    })
}

pub fn parse_arp(packet: &[u8]) -> Result<NetworkLayerPacket, PacketParseError> {
    let packet = ArpPacket::new(packet).ok_or(PacketParseError::MalformedArp(packet.to_vec()))?;
    let packet = packet.from_packet();
    Ok(NetworkLayerPacket {
        net: NetworkLayerPacketType::Arp(packet),
        upper: None,
    })
}
