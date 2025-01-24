use etherparse::{Ethernet2Header, IpNumber, Ipv4Extensions, Ipv4Header, Payload, TcpHeader};
use libafl::{generators::Generator, nonzero, state::HasRand, Error};
use libafl_bolts::rands::Rand;

use crate::runner::input::{EtherparseInput, ZephyrInputPart};

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
        let payload_raw = (0..payload_len)
            .map(|_| rand.next() as u8)
            .collect::<Vec<u8>>();

        let payload = Payload::Tcp(payload_raw);
        let tcp = TcpHeader::new(
            rand.next() as u16,
            rand.next() as u16,
            rand.next() as u32,
            rand.next() as u16,
        );

        // +CONFIG_ETH_NATIVE_POSIX_MAC_ADDR="02:00:5e:00:53:31"

        let ip = Ipv4Header::new(
            0, // is calculated automatically,
            rand.next() as u8,
            IpNumber::TCP,
            [192, 0, 2, 2],
            [192, 0, 2, 1],
        )
        .map_err(|e| Error::illegal_argument(format!("Could not create Ipv4Header: {}", e)))?;

        let ipv4_extensions = Ipv4Extensions::from_slice_lax(IpNumber(0), &[]).0;
        let eth_raw = [
            0x02, 0x00, 0x5e, 0x00, 0x53, 0x31, 0x00, 0x00, 0x5e, 0x00, 0x53, 0xff, 0x08, 0x00,
        ];
        let eth = Ethernet2Header::from_slice(&eth_raw)
            .map_err(|e| {
                Error::illegal_argument(format!("Could not create Ethernet2Header: {}", e))
            })?
            .0;
        let parsed = EtherparseInput::new(tcp, ip, ipv4_extensions, eth, payload);
        let bytes: Vec<u8> = parsed.into();
        Ok(bytes.into())
    }
}
