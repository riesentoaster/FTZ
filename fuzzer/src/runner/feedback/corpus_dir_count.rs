use std::{
    borrow::Cow,
    marker::PhantomData,
    path::{Path, PathBuf},
    time::{Duration, SystemTime},
};

use libafl::{
    events::{Event, EventFirer},
    feedbacks::{Feedback, StateInitializer},
    monitors::{AggregatorOps, UserStats, UserStatsValue},
    Error, HasNamedMetadata,
};
use libafl_bolts::Named;

static CORPUS_DIR_COUNT_NAME: Cow<'static, str> = Cow::Borrowed("corpus_dir_count");

/// Feedback that counts the number of files in the corpus directory.
pub struct CorpusDirCountFeedback {
    dir: PathBuf,
    last_timestamp: SystemTime,
    interval: Duration,
}

impl CorpusDirCountFeedback {
    pub fn new<D: AsRef<Path>>(dir: D, interval: Duration) -> Self {
        Self {
            dir: dir.as_ref().to_path_buf(),
            last_timestamp: SystemTime::now(),
            interval,
        }
    }
}

impl<EM, I, OT, S> Feedback<EM, I, OT, S> for CorpusDirCountFeedback
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
        if self.last_timestamp.elapsed().unwrap() > self.interval && self.dir.exists() {
            let corpus_dir_count = std::fs::read_dir(&self.dir)
                .unwrap()
                .flat_map(|e| e.ok())
                .filter(|e| e.file_name().to_str().is_some_and(|s| !s.starts_with(".")))
                .count();

            manager.fire(
                state,
                Event::UpdateUserStats {
                    name: CORPUS_DIR_COUNT_NAME.clone(),
                    value: UserStats::new(
                        UserStatsValue::Number(corpus_dir_count as u64),
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

impl Named for CorpusDirCountFeedback {
    fn name(&self) -> &Cow<'static, str> {
        &Cow::Borrowed("corpus-dir-count")
    }
}

impl<S> StateInitializer<S> for CorpusDirCountFeedback {}
