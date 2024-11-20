use std::borrow::Cow;

use libafl::{
    corpus::Testcase,
    executors::ExitKind,
    feedbacks::{Feedback, StateInitializer},
    Error, HasMetadata as _, SerdeAny,
};
use libafl_bolts::Named;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, SerdeAny, Serialize, Deserialize)]
struct ExitKindMetadata {
    exit_kind: ExitKind,
}

/// Feedback that marks inputs that result in an [`ExitKind::Crash`] as interesting. Additionally adds the [`ExitKind`] to a metadata field.
pub struct CrashLoggingFeedback {
    exit_kind: Option<ExitKind>,
}

impl<S> StateInitializer<S> for CrashLoggingFeedback {}

impl Named for CrashLoggingFeedback {
    fn name(&self) -> &Cow<'static, str> {
        &Cow::Borrowed("ExitKindFeedback")
    }
}

impl CrashLoggingFeedback {
    pub fn new() -> Self {
        Self { exit_kind: None }
    }
}

impl<EM, I, OT, S> Feedback<EM, I, OT, S> for CrashLoggingFeedback {
    fn is_interesting(
        &mut self,
        _state: &mut S,
        _manager: &mut EM,
        _input: &I,
        _observers: &OT,
        exit_kind: &ExitKind,
    ) -> Result<bool, Error> {
        self.exit_kind = Some(*exit_kind);
        Ok(matches!(exit_kind, ExitKind::Crash))
    }

    fn append_metadata(
        &mut self,
        _state: &mut S,
        _manager: &mut EM,
        _observers: &OT,
        testcase: &mut Testcase<I>,
    ) -> Result<(), Error> {
        let exit_kind = self.exit_kind.ok_or(Error::empty_optional(
            "No ExitKind was stored before appending metadata",
        ))?;
        testcase.add_metadata(ExitKindMetadata { exit_kind });
        Ok(())
    }
}
