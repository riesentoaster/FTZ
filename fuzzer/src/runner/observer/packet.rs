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
    states: Vec<PacketState>,
    state_map: Vec<u8>,
    start_time: SystemTime,
    use_state_diffs: bool,
}

impl PacketObserver {
    pub fn new(use_state_diffs: bool) -> Self {
        let state_map_size = if use_state_diffs {
            PacketState::array_size().pow(2)
        } else {
            PacketState::array_size()
        };

        Self {
            packets: vec![],
            states: vec![],
            state_map: vec![0; state_map_size],
            start_time: SystemTime::now(),
            use_state_diffs,
        }
    }

    pub fn get_packets(&self) -> &Vec<(Duration, Vec<u8>)> {
        &self.packets
    }

    pub fn get_states_mut(&mut self) -> &mut Vec<u8> {
        &mut self.state_map
    }

    pub fn add_packet(&mut self, packet: Vec<u8>) {
        let state = PacketState::from(&packet as &[u8]);
        self.packets
            .push((self.start_time.elapsed().unwrap(), packet));

        let state_idx = u16::from(&state) as usize;

        let offset = if self.use_state_diffs {
            let prev_state = self.states.last().unwrap_or(&PacketState::Nothing);
            let prev_idx = u16::from(prev_state) as usize;
            Self::calculate_combined_offset(prev_idx, state_idx)
        } else {
            state_idx
        };

        self.state_map[offset] = 1;
        self.states.push(state);
    }

    fn calculate_combined_offset(prev_idx: usize, state_idx: usize) -> usize {
        prev_idx * PacketState::array_size() + state_idx
    }
}

impl<I, S> Observer<I, S> for PacketObserver {
    fn pre_exec(&mut self, _state: &mut S, _input: &I) -> Result<(), Error> {
        self.packets = vec![];
        self.state_map.fill(0);
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
/// Returns constant `false` as [`Feedback::is_interesting`].
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

#[cfg(test)]
mod tests {
    use crate::runner::{observer::state::PacketState, PacketObserver};

    #[test]
    fn calculate_combined_offset() {
        let mut touched = vec![];
        for i in 0..PacketState::array_size() {
            for j in 0..PacketState::array_size() {
                let calculated = PacketObserver::calculate_combined_offset(i, j);
                touched.push(calculated);
            }
        }
        touched.sort();
        touched.windows(2).for_each(|w| {
            assert_eq!(w[0] + 1, w[1]);
        });
        assert_eq!(
            touched.len(),
            PacketState::array_size() * PacketState::array_size()
        );

        let arr = [0; PacketState::array_size()];
        assert_eq!(0, arr[u16::from(&PacketState::Nothing) as usize]); // Make sure everything fits
    }
}
