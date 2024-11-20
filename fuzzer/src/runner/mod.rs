use std::{sync::LazyLock, time::Duration};

use smoltcp::wire::IpAddress;

mod client;
mod executor;
mod fuzzer;
mod generator;
mod input;
mod metadata;
mod objective;

pub use {
    executor::ZepyhrExecutor,
    fuzzer::fuzz,
    generator::ZephyrInteractionGenerator,
    metadata::{PacketFeedback, PacketObserver},
};

pub const ZEPHYR_IP: IpAddress = IpAddress::v4(192, 0, 2, 1);
pub const ZEPHYR_PORT: u16 = 4242;
pub const CLIENT_PORT: u16 = 13377;
pub const SETUP_TIMEOUT: Duration = Duration::from_millis(500 + 200); // time waited until client attempts to send data, the second is taken from the zephyr startup wait time
pub const INTER_SEND_WAIT: Duration = Duration::from_millis(100);

pub static IPV6_LINK_LOCAL_ADDR: LazyLock<IpAddress> = LazyLock::new(|| {
    IpAddress::v6(
        0xfe80, 0x0000, 0x0000, 0x0000, 0x0200, 0x5eff, 0xfe00, 0x53ff,
    )
});
pub const CLIENT_MAC_ADDR: [u8; 6] = [0x00, 0x00, 0x5e, 0x00, 0x53, 0xff];
