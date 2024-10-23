use core::str;
use std::{
    os::unix::process::ExitStatusExt as _,
    sync::{
        mpsc::{self, TryRecvError},
        LazyLock,
    },
    thread::sleep,
    time::Duration,
};

use client::setup_client_and_connect;
use smoltcp::wire::IpAddress;

use crate::{
    runner::spawn::start_zephyr, smoltcp::shmem_net_device::ShmemNetworkDevice, SHMEM_SIZE,
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

#[allow(unused)]
#[derive(PartialEq, Clone)]
pub enum RunType {
    Default,
    Strace,
    Gdb,
    NoConnect,
}

/// Runs Zephyr and interacts with it.
pub fn run_zephyr(zephyr_dir: &str, ty: RunType) {
    let device = ShmemNetworkDevice::new(SHMEM_SIZE);
    let (tx, rx) = mpsc::channel();

    let zephyr_exec_dir = format!("{zephyr_dir}/build/zephyr/zephyr.exe");
    let (cmd, args, timeout): (&str, Vec<_>, u64) = match ty {
        RunType::Strace => (
            "strace",
            vec![
                "-f",
                "-ff",
                "-o",
                "./strace/pid",
                "-e",
                "trace=!clock_nanosleep,futex",
                &zephyr_exec_dir,
            ],
            10,
        ),
        RunType::Gdb => ("gdb", vec![&zephyr_exec_dir], u64::MAX),
        _ => (&zephyr_exec_dir, vec![], 10),
    };

    start_zephyr(
        cmd,
        args,
        device.get_shmem_path(),
        device.len(),
        tx,
        Duration::from_millis(100),
        Duration::from_secs(timeout),
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

    match ty {
        RunType::NoConnect => log::info!("Zephyr finished with {:?}", rx.recv()),
        RunType::Gdb => {
            sleep(Duration::from_secs(10));
            log::info!("Starting connection");
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
        }
        _ => setup_client_and_connect(
            device,
            wait,
            ZEPHYR_IP,
            ZEPHYR_PORT,
            CLIENT_PORT,
            CLIENT_MAC_ADDR,
            *IPV6_LINK_LOCAL_ADDR,
            SETUP_TIMEOUT_MILLIS,
            &MESSAGE,
        ),
    }
}
