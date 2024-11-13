use std::borrow::Cow;

use libafl::{
    corpus::Testcase,
    executors::ExitKind,
    feedbacks::{Feedback, StateInitializer},
    observers::Observer,
    Error, HasNamedMetadata, SerdeAny,
};
use libafl_bolts::{
    tuples::{Handle, Handled, MatchNameRef},
    Named,
};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct PacketObserver {
    packets: Vec<Vec<u8>>,
}

impl PacketObserver {
    pub fn new() -> Self {
        Self { packets: vec![] }
    }

    fn get_packets(&self) -> &Vec<Vec<u8>> {
        &self.packets
    }

    pub fn add_packet(&mut self, packet: Vec<u8>) {
        self.packets.push(packet);
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
struct PacketMetadata {
    packets: Vec<Vec<u8>>,
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

impl<EM, I, OT, S> Feedback<EM, I, OT, S> for PacketFeedback
where
    S: HasNamedMetadata,
    OT: MatchNameRef,
{
    fn append_metadata(
        &mut self,
        state: &mut S,
        _manager: &mut EM,
        observers: &OT,
        _testcase: &mut Testcase<I>,
    ) -> Result<(), Error> {
        let observer = observers
            .get(&self.packet_observer)
            .ok_or(Error::illegal_argument(
            "Could not retrieve PacketObserver, make sure you pass it to the executor in the OT.",
        ))?;

        let packets = observer.get_packets().to_vec();

        state.add_named_metadata("packets", PacketMetadata { packets });
        Ok(())
    }

    fn discard_metadata(&mut self, state: &mut S, _input: &I) -> Result<(), Error>
    where
        S: HasNamedMetadata,
        OT: MatchNameRef,
    {
        state.remove_named_metadata::<PacketMetadata>("packets");
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

impl<S> StateInitializer<S> for PacketFeedback {}
