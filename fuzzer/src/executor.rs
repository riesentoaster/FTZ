use crate::{
    direction::Direction,
    layers::{
        data_link::parse_eth, interactive::respond_to_icmpv6_multicast_listener_report_message,
    },
    pcap::add_packet_to_pcap_file,
    smoltcp::shmem_net_device::ShmemNetworkDevice,
    FUZZER_MAC_ADDR, IPV6_LINK_LOCAL_ADDR,
};
use core::str;
use std::{
    env::args,
    ffi::CStr,
    os::unix::process::ExitStatusExt,
    process::{Command, ExitStatus},
    sync::mpsc::{self, Receiver, Sender, TryRecvError},
    thread::{self, sleep},
    time::{Duration, Instant},
};

use libafl_bolts::shmem::{MmapShMemProvider, ShMem, ShMemProvider};
use smoltcp::{
    iface::{Config, Interface, SocketSet},
    socket::tcp::Socket,
    storage::RingBuffer,
    wire::{EthernetAddress, HardwareAddress, IpAddress, IpCidr, Ipv4Address, Ipv6Address},
};
use wait_timeout::ChildExt as _;

use crate::SHMEM_SIZE;

#[allow(unused)]
#[derive(PartialEq, Clone)]
pub enum RunType {
    Default,
    Strace,
    Gdb,
    NoConnect,
}

#[allow(unused)]
pub fn run_zephyr_manual(ty: RunType) {
    let zephyr_dir = args()
    .nth(1)
    .unwrap_or_else(|| {
        let res = format!("{}/zephyrproject/zephyr", env!("HOME"));
        println!("Did not receive zephyr's working directory as a command line argument, using '{}' instead", res);
        res
    });

    let mut shmem_provider = MmapShMemProvider::new().unwrap();
    let mut shmem = shmem_provider.new_shmem(SHMEM_SIZE * 2).unwrap();
    shmem.persist_for_child_processes().unwrap();

    shmem.fill(0);

    let shmem_path = shmem.filename_path().unwrap();
    let shmem_len = shmem.len();

    let mut device = ShmemNetworkDevice::new(shmem);

    let (tx, rx) = mpsc::channel();

    start_zephyr(zephyr_dir, ty.clone(), shmem_path, shmem_len, tx);

    log::info!("Started Zephyr");

    match ty {
        RunType::NoConnect => wait_to_finish(rx),
        RunType::Gdb => {
            sleep(Duration::from_secs(10));
            log::info!("Starting connection");
            setup_shmem_client_and_connect(device, rx);
        }
        _ => setup_shmem_client_and_connect(device, rx),
    }
}

fn bytes_to_string(bytes: &[u8]) -> &str {
    CStr::from_bytes_until_nul(bytes).unwrap().to_str().unwrap()
}

fn setup_shmem_client_and_connect<S: ShMem>(
    mut device: ShmemNetworkDevice<S>,
    rx: Receiver<Option<ExitStatus>>,
) {
    log::info!("starting interaction to prep zephyr");
    let iface = create_iface(&mut device);
    let start = Instant::now();
    while start.elapsed() < Duration::from_millis(3000) {
        if let Some(p) = device.recv() {
            let parsed = parse_eth(&p).unwrap();
            // log::info!("Took package from zephyr: {:?}", parsed);
            add_packet_to_pcap_file(Direction::Incoming(&p));
            if parsed.upper().is_some_and(|e| !e.is_tcp()) {
                if let Some(res) = respond_to_icmpv6_multicast_listener_report_message(&parsed) {
                    add_packet_to_pcap_file(Direction::Outgoing(&res));
                    device.send(&res);
                }
            } else {
                log::info!("Received weird package during setup: {:?}", parsed);
            }
        }
        sleep(Duration::from_millis(1));
    }

    let wait = || match rx.try_recv() {
        Err(TryRecvError::Empty) => {
            sleep(Duration::from_millis(10));
            true
        }
        Err(e) => panic!("{}", e),
        Ok(Some(e)) => {
            log::info!("zephyr down");
            assert!(e.success(), "{:?} {:?}", e.code(), e.signal());
            false
        }
        Ok(None) => false, //timeout
    };

    connect_to_zephyr(device, wait, iface);
}

fn connect_to_zephyr<S: ShMem>(
    mut device: ShmemNetworkDevice<S>,
    wait: impl FnMut() -> bool,
    mut iface: Interface,
) {
    let mut socket = Socket::new(
        RingBuffer::new([0; 100000].to_vec()),
        RingBuffer::new([0; 100000].to_vec()),
    );

    // iface.routes_mut().update(|r|r[0].);

    let remote = (IpAddress::v4(192, 0, 2, 1), 4242);
    log::info!("Connecting to socket: {:?}", remote);
    socket.connect(iface.context(), remote, 13377).unwrap();
    send_to_socket_manual(
        iface,
        socket,
        &mut device,
        wait,
        "Hello, World from Executor!".as_bytes(),
    );
}

