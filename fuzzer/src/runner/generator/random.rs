use etherparse::{PacketBuilder, PacketHeaders, TcpOptionElement, TcpOptions};
use libafl::{generators::Generator, nonzero, state::HasRand, Error};
use libafl_bolts::rands::Rand;

use crate::{packets::outgoing_tcp_packets, runner::input::ZephyrInputPart};

pub struct RandomTcpZephyrInputPartGenerator;

impl<I, S> Generator<I, S> for RandomTcpZephyrInputPartGenerator
where
    I: ZephyrInputPart + From<Vec<u8>>,
    Vec<u8>: From<I>,
    S: HasRand,
{
    fn generate(&mut self, state: &mut S) -> Result<I, Error> {
        let rand = state.rand_mut();

        let payload_len = rand.below(nonzero!(1000));
        let payload = (0..payload_len)
            .map(|_| rand.next() as u8)
            .collect::<Vec<u8>>();

        let outgoing_packets = outgoing_tcp_packets();
        let blueprint = PacketHeaders::from_ethernet_slice(&outgoing_packets[0]).unwrap();
        let eth = blueprint.link.unwrap().ethernet2().unwrap();
        let net = blueprint.net.unwrap();
        let (ipv4, _ipv4_extensions) = net.ipv4_ref().unwrap();

        let builder = PacketBuilder::ethernet2(eth.source, eth.destination)
            .ipv4(ipv4.source, ipv4.destination, rand.next() as u8)
            .tcp(
                rand.next() as u16,
                rand.next() as u16,
                rand.next() as u32,
                rand.next() as u16,
            );

        let builder = if rand.coinflip(0.5) {
            builder.ns()
        } else {
            builder
        };

        let builder = if rand.coinflip(0.5) {
            builder.ack(rand.next() as u32)
        } else {
            builder
        };

        let builder = if rand.coinflip(0.5) {
            builder.urg(rand.next() as u16)
        } else {
            builder
        };

        let builder = if rand.coinflip(0.5) {
            builder.psh()
        } else {
            builder
        };

        let builder = if rand.coinflip(0.5) {
            builder.rst()
        } else {
            builder
        };

        let builder = if rand.coinflip(0.5) {
            builder.syn()
        } else {
            builder
        };

        let builder = if rand.coinflip(0.5) {
            builder.fin()
        } else {
            builder
        };

        let builder = if rand.coinflip(0.5) {
            builder.ece()
        } else {
            builder
        };

        let builder = if rand.coinflip(0.5) {
            builder.cwr()
        } else {
            builder
        };

        let options = (0..rand.below(nonzero!(5)))
            .map(|_| match rand.below(nonzero!(6)) {
                0 => TcpOptionElement::MaximumSegmentSize(rand.next() as u16),
                1 => TcpOptionElement::WindowScale(rand.next() as u8),
                2 => TcpOptionElement::SelectiveAcknowledgementPermitted,
                3 => {
                    let mut acks = [None; 3];
                    if rand.coinflip(0.5) {
                        acks[0] = Some((rand.next() as u32, rand.next() as u32));
                    }
                    if rand.coinflip(0.5) {
                        acks[1] = Some((rand.next() as u32, rand.next() as u32));
                    }
                    if rand.coinflip(0.5) {
                        acks[2] = Some((rand.next() as u32, rand.next() as u32));
                    }
                    TcpOptionElement::SelectiveAcknowledgement(
                        (rand.next() as u32, rand.next() as u32),
                        acks,
                    )
                }
                4 => TcpOptionElement::Timestamp(rand.next() as u32, rand.next() as u32),
                5 => TcpOptionElement::Noop,
                _ => panic!("Something is rotten in the state of Denmark"),
            })
            .collect::<Vec<_>>();

        let builder = if TcpOptions::try_from_elements(&options).is_ok() && rand.coinflip(0.5) {
            builder.options(&options).unwrap()
        } else {
            builder
        };

        let mut bytes = Vec::<u8>::with_capacity(builder.size(payload.len()));
        builder.write(&mut bytes, &payload).unwrap();

        Ok(bytes.into())
    }
}

