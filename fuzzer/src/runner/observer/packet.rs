use crate::{layers::data_link::parse_eth, pcap::write_pcap};
use base64::prelude::*;
use libafl::{
    corpus::Testcase,
    executors::ExitKind,
    feedbacks::{Feedback, StateInitializer},
    observers::Observer,
    Error, HasMetadata, SerdeAny,
};
use libafl_bolts::{
    tuples::{Handle, MatchNameRef},
    Named,
};
use serde::{Deserialize, Serialize};
#[cfg(feature = "hashes")]
use std::hash::{DefaultHasher, Hash as _, Hasher};
use std::{
    borrow::Cow,
    time::{Duration, SystemTime},
};

use super::state::PacketState;

const MAX_PACKETS: usize = 100;

#[derive(Debug, Serialize, Deserialize)]
pub struct PacketObserver {
    packets: Vec<(Duration, Vec<u8>)>,
    states: Vec<u16>,
    start_time: SystemTime,
}

impl PacketObserver {
    pub fn new() -> Self {
        Self {
            packets: vec![],
            states: vec![0; MAX_PACKETS],
            start_time: SystemTime::now(),
        }
    }

    pub fn get_packets(&self) -> &Vec<(Duration, Vec<u8>)> {
        &self.packets
    }

    pub fn add_packet(&mut self, packet: Vec<u8>) {
        if let Some(e) = self.states.iter_mut().find(|e| **e == 0) {
            *e = (&PacketState::from(&packet as &[u8])).into();
        }
        self.packets
            .push((self.start_time.elapsed().unwrap(), packet));
    }

    pub fn get_state_map(&mut self) -> &mut [u16] {
        &mut self.states
    }
}

impl<I, S> Observer<I, S> for PacketObserver {
    fn pre_exec(&mut self, _state: &mut S, _input: &I) -> Result<(), Error> {
        self.packets = vec![];
        // self.state is reset by the MapObserver
        Ok(())
    }
}

impl Named for PacketObserver {
    fn name(&self) -> &Cow<'static, str> {
        &Cow::Borrowed("PacketObserver")
    }
}

#[derive(SerdeAny, Serialize, Deserialize, Debug)]
pub struct PacketMetadata {
    #[cfg(feature = "hashes")]
    hash: u64,
    pcap: String,
    packets: Vec<(Duration, String)>,
}

/// Feedback adding packets captured by a [`PacketObserver`] to a metadata field.
///
/// Returns constant `false` as [`Feedback::append_metadata`].
pub struct PacketMetadataFeedback {
    packet_observer: Handle<PacketObserver>,
}

impl PacketMetadataFeedback {
    pub fn new(packet_observer: Handle<PacketObserver>) -> Self {
        Self { packet_observer }
    }
}

impl<S> StateInitializer<S> for PacketMetadataFeedback {}

impl<EM, I, OT, S> Feedback<EM, I, OT, S> for PacketMetadataFeedback
where
    OT: MatchNameRef,
{
    fn append_metadata(
        &mut self,
        _state: &mut S,
        _manager: &mut EM,
        observers: &OT,
        testcase: &mut Testcase<I>,
    ) -> Result<(), Error> {
        let observer = observers
            .get(&self.packet_observer)
            .ok_or(Error::illegal_argument(
            "Could not retrieve PacketObserver, make sure you pass it to the executor in the OT.",
        ))?;

        let packets = observer
            .get_packets()
            .iter()
            .map(|(timestamp, packet)| {
                (
                    *timestamp,
                    match parse_eth(packet) {
                        Ok(p) => format!("{:?}", p),
                        Err(p) => format!(
                            "Error when parsing packet: {:?}.\n original data: 0x{:?}",
                            p,
                            hex::encode(packet)
                        ),
                    },
                )
            })
            .collect();

        let mut writer = Vec::new();
        write_pcap(
            &observer
                .get_packets()
                .iter()
                .map(|(d, p)| (d, p))
                .collect::<Vec<_>>(),
            &mut writer,
        )?;
        let pcap = BASE64_STANDARD.encode(writer);

        #[cfg(feature = "hashes")]
        let hash = {
            let mut hasher = DefaultHasher::new();
            observer
                .get_packets()
                .iter()
                .map(|(_, p)| p.clone())
                .collect::<Vec<_>>()
                .hash(&mut hasher);
            hasher.finish()
        };

        testcase.add_metadata(PacketMetadata {
            packets,
            pcap,
            #[cfg(feature = "hashes")]
            hash,
        });
        Ok(())
    }

    fn is_interesting(
        &mut self,
        _state: &mut S,
        _manager: &mut EM,
        _input: &I,
        _observers: &OT,
        _exit_kind: &ExitKind,
    ) -> Result<bool, Error> {
        Ok(false)
    }
}

impl Named for PacketMetadataFeedback {
    fn name(&self) -> &Cow<'static, str> {
        &Cow::Borrowed("PacketFeedback")
    }
}
