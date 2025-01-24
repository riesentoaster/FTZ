use core::hash::{Hash, Hasher};
use std::borrow::Cow;

use libafl::{
    corpus::CorpusId,
    inputs::{HasMutatorBytes, Input, ResizableMutator},
    mutators::{MutationResult, Mutator},
    Error,
};
use libafl_bolts::{
    generic_hash_std,
    tuples::{Map, MappingFunctor},
    HasLen, Named,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ReplayingStatefulInput<I> {
    parts: Vec<I>,
}

impl<I: Input + Hash> Input for ReplayingStatefulInput<I> {
    fn generate_name(&self, _id: Option<CorpusId>) -> String {
        format!(
            "ReplayingStatefulInput<{},{}>",
            generic_hash_std(&self.parts),
            self.parts.len()
        )
    }
}

impl<I: Hash> Hash for ReplayingStatefulInput<I> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.parts.hash(state);
    }
}

impl<I> From<Vec<I>> for ReplayingStatefulInput<I> {
    fn from(value: Vec<I>) -> Self {
        Self::new(value)
    }
}

impl<I> ReplayingStatefulInput<I> {
    pub fn new(parts: Vec<I>) -> Self {
        Self { parts }
    }

    pub fn map_mutators<M: Map<ToReplayingStatefulMutator>>(
        inner: M,
    ) -> <M as Map<ToReplayingStatefulMutator>>::MapResult {
        inner.map(ToReplayingStatefulMutator)
    }

    pub fn parts(&self) -> &[I] {
        &self.parts
    }

    pub fn parts_mut(&mut self) -> &mut Vec<I> {
        &mut self.parts
    }

    pub fn parts_owned(self) -> Vec<I> {
        self.parts
    }
}
impl<I: Default> ReplayingStatefulInput<I> {
    fn last_or_insert_empty(&mut self) -> &mut I {
        if self.parts.is_empty() {
            self.parts.push(I::default());
        }
        self.parts.last_mut().unwrap()
    }
}
impl<I: HasLen> HasLen for ReplayingStatefulInput<I> {
    fn len(&self) -> usize {
        self.parts.last().map(HasLen::len).unwrap_or(0)
    }
}

impl<I> HasMutatorBytes for ReplayingStatefulInput<I>
where
    I: HasMutatorBytes + Default + From<Vec<u8>>,
{
    fn mutator_bytes(&self) -> &[u8] {
        &[]
    }

    fn mutator_bytes_mut(&mut self) -> &mut [u8] {
        self.last_or_insert_empty().mutator_bytes_mut()
    }
}

impl<I> ResizableMutator<u8> for ReplayingStatefulInput<I>
where
    I: ResizableMutator<u8> + Default + From<Vec<u8>>,
{
    fn resize(&mut self, new_len: usize, value: u8) {
        self.last_or_insert_empty().resize(new_len, value);
    }

    fn extend<'a, J: IntoIterator<Item = &'a u8>>(&mut self, iter: J) {
        self.last_or_insert_empty().extend(iter);
    }

    fn splice<R, J>(&mut self, range: R, replace_with: J) -> std::vec::Splice<'_, J::IntoIter>
    where
        R: std::ops::RangeBounds<usize>,
        J: IntoIterator<Item = u8>,
    {
        self.last_or_insert_empty().splice(range, replace_with)
    }

    fn drain<R>(&mut self, range: R) -> std::vec::Drain<'_, u8>
    where
        R: std::ops::RangeBounds<usize>,
    {
        self.last_or_insert_empty().drain(range)
    }
}

pub struct ReplayingStatefulMutator<M> {
    inner: M,
    name: Cow<'static, str>,
}

impl<M: Named> ReplayingStatefulMutator<M> {
    pub fn new(inner: M) -> Self {
        let name = Cow::Owned(format!("ReplayingStatefulMutator<{}>", inner.name()));
        Self { inner, name }
    }
}

impl<I, S, M> Mutator<ReplayingStatefulInput<I>, S> for ReplayingStatefulMutator<M>
where
    M: Mutator<I, S>,
{
    fn mutate(
        &mut self,
        state: &mut S,
        input: &mut ReplayingStatefulInput<I>,
    ) -> Result<MutationResult, Error> {
        match input.parts.last_mut() {
            Some(inner_input) => self.inner.mutate(state, inner_input),
            None => Ok(MutationResult::Skipped),
        }
    }
}

pub struct ToReplayingStatefulMutator;

impl<M: Named> MappingFunctor<M> for ToReplayingStatefulMutator {
    type Output = ReplayingStatefulMutator<M>;

    fn apply(&mut self, from: M) -> Self::Output {
        ReplayingStatefulMutator::new(from)
    }
}

impl<M> Named for ReplayingStatefulMutator<M> {
    fn name(&self) -> &Cow<'static, str> {
        &self.name
    }
}
