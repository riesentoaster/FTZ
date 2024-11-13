use core::str;
use std::{process::ExitStatus, thread::JoinHandle, time::Instant};

use libafl::Error;

use smoltcp::{
    iface::{Config, Interface, SocketSet},
    socket::tcp::Socket,
    storage::RingBuffer,
    wire::{EthernetAddress, HardwareAddress, IpAddress, IpCidr, Ipv4Address, Ipv6Address},
};

use crate::{
    runner::{CLIENT_MAC_ADDR, CLIENT_PORT, IPV6_LINK_LOCAL_ADDR, ZEPHYR_IP, ZEPHYR_PORT},
    smoltcp::shmem_net_device::ShmemNetworkDevice,
};

#[allow(unused)]
pub enum WaitResult {
    Continue,
    Exit,
}

#[allow(unused)]
pub fn manually_connect_to_zephyr(
    device: &mut ShmemNetworkDevice,
    mut wait: impl FnMut(&JoinHandle<Result<Option<ExitStatus>, Error>>) -> WaitResult,
    join_handle: &JoinHandle<Result<Option<ExitStatus>, Error>>,
    message: &[u8],
) -> Result<(), Error> {
    let mut iface = create_iface(device, CLIENT_MAC_ADDR, *IPV6_LINK_LOCAL_ADDR)?;

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

    while sockets.get::<Socket<'_>>(handle).is_active() {
        iface.poll(Instant::now().into(), device, &mut sockets);

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

        match wait(join_handle) {
            WaitResult::Continue => continue,
            WaitResult::Exit => break,
        }
    }
    log::info!("Socket no longer active, shutting down");
    Ok(())
}

fn create_iface(
    device: &mut ShmemNetworkDevice,
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
