use crate::{direction::Source, pcap::write_pcap};
use base64::prelude::*;
use libafl::{
    corpus::Testcase,
    executors::ExitKind,
    feedbacks::{Feedback, StateInitializer},
    observers::Observer,
    // replaying::ObserverWithMetadata,
    Error,
    HasMetadata,
    SerdeAny,
};
use libafl_bolts::{
    generic_hash_std,
    tuples::{Handle, MatchNameRef},
    Named,
};
use serde::{Deserialize, Serialize};
use std::{
    borrow::Cow,
    ops::Deref,
    time::{Duration, SystemTime},
};

use super::state::PacketState;

const MAX_PACKETS: usize = 100;

#[derive(Debug, Serialize, Deserialize)]
pub struct PacketObserver {
    packets: Vec<(Duration, Vec<u8>)>,
    states: Vec<Source<PacketState>>,
    state_map: Vec<u8>,
    start_time: SystemTime,
    use_state_diffs: bool,
}

// impl ObserverWithMetadata for PacketObserver {
//     fn metadata(&self) -> String {
//         serde_json::to_string_pretty(&self.get_metadata().unwrap()).unwrap()
//     }
// }

impl PacketObserver {
    pub fn new(use_state_diffs: bool) -> Self {
        let state_map_size = if use_state_diffs {
            PacketState::array_size() * PacketState::array_size()
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

    pub fn get_state_map(&mut self) -> &mut Vec<u8> {
        &mut self.state_map
    }

    pub fn get_packets(&self) -> &Vec<(Duration, Vec<u8>)> {
        &self.packets
    }

    pub fn add_packet(&mut self, packet: Source<Vec<u8>>) {
        let current_state = packet.map(|p| PacketState::from(p.as_slice()));

        // ignore icmpv6 packets because they're inconsistent
        if matches!(*current_state, PacketState::Icmpv6) {
            return;
        }

        let current_idx = u16::from(&*current_state) as usize;

        let offset = if self.use_state_diffs {
            // only update states on incoming packets, otherwise any flag combination in front of nothing is a new combo.
            let prev_state = matches!(current_state, Source::Server(..)).then(|| {
                self.states
                    .last()
                    .map(Source::deref)
                    .unwrap_or(&PacketState::Nothing)
            });

            let prev_idx = prev_state.map(|p| u16::from(p) as usize);
            prev_idx.map(|p| Self::calculate_combined_offset(p, current_idx))
        } else {
            Some(current_idx)
        };

        if let Some(offset) = offset {
            self.state_map[offset] = 1;
        }

        self.states.push(current_state);
        self.packets
            .push((self.start_time.elapsed().unwrap(), packet.inner()));
    }

    fn calculate_combined_offset(prev_idx: usize, current_idx: usize) -> usize {
        prev_idx * PacketState::array_size() + current_idx
    }

    pub fn get_metadata(&self) -> Result<PacketMetadata, Error> {
        let hash = generic_hash_std(self.get_packets());

        let mut writer = Vec::new();
        write_pcap(
            &self
                .get_packets()
                .iter()
                .map(|(d, p)| (d, p))
                .collect::<Vec<_>>(),
            &mut writer,
        )?;
        let pcap = BASE64_STANDARD.encode(writer);

        let states = self
            .states
            .iter()
            .map(|s| format!("{:?}", s))
            .collect::<Vec<_>>();
        let state_map = self
            .state_map
            .iter()
            .map(|s| s.to_string())
            .collect::<Vec<_>>()
            .join("");

        Ok(PacketMetadata {
            hash,
            pcap,
            states,
            state_map,
        })
    }
}

impl<I, S> Observer<I, S> for PacketObserver {
    fn pre_exec(&mut self, _state: &mut S, _input: &I) -> Result<(), Error> {
        self.packets.clear();
        self.states.clear();
        self.state_map.fill(0);
        self.start_time = SystemTime::now();

        Ok(())
    }

    fn pre_exec_child(&mut self, state: &mut S, input: &I) -> Result<(), Error> {
        self.pre_exec(state, input)
    }

    fn post_exec_child(
        &mut self,
        _state: &mut S,
        _input: &I,
        _exit_kind: &ExitKind,
    ) -> Result<(), Error> {
        Ok(())
    }
}

impl Named for PacketObserver {
    fn name(&self) -> &Cow<'static, str> {
        &Cow::Borrowed("packet-observer")
    }
}

#[derive(SerdeAny, Serialize, Deserialize, Debug)]
pub struct PacketMetadata {
    hash: u64,
    pcap: String,
    states: Vec<String>,
    state_map: String,
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

        let metadata = observer.get_metadata()?;

        testcase.add_metadata(metadata);
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

impl AsRef<Self> for PacketObserver {
    fn as_ref(&self) -> &Self {
        self
    }
}

impl AsMut<Self> for PacketObserver {
    fn as_mut(&mut self) -> &mut Self {
        self
    }
}

#[derive(Serialize)]
struct SerializedPacketObserver {
    pcap: String,
    states: Vec<String>,
    state_map: Option<String>,
    use_state_diffs: bool,
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
