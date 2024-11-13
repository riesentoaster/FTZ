use std::{cell::RefCell, rc::Rc, thread::sleep, time::Duration};

use libafl::Error;
use libafl_bolts::shmem::{MmapShMem, MmapShMemProvider, ShMemDescription, ShMemProvider as _};

use smoltcp::{
    phy::{self, Device, DeviceCapabilities},
    time::Instant,
};

use crate::layers::data_link::parse_eth;

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
            sleep(Duration::from_millis(500));
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

    #[allow(unused)]
    pub fn try_recv(&self) -> Option<Vec<u8>> {
        let mut rx_shmem = self.shmem.clone().into_rx();
        rx_shmem.get_data_and_set_empty()
    }

    #[allow(unused)]
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
    #[allow(unused)]
    pub fn reset(&mut self) {
        self.shmem.reset();
    }

    pub fn get_shmem_description(&self) -> ShMemDescription {
        self.shmem.description()
    }
}

impl Device for ShmemNetworkDevice {
    type RxToken<'a> = RxToken
    where
        Self: 'a;

    type TxToken<'a> = TxToken
    where
        Self: 'a;

    fn receive(&mut self, _timestamp: Instant) -> Option<(Self::RxToken<'_>, Self::TxToken<'_>)> {
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

    fn transmit(&mut self, _timestamp: Instant) -> Option<Self::TxToken<'_>> {
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
