use pnet::packet::{
    arp::{Arp, ArpPacket},
    ip::IpNextHeaderProtocols,
    ipv4::{Ipv4, Ipv4Packet},
    ipv6::{Ipv6, Ipv6Packet},
    FromPacket,
};

use super::upper::{parse_hopopt, parse_icmpv6, parse_tcp, UpperLayerPacket};

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

pub fn parse_ipv6(packet: &[u8]) -> Result<NetworkLayerPacket, String> {
    let packet = Ipv6Packet::new(packet).ok_or("Could not parse Ipv6Packet".to_string())?;
    let packet = packet.from_packet();
    let upper = match packet.next_header {
        IpNextHeaderProtocols::Icmpv6 => parse_icmpv6(&packet.payload),
        IpNextHeaderProtocols::Hopopt => parse_hopopt(&packet.payload), // not sure if this is correct?
        _ => Err(format!("Not implemented: {:02x?}", packet)),
    }?;
    Ok(NetworkLayerPacket {
        net: NetworkLayerPacketType::Ipv6(packet),
        upper: Some(upper),
    })
}
pub fn parse_ipv4(packet: &[u8]) -> Result<NetworkLayerPacket, String> {
    let packet = Ipv4Packet::new(packet).ok_or("Could not parse Ipv4Packet".to_string())?;
    let packet = packet.from_packet();
    let upper = match packet.next_level_protocol {
        IpNextHeaderProtocols::Tcp => parse_tcp(&packet.payload),
        _ => Err(format!("Not implemented: {:02x?}", packet)),
    }?;
    Ok(NetworkLayerPacket {
        net: NetworkLayerPacketType::Ipv4(packet),
        upper: Some(upper),
    })
}

pub fn parse_arp(packet: &[u8]) -> Result<NetworkLayerPacket, String> {
    let packet = ArpPacket::new(packet).ok_or("Could not parse ArpPacket".to_string())?;
    let packet = packet.from_packet();
    Ok(NetworkLayerPacket {
        net: NetworkLayerPacketType::Arp(packet),
        upper: None,
    })
}