fn send_to_socket_manual<S: ShMem>(
    mut iface: Interface,
    socket_direct: Socket<'_>,
    device: &mut ShmemNetworkDevice<S>,
    mut wait: impl FnMut() -> bool,
    message: &[u8],
) {
    let mut sockets = SocketSet::new(Vec::new());
    let handle = sockets.add(socket_direct);
    let mut done_sending = 0;

    while sockets.get_mut::<Socket<'_>>(handle).is_active() {
        iface.poll(Instant::now().into(), device, &mut sockets);

        let socket: &mut Socket<'_> = sockets.get_mut(handle);

        let received = (socket.may_recv() && socket.can_recv())
            .then(|| {
                let data = socket
                    .recv(|data| {
                        if !data.is_empty() {
                            let mut data = data.to_owned();
                            data = data.split(|&b| b == b'\n').collect::<Vec<_>>().concat();
                            data.reverse();
                            data.extend(b"\n");
                        }
                        (data.len(), data)
                    })
                    .unwrap();
                if data.is_empty() {
                    None
                } else {
                    Some(data)
                }
            })
            .flatten();

        if let Some(data) = received {
            log::info!(
                "recv data: {:?}",
                str::from_utf8(data).unwrap_or("(invalid utf8)")
            );
            continue;
        }

        if socket.may_send() {
            if done_sending == 0 {
                log::info!(
                    "Sending data: {:?}",
                    String::from_utf8(message.to_vec()).unwrap()
                );
                socket.send_slice(message).unwrap();
                done_sending = 1;
            } else if done_sending > 100 {
                log::info!("Closing socket");
                socket.close();
                break;
            } else {
                done_sending += 1;
            }
        }
        if !wait() {
            log::info!("zephyr reached its exec timeout");
            break;
        }
    }
    log::info!("Socket no longer active, shutting down");
}

#[allow(unused)]
fn send_to_socket_per_example<S: ShMem>(
    mut iface: Interface,
    socket: Socket<'_>,
    mut device: ShmemNetworkDevice<S>,
    mut wait: impl FnMut() -> bool,
) {
    let mut sockets = SocketSet::new(Vec::new());
    let handle = sockets.add(socket);

    let mut tcp_active = false;
    loop {
        let timestamp = Instant::now().into();
        iface.poll(timestamp, &mut device, &mut sockets);

        let socket: &mut Socket<'_> = sockets.get_mut(handle);
        if socket.is_active() && !tcp_active {
            log::info!("connected");
        } else if !socket.is_active() && tcp_active {
            log::info!("disconnected");
            break;
        }
        tcp_active = socket.is_active();

        if socket.may_recv() {
            // log::info!("may_recv");
            let data = socket
                .recv(|data| {
                    let mut data = data.to_owned();
                    if !data.is_empty() {
                        log::info!(
                            "recv data: {:?}",
                            str::from_utf8(data.as_ref()).unwrap_or("(invalid utf8)")
                        );
                        data = data.split(|&b| b == b'\n').collect::<Vec<_>>().concat();
                        data.reverse();
                        data.extend(b"\n");
                    }
                    (data.len(), data)
                })
                .unwrap();
            if socket.can_send() && !data.is_empty() {
                log::info!(
                    "send data: {:?}",
                    str::from_utf8(data.as_ref()).unwrap_or("(invalid utf8)")
                );
                socket.send_slice(&data[..]).unwrap();
            }
        } else if socket.may_send() {
            log::info!("closing socket");
            socket.close();
        }
        if !wait() {
            log::info!("zephyr reached its exec timeout");
            break;
        }
    }
}

fn start_zephyr(
    zephyr_dir: String,
    ty: RunType,
    shmem_path: [u8; 20],
    shmem_len: usize,
    tx: Sender<Option<ExitStatus>>,
) {
    thread::spawn(move || {
        sleep(Duration::from_millis(100));
        let zephyr_exec_dir = format!("{zephyr_dir}/build/zephyr/zephyr.exe");
        let options: (&str, Vec<_>, u64) = match ty {
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
                2,
            ),
            RunType::Gdb => ("gdb", vec![&zephyr_exec_dir], u64::MAX),
            _ => (&zephyr_exec_dir, vec![], 10),
        };
        let result = Command::new(options.0)
            .args(options.1)
            .env("SHMEM_ETH_INTERFACE_NAME", bytes_to_string(&shmem_path))
            .env("SHMEM_ETH_INTERFACE_SIZE", shmem_len.to_string())
            .spawn()
            .unwrap()
            .wait_timeout(Duration::from_secs(options.2))
            .unwrap();
        tx.send(result).unwrap();
    });
}

fn wait_to_finish(rx: Receiver<Option<ExitStatus>>) {
    log::info!("Zephyr finished with {:?}", rx.recv());
}

fn create_iface<S: ShMem>(device: &mut ShmemNetworkDevice<S>) -> Interface {
    let mut iface = Interface::new(
        Config::new(HardwareAddress::Ethernet(EthernetAddress(FUZZER_MAC_ADDR))),
        device,
        smoltcp::time::Instant::ZERO,
    );

    iface.update_ip_addrs(|ip_addrs| {
        ip_addrs
            .push(IpCidr::new(IpAddress::v4(192, 0, 2, 2), 24))
            .unwrap();
        ip_addrs
            .push(IpCidr::new(*IPV6_LINK_LOCAL_ADDR, 64))
            .unwrap();
        ip_addrs
            .push(IpCidr::new(IpAddress::v6(0xfdaa, 0, 0, 0, 0, 0, 0, 1), 64))
            .unwrap();
    });

    iface
        .routes_mut()
        .add_default_ipv4_route(Ipv4Address::new(192, 0, 2, 1))
        .unwrap();
    iface
        .routes_mut()
        .add_default_ipv6_route(Ipv6Address::new(0xfe80, 0, 0, 0, 0, 0, 0, 0x100))
        .unwrap();

    iface
}
