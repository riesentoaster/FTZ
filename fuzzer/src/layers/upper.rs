use pnet::packet::{
    icmpv6::{Icmpv6, Icmpv6Packet},
    ip::IpNextHeaderProtocols,
    ipv6::{HopByHop, HopByHopPacket},
    tcp::{Tcp, TcpPacket},
    FromPacket,
};

#[allow(unused)]
#[derive(Debug)]
pub enum UpperLayerPacket {
    Icmpv6(Icmpv6),
    Hopopt(HopByHop, Box<UpperLayerPacket>),
    Tcp(Tcp, String),
}

#[allow(unused)]
impl UpperLayerPacket {
    #[must_use]
    pub fn is_icmpv6(&self) -> bool {
        matches!(self, Self::Icmpv6(..))
    }

    #[must_use]
    pub fn get_icmpv6(&self) -> Option<&Icmpv6> {
        match self {
            UpperLayerPacket::Icmpv6(icmpv6) => Some(icmpv6),
            UpperLayerPacket::Hopopt(extension, upper) => upper.get_icmpv6(),
            _ => None,
        }
    }

    #[must_use]
    pub fn is_hopopt(&self) -> bool {
        matches!(self, Self::Hopopt(..))
    }

    #[must_use]
    pub fn is_tcp(&self) -> bool {
        matches!(self, Self::Tcp(..))
    }

    pub fn types_to_string(&self) -> String {
        match self {
            UpperLayerPacket::Icmpv6(icmpv6) => "icmpv6".to_string(),
            UpperLayerPacket::Hopopt(extension, upper_layer_packet) => {
                format!("hopopt {{{}}}", upper_layer_packet.types_to_string())
            }
            UpperLayerPacket::Tcp(tcp, s) => format!("tcp [{}]", s),
        }
    }
}

#[allow(unused)]
pub fn parse_hopopt(packet: &[u8]) -> Result<UpperLayerPacket, String> {
    let ext_packet =
        HopByHopPacket::new(packet).ok_or("Could not parse HopByHopPacket".to_string())?;
    let ext_packet = ext_packet.from_packet();
    match ext_packet.next_header {
        IpNextHeaderProtocols::Icmpv6 => {
            let mut hopopt_header_len: usize = ext_packet.hdr_ext_len.into();
            hopopt_header_len += 8; // skip its own header
            let next_packet_buffer = &packet[hopopt_header_len..];
            let next_packet = parse_icmpv6(next_packet_buffer)?;
            Ok(UpperLayerPacket::Hopopt(ext_packet, Box::new(next_packet)))
        }
        e => Err(format!(
            "Weird HopByHopPacket of type {}: {:02x?}",
            e, ext_packet
        )),
    }
}
pub fn parse_icmpv6(packet: &[u8]) -> Result<UpperLayerPacket, String> {
    let packet = Icmpv6Packet::new(packet).ok_or("Could not parse Icmpv6Packet".to_string())?;
    let packet = packet.from_packet();
    Ok(UpperLayerPacket::Icmpv6(packet))
}

pub fn parse_tcp(packet: &[u8]) -> Result<UpperLayerPacket, String> {
    let packet = TcpPacket::new(packet).ok_or("Could not parse TcpPacket".to_string())?;
    let packet = packet.from_packet();
    let s = match String::from_utf8(packet.payload.clone()) {
        Ok(s) => format!("[{: >5x}]: '{}'", s.len(), s.escape_debug()),
        Err(e) => format!("{e} — {:02x?}", packet.payload),
    };

    Ok(UpperLayerPacket::Tcp(packet, s))
}
