use std::{borrow::Cow, marker::PhantomData};

use libafl::{
    events::{Event, EventFirer},
    feedbacks::{Feedback, StateInitializer},
    monitors::{AggregatorOps, UserStats, UserStatsValue},
};
use libafl_bolts::Named;

pub use libafl::fuzzer::replaying::HasLen;

pub struct InputLenFeedback;

impl<EM, I, OT, S> Feedback<EM, I, OT, S> for InputLenFeedback
where
    EM: EventFirer<I, S>,
    I: HasLen,
{
    fn is_interesting(
        &mut self,
        _state: &mut S,
        _manager: &mut EM,
        _input: &I,
        _observers: &OT,
        _exit_kind: &libafl::executors::ExitKind,
    ) -> Result<bool, libafl::Error> {
        Ok(false)
    }

    fn append_metadata(
        &mut self,
        state: &mut S,
        manager: &mut EM,
        _observers: &OT,
        testcase: &mut libafl::corpus::Testcase<I>,
    ) -> Result<(), libafl::Error> {
        if let Some(input) = testcase.input() {
            manager.fire(
                state,
                Event::UpdateUserStats {
                    name: Cow::Borrowed("input_len"),
                    value: UserStats::new(
                        UserStatsValue::Number(input.len() as u64),
                        AggregatorOps::Avg,
                    ),
                    phantom: PhantomData,
                },
            )?;
        } else {
            log::warn!("No input in testcase in InputLenFeedback");
        }
        Ok(())
    }
}

impl Named for InputLenFeedback {
    fn name(&self) -> &Cow<'static, str> {
        &Cow::Borrowed("input-len")
    }
}

impl<S> StateInitializer<S> for InputLenFeedback {}
