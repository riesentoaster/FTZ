use std::{borrow::Cow, collections::HashSet, marker::PhantomData};

use libafl::{
    events::{Event, EventFirer},
    executors::ExitKind,
    feedbacks::{Feedback, HasObserverHandle, StateInitializer},
    monitors::{AggregatorOps, UserStats, UserStatsValue},
    Error, HasNamedMetadata,
};
use libafl_bolts::{
    tuples::{Handle, Handled, MatchNameRef},
    Named,
};
use serde::{Deserialize, Serialize};

pub trait SparseMapFeedbackObserver {
    fn values(&self) -> impl Iterator<Item = &usize>;
    fn len(&self) -> usize;
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// Metadata for the sparse feedback
#[derive(Debug, Serialize, Deserialize)]
pub struct SparseMapFeedbackMetadata {
    history_indices: HashSet<usize>,
}

impl SparseMapFeedbackMetadata {
    pub fn new() -> Self {
        Self {
            history_indices: HashSet::new(),
        }
    }
}

impl Default for SparseMapFeedbackMetadata {
    fn default() -> Self {
        Self::new()
    }
}

libafl_bolts::impl_serdeany!(SparseMapFeedbackMetadata);

/// A feedback that tracks coverage sparsely, only storing indices that were hit
pub struct SparseMapFeedback<O> {
    name: Cow<'static, str>,
    monitor_name: Cow<'static, str>,
    len: usize,
    observer_handle: Handle<O>,
}

impl<O> Named for SparseMapFeedback<O> {
    fn name(&self) -> &Cow<'static, str> {
        &self.name
    }
}

impl<O> HasObserverHandle for SparseMapFeedback<O> {
    type Observer = O;

    fn observer_handle(&self) -> &Handle<O> {
        &self.observer_handle
    }
}

impl<O> SparseMapFeedback<O>
where
    O: Handled + Named + SparseMapFeedbackObserver,
{
    pub fn new(observer: &O, monitor_name: &'static str) -> Self {
        let len = observer.len();
        let name = Cow::Owned(format!("SparseMapFeedback<{}>", observer.name()));
        let monitor_name = Cow::Borrowed(monitor_name);
        Self {
            name,
            monitor_name,
            len,
            observer_handle: observer.handle(),
        }
    }
}

impl<S, O> StateInitializer<S> for SparseMapFeedback<O>
where
    S: HasNamedMetadata,
{
    fn init_state(&mut self, state: &mut S) -> Result<(), Error> {
        state.add_named_metadata(&self.name, SparseMapFeedbackMetadata::new());
        Ok(())
    }
}

impl<EM, I, OT, S, O> Feedback<EM, I, OT, S> for SparseMapFeedback<O>
where
    OT: MatchNameRef,
    O: SparseMapFeedbackObserver,
    S: HasNamedMetadata,
    EM: EventFirer<I, S>,
{
    fn is_interesting(
        &mut self,
        state: &mut S,
        _manager: &mut EM,
        _input: &I,
        observers: &OT,
        _exit_kind: &ExitKind,
    ) -> Result<bool, Error> {
        let observer = observers.get(&self.observer_handle).unwrap();
        let meta = state
            .named_metadata_map_mut()
            .get_mut::<SparseMapFeedbackMetadata>(&self.name)
            .unwrap();

        let mut interesting = false;

        // Only iterate over indices that were actually hit
        for &idx in observer.values() {
            if !meta.history_indices.contains(&idx) {
                meta.history_indices.insert(idx);
                interesting = true;
            }
        }

        Ok(interesting)
    }

    fn append_metadata(
        &mut self,
        state: &mut S,
        manager: &mut EM,
        _observers: &OT,
        _testcase: &mut libafl::corpus::Testcase<I>,
    ) -> Result<(), Error> {
        let covered = state
            .named_metadata_map_mut()
            .get_mut::<SparseMapFeedbackMetadata>(&self.name)
            .unwrap()
            .history_indices
            .len();

        manager.fire(
            state,
            Event::UpdateUserStats {
                name: self.monitor_name.clone(),
                value: UserStats::new(
                    UserStatsValue::Ratio(covered as u64, self.len as u64),
                    AggregatorOps::Avg,
                ),
                phantom: PhantomData,
            },
        )
    }
}
