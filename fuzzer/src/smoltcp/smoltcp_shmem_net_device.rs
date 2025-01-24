use std::{thread::sleep, time::Duration};

use libafl_bolts::shmem::MmapShMem;
use smoltcp::phy::{self, Device, DeviceCapabilities};

use crate::{
    direction::Direction, layers::data_link::parse_eth,
    smoltcp::shmem_net_device::ShmemNetworkDevice,
};

use super::shmem_net_device_buffers::ShmemNetDeviceBuffer;

pub struct SmoltcpShmemNetworkDevice {
    device: ShmemNetworkDevice,
    packet_logger: Box<dyn Fn(Direction<Vec<u8>>) + Send>,
}

impl SmoltcpShmemNetworkDevice {
    pub fn new(
        device: ShmemNetworkDevice,
        packet_logger: impl Fn(Direction<Vec<u8>>) + Send + 'static,
    ) -> Self {
        Self {
            device,
            packet_logger: Box::new(packet_logger),
        }
    }
}

impl Device for SmoltcpShmemNetworkDevice {
    type RxToken<'a>
        = RxToken
    where
        Self: 'a;

    type TxToken<'a>
        = TxToken<'a>
    where
        Self: 'a;

    fn receive(
        &mut self,
        _timestamp: smoltcp::time::Instant,
    ) -> Option<(Self::RxToken<'_>, Self::TxToken<'_>)> {
        self.device.try_recv().map(|data| {
            // log::warn!("Received: {}", BASE64_STANDARD.encode(&data));
            log::debug!("Recieved {} bytes", data.len());
            log::debug!("Package contents: {:?}", parse_eth(&data).unwrap());
            (self.packet_logger)(Direction::Incoming(data.clone()));
            (
                RxToken { buf: data },
                TxToken {
                    shmem: self.device.copy_of_tx_buffer(),
                    packet_logger: &self.packet_logger,
                },
            )
        })
    }

    fn transmit(&mut self, _timestamp: smoltcp::time::Instant) -> Option<Self::TxToken<'_>> {
        log::debug!("Retrieving TxToken");
        Some(TxToken {
            shmem: self.device.copy_of_tx_buffer(),
            packet_logger: &self.packet_logger,
        })
    }

    fn capabilities(&self) -> DeviceCapabilities {
        let mut res = DeviceCapabilities::default();
        res.max_transmission_unit = 1500;
        res.medium = phy::Medium::Ethernet;
        res
    }
}

pub struct RxToken {
    buf: Vec<u8>,
}

impl phy::RxToken for RxToken {
    fn consume<R, F>(mut self, f: F) -> R
    where
        F: FnOnce(&mut [u8]) -> R,
    {
        log::debug!("Consuming a RxToken");
        let res = f(&mut self.buf);
        log::debug!("rx is empty: {}", self.buf.is_empty());
        res
    }
}

pub struct TxToken<'a> {
    shmem: ShmemNetDeviceBuffer<MmapShMem>,
    packet_logger: &'a dyn Fn(Direction<Vec<u8>>),
}

impl phy::TxToken for TxToken<'_> {
    fn consume<R, F>(mut self, len: usize, f: F) -> R
    where
        F: FnOnce(&mut [u8]) -> R,
    {
        log::debug!("Sending {len} bytes");
        while !self.shmem.is_empty() {
            log::info!("not ready");
            sleep(Duration::from_millis(1));
        }

        let mut buf = vec![0; len];
        let res = f(&mut buf);

        self.shmem.prep_data(len).copy_from_slice(&buf);
        (self.packet_logger)(Direction::Outgoing(buf.clone()));
        self.shmem.send(len);
        log::debug!("Sent packet of len: {}", len);
        log::debug!("tx is empty: {}", self.shmem.is_empty());
        res
    }
}
