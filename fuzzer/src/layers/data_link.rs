use std::fmt::{Debug, Error, Formatter};

use pnet::packet::{
    ethernet::{EtherTypes, Ethernet, EthernetPacket},
    FromPacket,
};

use super::{
    network::{parse_arp, parse_ipv4, parse_ipv6, NetworkLayerPacketType},
    upper::UpperLayerPacket,
};

#[allow(unused)]
pub struct DataLinkLayerPacket {
    upper: Option<UpperLayerPacket>,
    net: NetworkLayerPacketType,
    eth: Ethernet,
}

impl Debug for DataLinkLayerPacket {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        f.write_str("--------------------------------\nDataLinkLayerPacket {\n\tupper: ")?;
        self.upper.fmt(f)?;
        f.write_str("\n\tnet:   ")?;
        self.net().fmt(f)?;
        f.write_str("\n\teth:   ")?;
        self.eth().fmt(f)?;
        f.write_str("\n}\n--------------------------------")
    }
}

#[allow(unused)]
impl DataLinkLayerPacket {
    pub fn upper(&self) -> Option<&UpperLayerPacket> {
        self.upper.as_ref()
    }

    pub fn net(&self) -> &NetworkLayerPacketType {
        &self.net
    }

    pub fn eth(&self) -> &Ethernet {
        &self.eth
    }

    pub fn types_to_string(&self) -> String {
        if self.upper().is_some() {
            format!(
                "Packet: {} {:?}",
                self.net().types_to_string(),
                self.upper().unwrap().types_to_string()
            )
        } else {
            format!("Packet: {} [no upper]", self.net().types_to_string(),)
        }
    }
}

#[allow(unused)]
pub fn parse_eth(input: &[u8]) -> Result<DataLinkLayerPacket, String> {
    let eth = EthernetPacket::new(input)
        .ok_or(format!(
            "Could not create a Ethernet packet: {:02x?}",
            input
        ))?
        .from_packet();

    let net = match eth.ethertype {
        EtherTypes::Ipv4 => parse_ipv4(&eth.payload),
        EtherTypes::Ipv6 => parse_ipv6(&eth.payload),
        EtherTypes::Arp => parse_arp(&eth.payload),
        _ => Err(format!("Not implemented: {:02x?}", eth)),
    }?;

    let (net, upper) = net.contents();

    Ok(DataLinkLayerPacket { eth, net, upper })
}
