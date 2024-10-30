use core::str;
use std::{
    ffi::CStr,
    os::unix::process::ExitStatusExt as _,
    sync::{
        mpsc::{self, TryRecvError},
        LazyLock,
    },
    thread::sleep,
    time::Duration,
};

use client::setup_client_and_connect;
use libafl::Error;
use libafl_bolts::shmem::{
    MmapShMem, MmapShMemProvider, ShMem as _, ShMemDescription, ShMemProvider as _,
};
use smoltcp::wire::IpAddress;

use crate::{
    runner::spawn::start_zephyr, smoltcp::shmem_net_device::ShmemNetworkDevice, COV_SHMEM_SIZE,
    NETWORK_SHMEM_SIZE,
};

mod client;
mod spawn;

static ZEPHYR_IP: IpAddress = IpAddress::v4(192, 0, 2, 1);
static ZEPHYR_PORT: u16 = 4242;
static CLIENT_PORT: u16 = 13377;
static SETUP_TIMEOUT_MILLIS: u64 = 500; // time waited until client attempts to send data
static MESSAGE: [u8; 13] = *b"Hello, World!";
static IPV6_LINK_LOCAL_ADDR: LazyLock<IpAddress> = LazyLock::new(|| {
    IpAddress::v6(
        0xfe80, 0x0000, 0x0000, 0x0000, 0x0200, 0x5eff, 0xfe00, 0x53ff,
    )
});
static CLIENT_MAC_ADDR: [u8; 6] = [0x00, 0x00, 0x5e, 0x00, 0x53, 0xff];

/// Runs Zephyr and interacts with it.
pub fn run_zephyr(zephyr_exec_path: &str) {
    let (coverage_shmem, coverage_shmem_description) = get_coverage_shmem(COV_SHMEM_SIZE).unwrap();

    let device = ShmemNetworkDevice::new(NETWORK_SHMEM_SIZE);

    let net_shmem_name = CStr::from_bytes_until_nul(device.get_shmem_path())
        .unwrap()
        .to_str()
        .unwrap();

    let net_shmem_size = &device.len().to_string();

    let cov_shmem_size = &coverage_shmem.len().to_string();
    let cov_shmem_name = CStr::from_bytes_until_nul(&coverage_shmem_description.id)
        .unwrap()
        .to_str()
        .unwrap();

    let envs: &[(&str, &str)] = &[
        ("SHMEM_ETH_INTERFACE_SIZE", net_shmem_size),
        ("SHMEM_ETH_INTERFACE_NAME", net_shmem_name),
        ("SHMEM_COVERAGE_NAME", cov_shmem_name),
        ("SHMEM_COVERAGE_SIZE", cov_shmem_size),
    ];

    let args: &[&str] = &[];

    let (tx, rx) = mpsc::channel();

    start_zephyr(
        zephyr_exec_path,
        args,
        envs,
        tx,
        Duration::from_millis(100),
        Duration::from_secs(10),
    );

    log::info!("Started Zephyr");
    let wait = || match rx.try_recv() {
        Err(TryRecvError::Empty) => {
            sleep(Duration::from_millis(10));
            true
        }
        Err(e) => panic!("{}", e),
        Ok(Some(e)) => {
            log::info!(
                "zephyr down with code {:?} and signal {:?}",
                e.code(),
                e.signal()
            );
            assert!(e.success(), "{:?} {:?}", e.code(), e.signal());
            false
        }
        Ok(None) => false, //timeout
    };

    setup_client_and_connect(
        device,
        wait,
        ZEPHYR_IP,
        ZEPHYR_PORT,
        CLIENT_PORT,
        CLIENT_MAC_ADDR,
        *IPV6_LINK_LOCAL_ADDR,
        SETUP_TIMEOUT_MILLIS,
        &MESSAGE,
    );

    log::info!(
        "{} edges visited",
        coverage_shmem.iter().filter(|e| **e != 0).count()
    );
}

fn get_coverage_shmem(size: usize) -> Result<(MmapShMem, ShMemDescription), Error> {
    let mut shmem_provider = MmapShMemProvider::default();
    let shmem = shmem_provider
        .new_shmem(size)
        .expect("Could not get the shared memory map");

    shmem.persist_for_child_processes()?;

    let shmem_description = shmem.description();
    Ok((shmem, shmem_description))
}
