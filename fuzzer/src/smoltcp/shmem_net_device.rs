use std::{cell::RefCell, rc::Rc, thread::sleep, time::Duration};

use libafl_bolts::shmem::ShMem;

use smoltcp::{
    phy::{self, Device, DeviceCapabilities},
    time::Instant,
};

use crate::{direction::Direction, layers::data_link::parse_eth, pcap::add_packet_to_pcap_file};

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

pub struct TxToken<S>
where
    S: ShMem,
{
    shmem: ShmemNetDeviceBuffers<S>,
}

impl<S> phy::TxToken for TxToken<S>
where
    S: ShMem,
{
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

        match parse_eth(&buf) {
            Ok(p) => {
                // if p.net().is_arp() {
                //     log::info!("Attempting to send ARP packet, manually responding.");
                //     add_packet_to_pcap_file(Direction::Outgoing(&buf));
                //     log::debug!("Request with len {}: {:?}", buf.len(), p);
                //     buf = respond_to_arp(&p);
                //     len = buf.len();
                //     log::debug!(
                //         "Response with len {}: {:?}",
                //         buf.len(),
                //         parse_eth(&buf).unwrap()
                //     );
                //     let mut rx_shmem = self.shmem.into_rx();
                //     rx_shmem.prep_data(len).copy_from_slice(&buf);
                //     rx_shmem.send(len);
                //     return res;
                // }

                log::debug!(
                    "Attempting to send packet with len {} of type {}",
                    buf.len(),
                    p.types_to_string()
                );
            }
            Err(e) => panic!("Could not parse outgoing packet: {:?}", e),
        }

        add_packet_to_pcap_file(Direction::Outgoing(&buf));

        self.shmem.prep_data(len).copy_from_slice(&buf);
        self.shmem.send(len);
        log::debug!("Sent the following packet: {:?}", parse_eth(&buf));
        res
    }
}

pub(crate) struct ShmemNetworkDevice<S>
where
    S: ShMem,
{
    shmem: ShmemNetDeviceBuffers<S>,
}

impl<S> ShmemNetworkDevice<S>
where
    S: ShMem,
{
    pub fn new(shmem: S) -> Self {
        log::debug!("Created ShmemNetworkDevice");
        let mut shmem = ShmemNetDeviceBuffers::new(Rc::new(RefCell::new(shmem)));
        shmem.set_empty(); // clone the references, set the outgoing channel to nothing
        shmem.clone().into_rx().set_empty();
        Self { shmem }
    }

    #[allow(unused)]
    pub fn recv(&self) -> Option<Vec<u8>> {
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
}

impl<S> Device for ShmemNetworkDevice<S>
where
    S: ShMem,
{
    type RxToken<'a> = RxToken
    where
        Self: 'a;

    type TxToken<'a> = TxToken<S>
    where
        Self: 'a;

    fn receive(&mut self, _timestamp: Instant) -> Option<(Self::RxToken<'_>, Self::TxToken<'_>)> {
        let mut rx_shmem = self.shmem.clone().into_rx();
        rx_shmem.get_data_and_set_empty().map(|data| {
            log::debug!("Recieved {} bytes", data.len());
            log::debug!("Package contents: {:?}", parse_eth(&data).unwrap());
            add_packet_to_pcap_file(Direction::Incoming(&data));
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
