use std::{
    thread::sleep,
    time::{Duration, Instant},
};

use libafl::Error;
use pnet::packet::icmpv6::Icmpv6Types;

use crate::{
    direction::Direction,
    layers::{
        data_link::parse_eth, interactive::create_response_to_icmpv6_neighbor_solicitation,
        upper::UpperLayerPacket,
    },
    smoltcp::shmem_net_device::ShmemNetworkDevice,
};

use super::{CLIENT_MAC_ADDR, IPV6_LINK_LOCAL_ADDR, SETUP_TIMEOUT};

pub fn init_zephyr(
    device: &mut ShmemNetworkDevice,
    mut package_logger: impl FnMut(Direction<Vec<u8>>),
) -> Result<(), Error> {
    let start = Instant::now();
    while start.elapsed() < SETUP_TIMEOUT {
        if let Some(p) = device.try_recv() {
            let parsed = parse_eth(&p).map_err(Error::illegal_argument)?;
            if let Some(icmpv6) = parsed.upper().and_then(UpperLayerPacket::get_icmpv6) {
                if icmpv6.icmpv6_type == Icmpv6Types::NeighborSolicit {
                    let res =
                    create_response_to_icmpv6_neighbor_solicitation(&parsed, CLIENT_MAC_ADDR, *IPV6_LINK_LOCAL_ADDR).ok_or({
                        Error::illegal_argument(format!("Could not calculate return package for an incoming icmpv6 message:\n{:?}", parsed))
                    })?;
                    device.send(&res);
                    package_logger(Direction::Outgoing(res));
                } else {
                    log::debug!(
                        "Received icmpv6 package of type other than NeighborSolicit: {:?}",
                        parsed
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
        sleep(Duration::from_millis(1));
    }
    Ok(())
}
