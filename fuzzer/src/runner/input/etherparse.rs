use etherparse::{
    ip_number::AUTH, Ethernet2Header, Ipv4Extensions, Ipv4Header, Packet, PacketHeaders, Payload,
    TcpHeader,
};

use libafl::{
    corpus::CorpusId,
    inputs::Input,
    mutators::{
        numeric::{int_mutators_no_crossover, IntMutatorsNoCrossoverType},
        ToMappingMutator,
    },
};
use libafl_bolts::{
    generic_hash_std, map_tuple_list_type, merge_tuple_list_type,
    tuples::{tuple_list, tuple_list_type, Map as _, Merge as _},
};
use serde::{Deserialize, Serialize};
use std::{
    hash::{Hash, Hasher},
    io::Write as _,
    vec::Vec,
};

use crate::layers::PacketParseError;

use super::bool::BoolMutator;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EtherparseInput {
    tcp: TcpHeader,
    ip: Ipv4Header,
    ipv4_extensions: Ipv4Extensions,
    eth: Ethernet2Header,
    payload: Payload,
}

impl Input for EtherparseInput {
    fn generate_name(&self, _id: Option<CorpusId>) -> String {
        let buf: Vec<u8> = self.into();
        format!("{:16x}", generic_hash_std(&buf))
    }
}

impl Hash for EtherparseInput {
    fn hash<H: Hasher>(&self, state: &mut H) {
        serde_json::to_string(self).unwrap().hash(state)
    }
}

impl From<&EtherparseInput> for Vec<u8> {
    fn from(value: &EtherparseInput) -> Self {
        let mut buf = Vec::<u8>::with_capacity(
            //lets reserve enough memory to avoid unnecessary allocations
            Ethernet2Header::LEN + Ipv4Header::MAX_LEN + TcpHeader::MAX_LEN + 8, //payload
        );

        value.eth.write(&mut buf).unwrap();
        // checksum calculated automatically
        value.ip.write(&mut buf).unwrap();
        if value.ipv4_extensions.auth.is_some() {
            value.ipv4_extensions.write(&mut buf, AUTH).unwrap()
        }
        let tcp_checksum = value
            .tcp
            .calc_checksum_ipv4(&value.ip, value.payload.slice())
            .unwrap();
        let mut tcp = value.tcp.clone();
        tcp.checksum = tcp_checksum;
        tcp.write(&mut buf).unwrap();
        buf.write_all(value.payload.slice()).unwrap();
        buf
    }
}

impl From<EtherparseInput> for Vec<u8> {
    fn from(value: EtherparseInput) -> Self {
        (&value).into()
    }
}

impl TryFrom<&[u8]> for EtherparseInput {
    type Error = PacketParseError;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        let res: Packet = match PacketHeaders::from_ethernet_slice(value) {
            Ok(e) => Ok(e.into()),
            Err(e) => Err(PacketParseError::from_slice_error(e)),
        }?;
        let tcp = res.transport.unwrap().tcp().unwrap();
        let (ip, ipv4_extensions) = match res.net.unwrap() {
            etherparse::NetHeaders::Ipv4(ipv4_header, ipv4_extensions) => {
                (ipv4_header, ipv4_extensions)
            }
            etherparse::NetHeaders::Ipv6(_ipv6_header, _ipv6_extensions) => {
                panic!("ipv6 not supported")
            }
        };

        let eth = match res.link.unwrap() {
            etherparse::LinkHeader::Ethernet2(eth) => eth,
            etherparse::LinkHeader::LinuxSll(_sll) => panic!("sll not supported"),
        };

        let payload = res.payload;
        Ok(EtherparseInput {
            tcp,
            ip,
            ipv4_extensions,
            eth,
            payload,
        })
    }
}

impl From<Vec<u8>> for EtherparseInput {
    fn from(value: Vec<u8>) -> Self {
        EtherparseInput::try_from(&value as &[u8]).unwrap()
    }
}

impl EtherparseInput {
    pub fn tcp_source_port(&mut self) -> &mut u16 {
        &mut self.tcp.source_port
    }
    pub fn tcp_destination_port(&mut self) -> &mut u16 {
        &mut self.tcp.destination_port
    }
    pub fn tcp_sequence_number(&mut self) -> &mut u32 {
        &mut self.tcp.sequence_number
    }
    pub fn tcp_acknowledgment_number(&mut self) -> &mut u32 {
        &mut self.tcp.acknowledgment_number
    }
    pub fn tcp_ns(&mut self) -> &mut bool {
        &mut self.tcp.ns
    }
    pub fn tcp_fin(&mut self) -> &mut bool {
        &mut self.tcp.fin
    }
    pub fn tcp_syn(&mut self) -> &mut bool {
        &mut self.tcp.syn
    }
    pub fn tcp_rst(&mut self) -> &mut bool {
        &mut self.tcp.rst
    }
    pub fn tcp_psh(&mut self) -> &mut bool {
        &mut self.tcp.psh
    }
    pub fn tcp_ack(&mut self) -> &mut bool {
        &mut self.tcp.ack
    }
    pub fn tcp_urg(&mut self) -> &mut bool {
        &mut self.tcp.urg
    }
    pub fn tcp_ece(&mut self) -> &mut bool {
        &mut self.tcp.ece
    }
    pub fn tcp_cwr(&mut self) -> &mut bool {
        &mut self.tcp.cwr
    }
    pub fn tcp_window_size(&mut self) -> &mut u16 {
        &mut self.tcp.window_size
    }
    pub fn tcp_urgent_pointer(&mut self) -> &mut u16 {
        &mut self.tcp.urgent_pointer
    }

