use std::{borrow::Cow, marker::PhantomData};

use libafl::{
    events::{Event, EventFirer},
    feedbacks::{Feedback, StateInitializer},
    inputs::UsesInput,
    monitors::{AggregatorOps, UserStats, UserStatsValue},
    Error, HasNamedMetadata,
};
use libafl_bolts::Named;

static FREE_MEMORY_NAME: Cow<'static, str> = Cow::Borrowed("free_memory");

/// Feedback that tracks the available memory in the system.
pub struct MemoryPseudoFeedback;

impl<EM, I, OT, S> Feedback<EM, I, OT, S> for MemoryPseudoFeedback
where
    S: HasNamedMetadata + UsesInput,
    EM: EventFirer<State = S>,
{
    fn append_metadata(
        &mut self,
        state: &mut S,
        manager: &mut EM,
        _observers: &OT,
        _testcase: &mut libafl::corpus::Testcase<I>,
    ) -> Result<(), Error> {
        let free_memory = sys_info::mem_info()
            .map_err(|e| Error::illegal_state(e.to_string()))?
            .avail as usize;

        manager.fire(
            state,
            Event::UpdateUserStats {
                name: FREE_MEMORY_NAME.clone(),
                value: UserStats::new(
                    UserStatsValue::Number(free_memory as u64),
                    AggregatorOps::Min,
                ),
                phantom: PhantomData,
            },
        )
    }
}

impl Named for MemoryPseudoFeedback {
    fn name(&self) -> &Cow<'static, str> {
        &Cow::Borrowed("memory-observer")
    }
}

impl<S> StateInitializer<S> for MemoryPseudoFeedback {}
