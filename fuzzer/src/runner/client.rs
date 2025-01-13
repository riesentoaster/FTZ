use core::str;
use std::{
    fs::OpenOptions,
    path::PathBuf,
    process::{Child, Command, Stdio},
    sync::{Arc, Mutex},
    thread,
    time::{Duration, Instant},
};

use libafl::Error;
use libafl_bolts::shmem::{ShMem, ShMemDescription};

use smoltcp::{
    iface::{Config, Interface, SocketSet},
    socket::tcp::Socket,
    storage::RingBuffer,
    wire::{EthernetAddress, HardwareAddress, IpAddress, IpCidr, Ipv4Address, Ipv6Address},
};

use crate::{
    runner::{
        get_path, CLIENT_MAC_ADDR, CLIENT_PORT, IPV6_LINK_LOCAL_ADDR, ZEPHYR_IP, ZEPHYR_PORT,
    },
    shmem::get_shmem,
    smoltcp::{
        shmem_net_device::ShmemNetworkDevice, smoltcp_shmem_net_device::SmoltcpShmemNetworkDevice,
    },
    COV_SHMEM_SIZE,
};

/// Initialize shared memory for coverage and network
fn init_shared_memory(
    network_buf_size: usize,
    id: usize,
) -> Result<(ShMemDescription, ShmemNetworkDevice), Error> {
    // Create coverage shared memory
    let cov_shmem = get_shmem(COV_SHMEM_SIZE, id, "cov")?;
    let cov_shmem_description = cov_shmem.description();

    // Create network device with its own shared memory
    let device = ShmemNetworkDevice::new(network_buf_size, id)?;

    Ok((cov_shmem_description, device))
}

/// Setup environment variables for Zephyr process
fn setup_env_vars(
    net_shmem_desc: &ShMemDescription,
    cov_shmem_desc: &ShMemDescription,
) -> Result<Vec<(String, String)>, Error> {
    let envs = ([
        (&"SHMEM_ETH_INTERFACE_SIZE", &net_shmem_desc.size),
        (&"SHMEM_ETH_INTERFACE_NAME", &get_path(net_shmem_desc)?),
        (&"SHMEM_COVERAGE_SIZE", &cov_shmem_desc.size),
        (&"SHMEM_COVERAGE_NAME", &get_path(cov_shmem_desc)?),
    ] as [(&dyn ToString, &dyn ToString); 4])
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();
    Ok(envs)
}

/// Setup stdio redirection for Zephyr process
fn setup_stdio(zephyr_out_path: Option<&PathBuf>) -> (Stdio, Stdio) {
    zephyr_out_path
        .map(|path| {
            let file = OpenOptions::new()
                .create(true)
                .append(true)
                .open(path)
                .expect("Failed to open file");
            (
                Stdio::from(file.try_clone().expect("Could not clone zephyr outfile")),
                Stdio::from(file),
            )
        })
        .unwrap_or((Stdio::null(), Stdio::null()))
}

/// Start Zephyr process with given configuration
fn start_zephyr_process(
    zephyr_exec_path: &PathBuf,
    zephyr_out_path: Option<&PathBuf>,
    envs: Vec<(String, String)>,
) -> Result<Child, Error> {
    let stdio = setup_stdio(zephyr_out_path);
    Command::new(zephyr_exec_path)
        .stdout(stdio.0)
        .stderr(stdio.1)
        .envs(envs)
        .spawn()
        .map_err(|e| Error::unknown(format!("Could not start command: {e:?}")))
}

