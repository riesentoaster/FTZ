use core::str;
use std::{
    thread::sleep,
    time::{Duration, Instant},
};

use pnet::packet::icmpv6::Icmpv6Types;

use smoltcp::{
    iface::{Config, Interface, SocketSet},
    socket::tcp::Socket,
    storage::RingBuffer,
    wire::{EthernetAddress, HardwareAddress, IpAddress, IpCidr, Ipv4Address, Ipv6Address},
};

use crate::{
    direction::Direction,
    layers::{
        data_link::parse_eth, interactive::respond_to_icmpv6_neighbor_solicitation,
        upper::UpperLayerPacket,
    },
    pcap::{add_packet_to_pcap_file, dump_to_pcap_file},
    smoltcp::shmem_net_device::ShmemNetworkDevice,
    PCAP_PATH,
};

/// Based on the provided [`ShmemNetworkDevice`], a [`smoltcp`] interface is created and then used to open a socket to the provided remote ip and port.
///
/// The specified client port is used in client. The provided message is sent once the setup is completed.
/// This entails waiting for at least `setup_timeout_millis` ms, while receiving and sending packages as necessary. (1ms [`sleep`] in between each packet)
/// Specifically, incoming ICMPv6 packages of type [`Icmpv6Types::NeighborSolicit`] are manually responded to according to [`respond_to_icmpv6_neighbor_solicitation`].
///
/// Finally, `wait` is called during the interaction phase after the setup after each send/recv iteration. Once it returns `false`, the client is stopped.
#[allow(clippy::too_many_arguments)]
pub fn setup_client_and_connect(
    mut device: ShmemNetworkDevice,
    wait: impl FnMut() -> bool,
    remote_ip: IpAddress,
    remote_port: u16,
    client_port: u16,
    client_mac: [u8; 6],
    ipv6_link_local_addr: IpAddress,
    setup_timeout_millis: u64,
    message: &[u8],
) {
    log::info!("starting interaction to prep zephyr");

    let iface = create_iface(&mut device, client_mac, ipv6_link_local_addr);

    zephyr_init_phase(
        setup_timeout_millis,
        &mut device,
        client_mac,
        ipv6_link_local_addr,
    );

    log::info!("Done with setup and initialization of Zephyr");

    zephyr_interaction_phase(
        device,
        wait,
        iface,
        remote_ip,
        remote_port,
        client_port,
        message,
    );
}

fn zephyr_init_phase(
    setup_timeout_millis: u64,
    device: &mut ShmemNetworkDevice,
    client_mac: [u8; 6],
    ipv6_link_local_addr: IpAddress,
) {
    let start = Instant::now();
    while start.elapsed() < Duration::from_millis(setup_timeout_millis) {
        if let Some(p) = device.try_recv() {
            let parsed = parse_eth(&p).unwrap();
            add_packet_to_pcap_file(Direction::Incoming(&p));
            if let Some(icmpv6) = parsed.upper().and_then(UpperLayerPacket::get_icmpv6) {
                if icmpv6.icmpv6_type == Icmpv6Types::NeighborSolicit {
                    let res =
                    respond_to_icmpv6_neighbor_solicitation(&parsed, client_mac, ipv6_link_local_addr).unwrap_or_else(|| {
                        dump_to_pcap_file(PCAP_PATH).unwrap();
                        panic!("Could not calculate return package for an incoming icmpv6 message:\n{:?}", parsed);
                    });
                    add_packet_to_pcap_file(Direction::Outgoing(&res));
                    device.send(&res);
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
        }
        sleep(Duration::from_millis(1));
    }
}

fn zephyr_interaction_phase(
    mut device: ShmemNetworkDevice,
    mut wait: impl FnMut() -> bool,
    mut iface: Interface,
    remote_ip: IpAddress,
    remote_port: u16,
    client_port: u16,
    message: &[u8],
) {
    let mut socket = Socket::new(
        RingBuffer::new([0; 100000].to_vec()),
        RingBuffer::new([0; 100000].to_vec()),
    );

    log::info!("Connecting to socket on {}:{}", remote_ip, remote_port);
    socket
        .connect(iface.context(), (remote_ip, remote_port), client_port)
        .unwrap();

    let mut sockets = SocketSet::new(Vec::new());
    let handle = sockets.add(socket);
    let mut iters_since_sending = 0;

    while sockets.get::<Socket<'_>>(handle).is_active() {
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
                    String::from_utf8(message.to_vec()).unwrap()
                );
                socket.send_slice(message).unwrap();
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
        if !wait() {
            log::info!("Zephyr reached its exec timeout, breaking from recv/send loop");
            break;
        }
    }
    log::info!("Socket no longer active, shutting down");
}

fn create_iface(
    device: &mut ShmemNetworkDevice,
    client_mac: [u8; 6],
    ipv6_link_local_addr: IpAddress,
) -> Interface {
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
        .unwrap();
    iface
        .routes_mut()
        .add_default_ipv6_route(Ipv6Address::new(0xfe80, 0, 0, 0, 0, 0, 0, 0x100))
        .unwrap();

    iface
}
