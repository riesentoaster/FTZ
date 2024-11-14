use std::{
    cell::RefCell,
    rc::Rc,
    thread::sleep,
    time::{Duration, Instant},
};

use libafl::Error;
use libafl_bolts::shmem::{MmapShMem, MmapShMemProvider, ShMemDescription, ShMemProvider as _};

use pnet::packet::icmpv6::Icmpv6Types;
use smoltcp::phy::{self, Device, DeviceCapabilities};

use crate::{
    direction::Direction,
    layers::{
        data_link::parse_eth, interactive::create_response_to_icmpv6_neighbor_solicitation,
        upper::UpperLayerPacket,
    },
    runner::{CLIENT_MAC_ADDR, IPV6_LINK_LOCAL_ADDR, SETUP_TIMEOUT},
};

use super::shmem_net_device_buffers::ShmemNetDeviceBuffers;

pub struct RxToken {
    buf: Vec<u8>,
}

impl phy::RxToken for RxToken {
    fn consume<R, F>(mut self, f: F) -> R
    where
        F: FnOnce(&mut [u8]) -> R,
    {
        log::debug!("Consuming a RxToken");
        f(&mut self.buf)
    }
}

pub struct TxToken {
    shmem: ShmemNetDeviceBuffers<MmapShMem>,
}

impl phy::TxToken for TxToken {
    fn consume<R, F>(mut self, len: usize, f: F) -> R
    where
        F: FnOnce(&mut [u8]) -> R,
    {
        log::debug!("Sending {len} bytes");
        while !self.shmem.is_empty() {
            log::info!("not ready");
            sleep(Duration::from_millis(10));
        }

        let mut buf = vec![0; len];
        let res = f(&mut buf);
        // log::warn!("Sent: {}", BASE64_STANDARD.encode(&buf));

        match parse_eth(&buf) {
            Ok(p) => {
                log::debug!(
                    "Attempting to send packet with len {} of type {}",
                    buf.len(),
                    p.types_to_string()
                );
            }
            Err(e) => panic!("Could not parse outgoing packet: {:?}", e),
        }

        self.shmem.prep_data(len).copy_from_slice(&buf);
        self.shmem.send(len);
        log::debug!("Sent the following packet: {:?}", parse_eth(&buf));
        res
    }
}

pub struct ShmemNetworkDevice {
    shmem: ShmemNetDeviceBuffers<MmapShMem>,
}

impl ShmemNetworkDevice {
    pub fn new(buf_size: usize) -> Result<Self, Error> {
        let shmem = MmapShMemProvider::new()?.new_shmem_persistent(buf_size * 2 + 8)?; // two buffers plus two lengths

        log::debug!("Created ShmemNetworkDevice");
        let mut shmem = ShmemNetDeviceBuffers::new(Rc::new(RefCell::new(shmem)));
        shmem.set_empty(); // clone the references, set the outgoing channel to nothing
        shmem.clone().into_rx().set_empty();
        Ok(Self { shmem })
    }

    pub fn try_recv(&self) -> Option<Vec<u8>> {
        let mut rx_shmem = self.shmem.clone().into_rx();
        rx_shmem.get_data_and_set_empty()
    }

    pub fn send(&mut self, data: &[u8]) {
        self.shmem.prep_data(data.len()).copy_from_slice(data);
        self.shmem.send(data.len());
    }

    #[allow(unused)]
    pub fn log_status(&mut self) {
        let mut binding = self.shmem.clone();
        let tx = binding.get_size();
        let mut binding = self.shmem.clone().into_rx();
        let rx = binding.get_size();
        log::debug!("status update: tx {}, rx {}", tx, rx);
    }

    /// Reset the entire layer 1.
    ///
    /// This empties both buffers and puts them into a ready state.
    pub fn reset(&mut self) {
        self.shmem.reset();
    }

    pub fn get_shmem_description(&self) -> ShMemDescription {
        self.shmem.description()
    }

    pub fn init_zephyr(
        &mut self,
        mut package_logger: impl FnMut(Direction<Vec<u8>>),
    ) -> Result<(), Error> {
        let start = Instant::now();
        while start.elapsed() < SETUP_TIMEOUT {
            if let Some(p) = self.try_recv() {
                let parsed = parse_eth(&p).map_err(Error::illegal_argument)?;
                if let Some(icmpv6) = parsed.upper().and_then(UpperLayerPacket::get_icmpv6) {
                    if icmpv6.icmpv6_type == Icmpv6Types::NeighborSolicit {
                        let res =
                        create_response_to_icmpv6_neighbor_solicitation(&parsed, CLIENT_MAC_ADDR, *IPV6_LINK_LOCAL_ADDR).ok_or({
                            Error::illegal_argument(format!("Could not calculate return package for an incoming icmpv6 message:\n{:?}", parsed))
                        })?;
                        self.send(&res);
                        package_logger(Direction::Outgoing(res));
                    } else {
                        log::debug!(
                            "Received icmpv6 package of type other than NeighborSolicit of upper type {:?}",
                            icmpv6.icmpv6_type
                        );
                    }
                } else {
                    log::info!(
                        "Received weird (i.e. non-icmpv6) package during setup: {:?}",
                        parsed
                    );
                }
                package_logger(Direction::Incoming(p));
            }
        }
        Ok(())
    }
}

impl Device for ShmemNetworkDevice {
    type RxToken<'a> = RxToken
    where
        Self: 'a;

    type TxToken<'a> = TxToken
    where
        Self: 'a;

    fn receive(
        &mut self,
        _timestamp: smoltcp::time::Instant,
    ) -> Option<(Self::RxToken<'_>, Self::TxToken<'_>)> {
        let mut rx_shmem = self.shmem.clone().into_rx();
        rx_shmem.get_data_and_set_empty().map(|data| {
            // log::warn!("Received: {}", BASE64_STANDARD.encode(&data));
            log::debug!("Recieved {} bytes", data.len());
            log::debug!("Package contents: {:?}", parse_eth(&data).unwrap());
            (
                RxToken { buf: data },
                TxToken {
                    shmem: self.shmem.clone(),
                },
            )
        })
    }

    fn transmit(&mut self, _timestamp: smoltcp::time::Instant) -> Option<Self::TxToken<'_>> {
        log::debug!("Retrieving TxToken");
        Some(TxToken {
            shmem: self.shmem.clone(),
        })
    }

    fn capabilities(&self) -> DeviceCapabilities {
        let mut res = DeviceCapabilities::default();
        res.max_transmission_unit = 1500;
        res.medium = phy::Medium::Ethernet;
        res
    }
}