    pub fn mutators() -> TcpMutators {
        int_mutators_no_crossover()
            .map(ToMappingMutator::new(
                Self::tcp_destination_port as fn(&mut EtherparseInput) -> &mut u16,
            ))
            .merge(int_mutators_no_crossover().map(ToMappingMutator::new(
                Self::tcp_source_port as fn(&mut EtherparseInput) -> &mut u16,
            )))
            .merge(int_mutators_no_crossover().map(ToMappingMutator::new(
                Self::tcp_sequence_number as fn(&mut EtherparseInput) -> &mut u32,
            )))
            .merge(int_mutators_no_crossover().map(ToMappingMutator::new(
                Self::tcp_acknowledgment_number as fn(&mut EtherparseInput) -> &mut u32,
            )))
            .merge(int_mutators_no_crossover().map(ToMappingMutator::new(
                Self::tcp_urgent_pointer as fn(&mut EtherparseInput) -> &mut u16,
            )))
            .merge(int_mutators_no_crossover().map(ToMappingMutator::new(
                Self::tcp_window_size as fn(&mut EtherparseInput) -> &mut u16,
            )))
            .merge(tuple_list!(BoolMutator).map(ToMappingMutator::new(
                Self::tcp_ns as fn(&mut EtherparseInput) -> &mut bool,
            )))
            .merge(tuple_list!(BoolMutator).map(ToMappingMutator::new(
                Self::tcp_fin as fn(&mut EtherparseInput) -> &mut bool,
            )))
            .merge(tuple_list!(BoolMutator).map(ToMappingMutator::new(
                Self::tcp_syn as fn(&mut EtherparseInput) -> &mut bool,
            )))
            .merge(tuple_list!(BoolMutator).map(ToMappingMutator::new(
                Self::tcp_rst as fn(&mut EtherparseInput) -> &mut bool,
            )))
            .merge(tuple_list!(BoolMutator).map(ToMappingMutator::new(
                Self::tcp_psh as fn(&mut EtherparseInput) -> &mut bool,
            )))
            .merge(tuple_list!(BoolMutator).map(ToMappingMutator::new(
                Self::tcp_ack as fn(&mut EtherparseInput) -> &mut bool,
            )))
            .merge(tuple_list!(BoolMutator).map(ToMappingMutator::new(
                Self::tcp_urg as fn(&mut EtherparseInput) -> &mut bool,
            )))
            .merge(tuple_list!(BoolMutator).map(ToMappingMutator::new(
                Self::tcp_ece as fn(&mut EtherparseInput) -> &mut bool,
            )))
            .merge(tuple_list!(BoolMutator).map(ToMappingMutator::new(
                Self::tcp_cwr as fn(&mut EtherparseInput) -> &mut bool,
            )))
    }
}

pub type TcpMutators = merge_tuple_list_type!(
    map_tuple_list_type!(
        IntMutatorsNoCrossoverType,
        ToMappingMutator<fn(&mut EtherparseInput) -> &mut u16>
    ),
    map_tuple_list_type!(
        IntMutatorsNoCrossoverType,
        ToMappingMutator<fn(&mut EtherparseInput) -> &mut u16>
    ),
    map_tuple_list_type!(
        IntMutatorsNoCrossoverType,
        ToMappingMutator<fn(&mut EtherparseInput) -> &mut u32>
    ),
    map_tuple_list_type!(
        IntMutatorsNoCrossoverType,
        ToMappingMutator<fn(&mut EtherparseInput) -> &mut u32>
    ),
    map_tuple_list_type!(
        IntMutatorsNoCrossoverType,
        ToMappingMutator<fn(&mut EtherparseInput) -> &mut u16>
    ),
    map_tuple_list_type!(
        IntMutatorsNoCrossoverType,
        ToMappingMutator<fn(&mut EtherparseInput) -> &mut u16>
    ),
    map_tuple_list_type!(
        tuple_list_type!(BoolMutator),
        ToMappingMutator<fn(&mut EtherparseInput) -> &mut bool>
    ),
    map_tuple_list_type!(
        tuple_list_type!(BoolMutator),
        ToMappingMutator<fn(&mut EtherparseInput) -> &mut bool>
    ),
    map_tuple_list_type!(
        tuple_list_type!(BoolMutator),
        ToMappingMutator<fn(&mut EtherparseInput) -> &mut bool>
    ),
    map_tuple_list_type!(
        tuple_list_type!(BoolMutator),
        ToMappingMutator<fn(&mut EtherparseInput) -> &mut bool>
    ),
    map_tuple_list_type!(
        tuple_list_type!(BoolMutator),
        ToMappingMutator<fn(&mut EtherparseInput) -> &mut bool>
    ),
    map_tuple_list_type!(
        tuple_list_type!(BoolMutator),
        ToMappingMutator<fn(&mut EtherparseInput) -> &mut bool>
    ),
    map_tuple_list_type!(
        tuple_list_type!(BoolMutator),
        ToMappingMutator<fn(&mut EtherparseInput) -> &mut bool>
    ),
    map_tuple_list_type!(
        tuple_list_type!(BoolMutator),
        ToMappingMutator<fn(&mut EtherparseInput) -> &mut bool>
    ),
    map_tuple_list_type!(
        tuple_list_type!(BoolMutator),
        ToMappingMutator<fn(&mut EtherparseInput) -> &mut bool>
    )
);
