use std::{
    cell::RefCell,
    rc::Rc,
    thread::sleep,
    time::{Duration, Instant},
};

use libafl::Error;
use libafl_bolts::shmem::{MmapShMem, ShMemDescription};

use pnet::packet::icmpv6::Icmpv6Types;

use crate::{
    layers::{
        data_link::{parse_eth, DataLinkLayerPacket},
        interactive::{
            create_response_to_icmpv6_neighbor_solicitation,
            create_response_to_icmpv6_router_solicitation, respond_to_arp,
        },
        upper::UpperLayerPacket,
    },
    runner::{CLIENT_MAC_ADDR, IPV6_LINK_LOCAL_ADDR, SETUP_TIMEOUT},
    shmem::get_shmem,
};

use super::shmem_net_device_buffers::ShmemNetDeviceBuffer;

pub struct ShmemNetworkDevice {
    tx_shmem: ShmemNetDeviceBuffer<MmapShMem>,
    rx_shmem: ShmemNetDeviceBuffer<MmapShMem>,
}

impl ShmemNetworkDevice {
    pub fn new(buf_size: usize, id: usize) -> Result<Self, Error> {
        let shmem = get_shmem(buf_size * 2 + 8, id, "net")?;

        log::debug!("Created ShmemNetworkDevice");
        let (tx_shmem, rx_shmem) = ShmemNetDeviceBuffer::new(Rc::new(RefCell::new(shmem)));
        let mut res = Self { tx_shmem, rx_shmem };
        res.reset();
        Ok(res)
    }

    pub fn try_recv(&mut self) -> Option<Vec<u8>> {
        let res = self.rx_shmem.get_data_and_set_empty();
        if let Some(p) = res.as_ref() {
            log::debug!("Received packet of len: {}", p.len());
        }
        res
    }

    pub fn send(&mut self, data: &[u8]) {
        self.tx_shmem.prep_data(data.len()).copy_from_slice(data);
        self.tx_shmem.send(data.len());
        log::debug!("Sent packet of len: {}", data.len());
    }

    pub fn copy_of_tx_buffer(&self) -> ShmemNetDeviceBuffer<MmapShMem> {
        self.tx_shmem.clone()
    }

    /// Reset the entire layer 1.
    ///
    /// This empties both buffers and puts them into a ready state.
    pub fn reset(&mut self) {
        self.tx_shmem.reset();
        self.rx_shmem.reset();
    }

    pub fn get_shmem_description(&self) -> ShMemDescription {
        self.rx_shmem.description()
    }

    pub fn respond_manually(parsed: DataLinkLayerPacket) -> Option<Result<Vec<u8>, Error>> {
        if let Some(icmpv6) = parsed.upper().and_then(UpperLayerPacket::get_icmpv6) {
            match icmpv6.icmpv6_type {
                Icmpv6Types::NeighborSolicit => {
                    log::debug!("Manually responding to icmpv6 NeighborSolicit");
                    let res = create_response_to_icmpv6_neighbor_solicitation(&parsed, CLIENT_MAC_ADDR, *IPV6_LINK_LOCAL_ADDR).ok_or({
                        Error::illegal_argument(format!("Could not calculate return package for an incoming icmpv6 message:\n{:?}", parsed))
                    });
                    Some(res)
                }
                Icmpv6Types::RouterSolicit => {
                    log::debug!("Manually responding to icmpv6 RouterSolicit");
                    let res = create_response_to_icmpv6_router_solicitation(&parsed, CLIENT_MAC_ADDR, *IPV6_LINK_LOCAL_ADDR).ok_or({
                        Error::illegal_argument(format!("Could not calculate return package for an incoming icmpv6 message:\n{:?}", parsed))
                    });
                    Some(res)
                }
                _ => {
                    log::debug!(
                        "Received icmpv6 package of type other than NeighborSolicit or RouterSolicit of upper type {:?}",
                        icmpv6.icmpv6_type
                    );
                    None
                }
            }
        } else if parsed.net().is_arp() {
            log::debug!("Manually responding to arp");
            let res = respond_to_arp(&parsed, CLIENT_MAC_ADDR);
            Some(Ok(res))
        } else {
            None
        }
    }
    pub fn init_zephyr(&mut self, mut package_logger: impl FnMut(Vec<u8>)) -> Result<(), Error> {
        let start = Instant::now();
        while start.elapsed() < SETUP_TIMEOUT {
            if let Some(p) = self.try_recv() {
                let parsed =
                    parse_eth(&p).map_err(|e| Error::illegal_argument(format!("{e:?}")))?;
                package_logger(p);
                if let Some(res) = Self::respond_manually(parsed) {
                    match res {
                        Ok(response) => {
                            self.send(&response);
                            package_logger(response);
                        }
                        Err(e) => return Err(e),
                    }
                }
            }
            sleep(Duration::from_millis(5));
        }
        Ok(())
    }
}
