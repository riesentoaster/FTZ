use std::{
    borrow::Cow,
    marker::PhantomData,
    time::{Duration, SystemTime},
};

use libafl::{
    events::{Event, EventFirer},
    feedbacks::{Feedback, StateInitializer},
    monitors::{AggregatorOps, UserStats, UserStatsValue},
    Error, HasNamedMetadata,
};
use libafl_bolts::Named;

static FREE_MEMORY_NAME: Cow<'static, str> = Cow::Borrowed("free_memory");

/// Feedback that tracks the available memory in the system.
pub struct MemoryPseudoFeedback {
    interval: Duration,
    last_timestamp: SystemTime,
}

impl MemoryPseudoFeedback {
    pub fn new(interval: Duration) -> Self {
        Self {
            interval,
            last_timestamp: SystemTime::now(),
        }
    }
}

impl<EM, I, OT, S> Feedback<EM, I, OT, S> for MemoryPseudoFeedback
where
    S: HasNamedMetadata,
    EM: EventFirer<I, S>,
{
    fn is_interesting(
        &mut self,
        state: &mut S,
        manager: &mut EM,
        _input: &I,
        _observers: &OT,
        _exit_kind: &libafl::executors::ExitKind,
    ) -> Result<bool, Error> {
        if self.last_timestamp.elapsed().unwrap() > self.interval {
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
            )?;
            self.last_timestamp = SystemTime::now();
        }
        Ok(false)
    }
}

impl Named for MemoryPseudoFeedback {
    fn name(&self) -> &Cow<'static, str> {
        &Cow::Borrowed("memory-observer")
    }
}

impl<S> StateInitializer<S> for MemoryPseudoFeedback {}
