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
    tuples::{Handle, Handled, MatchNameRef},
    Named,
};
use serde::{Deserialize, Serialize};
use std::{
    borrow::Cow,
    time::{Duration, SystemTime},
};

#[derive(Serialize, Deserialize, Debug)]
pub struct PacketObserver {
    packets: Vec<(Duration, Vec<u8>)>,
    start_time: SystemTime,
}

impl PacketObserver {
    pub fn new() -> Self {
        Self {
            packets: vec![],
            start_time: SystemTime::now(),
        }
    }

    fn get_packets(&self) -> &Vec<(Duration, Vec<u8>)> {
        &self.packets
    }

    pub fn add_packet(&mut self, packet: Vec<u8>) {
        self.packets
            .push((self.start_time.elapsed().unwrap(), packet));
    }
}

impl<I, S> Observer<I, S> for PacketObserver {
    fn pre_exec(&mut self, _state: &mut S, _input: &I) -> Result<(), Error> {
        self.packets = vec![];
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
    packets: Vec<(Duration, String)>,
    pcap: String,
}

/// Feedback adding packets captured by a [`PacketObserver`] to a metadata field.
///
/// Returns constant `false` as [`Feedback::append_metadata`].
pub struct PacketFeedback {
    packet_observer: Handle<PacketObserver>,
}

impl PacketFeedback {
    pub fn new(packet_observer: &PacketObserver) -> Self {
        Self {
            packet_observer: packet_observer.handle(),
        }
    }
}

impl<S> StateInitializer<S> for PacketFeedback {}

impl<EM, I, OT, S> Feedback<EM, I, OT, S> for PacketFeedback
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
                        Ok(p) => format!("{:?}: {:?}", timestamp, p),
                        Err(p) => format!(
                            "{:?}: Error when parsing packet: {}.\n original data: 0x{:?}",
                            timestamp,
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

        testcase.add_metadata(PacketMetadata {
            packets,
            pcap: BASE64_STANDARD.encode(writer),
        });
        Ok(())
    }

    fn is_interesting(
        &mut self,
        state: &mut S,
        manager: &mut EM,
        input: &I,
        observers: &OT,
        _exit_kind: &ExitKind,
    ) -> Result<bool, Error> {
        self.append_metadata(state, manager, observers, &mut Testcase::new(input))?;
        Ok(false)
    }
}

impl Named for PacketFeedback {
    fn name(&self) -> &Cow<'static, str> {
        &Cow::Borrowed("PacketFeedback")
    }
}
