use crate::pcap::write_pcap;
use base64::prelude::*;
use libafl::{
    corpus::Testcase,
    executors::ExitKind,
    feedbacks::{Feedback, StateInitializer},
    observers::Observer,
    stages::calibrate::SerializeObserver,
    Error, HasMetadata, SerdeAny,
};
use libafl_bolts::{
    generic_hash_std,
    tuples::{Handle, MatchNameRef},
    Named,
};
use serde::{Deserialize, Serialize};
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
    state_map: Option<Vec<u8>>,
    start_time: SystemTime,
    use_state_diffs: bool,
}

impl PacketObserver {
    pub fn new(use_state_diffs: bool) -> Self {
        Self {
            packets: vec![],
            states: vec![],
            state_map: None,
            start_time: SystemTime::now(),
            use_state_diffs,
        }
    }

    pub fn with_state_map(use_state_diffs: bool) -> Self {
        let state_map_size = if use_state_diffs {
            PacketState::array_size() * PacketState::array_size()
        } else {
            PacketState::array_size()
        };

        Self {
            packets: vec![],
            states: vec![],
            state_map: Some(vec![0; state_map_size]),
            start_time: SystemTime::now(),
            use_state_diffs,
        }
    }

    pub fn get_state_map(&mut self) -> Option<&mut Vec<u8>> {
        self.state_map.as_mut()
    }

    pub fn get_packets(&self) -> &Vec<(Duration, Vec<u8>)> {
        &self.packets
    }

    pub fn add_packet(&mut self, packet: Vec<u8>) {
        let current_state = PacketState::from(&packet as &[u8]);
        self.packets
            .push((self.start_time.elapsed().unwrap(), packet));

        // ignore icmpv6 packets because they're inconsistent
        if matches!(current_state, PacketState::Icmpv6) {
            return;
        }

        let current_idx = u16::from(&current_state) as usize;

        let offset = if self.use_state_diffs {
            let prev_state = self.states.last().unwrap_or(&PacketState::Nothing);
            let prev_idx = u16::from(prev_state) as usize;
            Self::calculate_combined_offset(prev_idx, current_idx)
        } else {
            current_idx
        };

        if let Some(state_map) = self.state_map.as_mut() {
            state_map[offset] = 1;
        }

        self.states.push(current_state);
    }

    fn calculate_combined_offset(prev_idx: usize, current_idx: usize) -> usize {
        prev_idx * PacketState::array_size() + current_idx
    }
}

impl<I, S> Observer<I, S> for PacketObserver {
    fn pre_exec(&mut self, _state: &mut S, _input: &I) -> Result<(), Error> {
        self.packets.clear();
        self.states.clear();
        if let Some(m) = self.state_map.as_mut() {
            m.fill(0);
        }
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
        &Cow::Borrowed("PacketObserver")
    }
}

#[derive(SerdeAny, Serialize, Deserialize, Debug)]
pub struct PacketMetadata {
    hash: u64,
    pcap: String,
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

        let hash = generic_hash_std(observer.get_packets());

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

        testcase.add_metadata(PacketMetadata { hash, pcap });
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

impl SerializeObserver for PacketObserver {
    fn serialize_observer(&self) -> String {
        let mut writer = Vec::new();
        write_pcap(
            &self
                .get_packets()
                .iter()
                .map(|(d, p)| (d, p))
                .collect::<Vec<_>>(),
            &mut writer,
        )
        .unwrap();
        let pcap = BASE64_STANDARD.encode(writer);

        let states = self
            .states
            .iter()
            .map(|s| format!("{:?}", s))
            .collect::<Vec<_>>();

        let state_map = self
            .state_map
            .as_ref()
            .map(|s| s.iter().map(|i| i.to_string()).collect::<Vec<_>>().join(""));

        let serialized = SerializedPacketObserver {
            pcap,
            states,
            state_map,
            use_state_diffs: self.use_state_diffs,
        };
        serde_json::to_string_pretty(&serialized).unwrap()
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