/// Connect to Zephyr and send a message, waiting for a response
pub fn connect_to_zephyr(
    message: &[u8],
    zephyr_exec_path: &PathBuf,
    zephyr_out_path: Option<&PathBuf>,
    id: usize,
    network_buf_size: usize,
    timeout: Duration,
) -> Result<Vec<(Duration, Vec<u8>)>, Error> {
    // Initialize shared memory
    let (cov_shmem_description, mut device) = init_shared_memory(network_buf_size, id)?;
    let net_shmem_desc = device.get_shmem_description();

    // Setup environment variables and start Zephyr
    let envs = setup_env_vars(&net_shmem_desc, &cov_shmem_description)?;
    let mut child = start_zephyr_process(zephyr_exec_path, zephyr_out_path, envs)?;

    let start_time = Instant::now();
    let packets = Arc::new(Mutex::new(Vec::new()));
    let packets_clone = packets.clone();

    device.init_zephyr(|p| {
        packets
            .lock()
            .unwrap()
            .push((start_time.elapsed(), p.inner()));
    })?;

    let mut device = SmoltcpShmemNetworkDevice::new(device, move |packet| {
        let elapsed = start_time.elapsed();
        packets_clone
            .lock()
            .unwrap()
            .push((elapsed, packet.inner()));
    });

    // Setup network interface
    let mut iface = create_iface(&mut device, CLIENT_MAC_ADDR, *IPV6_LINK_LOCAL_ADDR)?;

    let mut socket = Socket::new(
        RingBuffer::new([0; 100000].to_vec()),
        RingBuffer::new([0; 100000].to_vec()),
    );

    log::info!("Connecting to socket on {}:{}", ZEPHYR_IP, ZEPHYR_PORT);
    socket
        .connect(iface.context(), (ZEPHYR_IP, ZEPHYR_PORT), CLIENT_PORT)
        .map_err(|e| Error::unknown(format!("Could not connect socket: {:#?}", e)))?;

    let mut sockets = SocketSet::new(Vec::new());
    let handle = sockets.add(socket);
    let mut iters_since_sending = 0;
    let start_time = Instant::now();

    while sockets.get::<Socket<'_>>(handle).is_active() {
        if start_time.elapsed() > timeout {
            log::warn!("Timeout reached, stopping");
            break;
        }

        iface.poll(Instant::now().into(), &mut device, &mut sockets);

        let socket: &mut Socket<'_> = sockets.get_mut(handle);

        let received = (socket.may_recv() && socket.can_recv())
            .then(|| socket.recv(|data| (data.len(), data)).unwrap())
            .and_then(|data| (!data.is_empty()).then_some(data));

        if let Some(data) = received {
            log::info!(
                "Received data: {:?}",
                str::from_utf8(data).unwrap_or("(invalid utf8)")
            );
            continue;
        }

        if socket.may_send() {
            if iters_since_sending == 0 {
                log::info!(
                    "Sending data: {:?}",
                    str::from_utf8(message).unwrap_or("(invalid utf8)")
                );
                socket
                    .send_slice(message)
                    .map_err(|e| Error::unknown(format!("Could not send slice: {:#?}", e)))?;
                iters_since_sending = 1;
            } else if iters_since_sending == 10 {
                log::info!("Closing socket");
                socket.close();
            } else if iters_since_sending == 20 {
                log::warn!("Force stopping listening");
                break;
            } else {
                iters_since_sending += 1;
            }
        }

        // Add sleep between iterations
        thread::sleep(Duration::from_millis(5));

        // Check if process is still running
        if let Some(status) = child.try_wait().unwrap() {
            log::warn!("Zephyr process exited with status: {}", status);
            break;
        }
    }

    // Cleanup
    log::info!("Socket no longer active, shutting down");
    child.kill().unwrap();
    child.wait().unwrap();
    let res = Ok(packets.lock().unwrap().clone());
    res
}

fn create_iface(
    device: &mut SmoltcpShmemNetworkDevice,
    client_mac: [u8; 6],
    ipv6_link_local_addr: IpAddress,
) -> Result<Interface, Error> {
    let mut iface = Interface::new(
        Config::new(HardwareAddress::Ethernet(EthernetAddress(client_mac))),
        device,
        smoltcp::time::Instant::ZERO,
    );

    iface.update_ip_addrs(|ip_addrs| {
        ip_addrs
            .push(IpCidr::new(IpAddress::v4(192, 0, 2, 2), 24))
            .unwrap();
        ip_addrs
            .push(IpCidr::new(ipv6_link_local_addr, 64))
            .unwrap();
        ip_addrs
            .push(IpCidr::new(IpAddress::v6(0xfdaa, 0, 0, 0, 0, 0, 0, 1), 64))
            .unwrap();
    });

    iface
        .routes_mut()
        .add_default_ipv4_route(Ipv4Address::new(192, 0, 2, 1))
        .map_err(|e| Error::unknown(format!("Could not add ipv4 route: {e:#?}")))?;
    iface
        .routes_mut()
        .add_default_ipv6_route(Ipv6Address::new(0xfe80, 0, 0, 0, 0, 0, 0, 0x100))
        .map_err(|e| Error::unknown(format!("Could not add ipv6 route: {e:#?}")))?;

    Ok(iface)
}
