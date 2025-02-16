use core::hash::{Hash, Hasher};
use std::{borrow::Cow, num::NonZero};

use libafl::{
    corpus::CorpusId,
    inputs::Input,
    mutators::{MutationResult, Mutator},
    state::HasRand,
    Error,
};
use libafl_bolts::{
    generic_hash_std,
    rands::Rand as _,
    tuples::{Map, MappingFunctor},
    Named,
};
use serde::{Deserialize, Serialize};

use crate::runner::feedback::input_len::HasLen;

use super::{EtherparseInput, ZephyrInput, ZephyrInputPart};

pub type ListZephyrInputType = ListInput<EtherparseInput>;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ListInput<I> {
    parts: Vec<I>,
}

impl<I> HasLen for ListInput<I> {
    fn len(&self) -> usize {
        self.parts.len()
    }
}

impl<I: Input + Hash> Input for ListInput<I> {
    fn generate_name(&self, _id: Option<CorpusId>) -> String {
        format!(
            "ListInput<{},{}>",
            generic_hash_std(&self.parts),
            self.parts.len()
        )
    }
}

impl<I: Hash> Hash for ListInput<I> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.parts.hash(state);
    }
}

impl<I> From<Vec<I>> for ListInput<I> {
    fn from(value: Vec<I>) -> Self {
        Self::new(value)
    }
}

impl<I> ListInput<I> {
    pub fn new(parts: Vec<I>) -> Self {
        Self { parts }
    }

    pub fn map_to_mutate_on_last<M: Map<ToLastEntryListMutator>>(
        inner: M,
    ) -> <M as Map<ToLastEntryListMutator>>::MapResult {
        inner.map(ToLastEntryListMutator)
    }

    pub fn map_to_mutate_on_random<M: Map<ToRandomEntryListMutator>>(
        inner: M,
    ) -> <M as Map<ToRandomEntryListMutator>>::MapResult {
        inner.map(ToRandomEntryListMutator)
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

impl<I: libafl_bolts::HasLen> libafl_bolts::HasLen for ListInput<I> {
    fn len(&self) -> usize {
        self.parts
            .last()
            .map(libafl_bolts::HasLen::len)
            .unwrap_or(0)
    }
}

pub struct LastEntryListMutator<M> {
    inner: M,
    name: Cow<'static, str>,
}

impl<M: Named> LastEntryListMutator<M> {
    pub fn new(inner: M) -> Self {
        let name = Cow::Owned(format!("LastEntryListMutator<{}>", inner.name()));
        Self { inner, name }
    }
}

impl<I, S, M> Mutator<ListInput<I>, S> for LastEntryListMutator<M>
where
    M: Mutator<I, S>,
{
    fn mutate(&mut self, state: &mut S, input: &mut ListInput<I>) -> Result<MutationResult, Error> {
        match input.parts.last_mut() {
            Some(inner_input) => self.inner.mutate(state, inner_input),
            None => Ok(MutationResult::Skipped),
        }
    }
}

pub struct ToLastEntryListMutator;

impl<M: Named> MappingFunctor<M> for ToLastEntryListMutator {
    type Output = LastEntryListMutator<M>;

    fn apply(&mut self, from: M) -> Self::Output {
        LastEntryListMutator::new(from)
    }
}

impl<M> Named for LastEntryListMutator<M> {
    fn name(&self) -> &Cow<'static, str> {
        &self.name
    }
}

pub struct RandomEntryListMutator<M> {
    inner: M,
    name: Cow<'static, str>,
}

impl<M: Named> RandomEntryListMutator<M> {
    pub fn new(inner: M) -> Self {
        let name = Cow::Owned(format!("RandomEntryListMutator<{}>", inner.name()));
        Self { inner, name }
    }
}

impl<I, S, M> Mutator<ListInput<I>, S> for RandomEntryListMutator<M>
where
    M: Mutator<I, S>,
    S: HasRand,
{
    fn mutate(&mut self, state: &mut S, input: &mut ListInput<I>) -> Result<MutationResult, Error> {
        let rand = state.rand_mut();
        match input.parts.len() {
            0 => Ok(MutationResult::Skipped),
            len => {
                let index = rand.below(unsafe { NonZero::new_unchecked(len) });
                self.inner.mutate(state, &mut input.parts[index])
            }
        }
    }
}

pub struct ToRandomEntryListMutator;

impl<M: Named> MappingFunctor<M> for ToRandomEntryListMutator {
    type Output = RandomEntryListMutator<M>;

    fn apply(&mut self, from: M) -> Self::Output {
        RandomEntryListMutator::new(from)
    }
}

impl<M> Named for RandomEntryListMutator<M> {
    fn name(&self) -> &Cow<'static, str> {
        &self.name
    }
}

impl<I> ZephyrInput<I> for ListInput<I>
where
    I: ZephyrInputPart + for<'a> TryFrom<&'a [u8]> + Clone,
    Vec<u8>: From<I>,
{
    fn parse(input: &[Vec<u8>]) -> Self {
        input
            .iter()
            .map(|e| {
                I::try_from(e)
                    .map_err(|_e| "Could not parse to ZephyrInputPart")
                    .unwrap()
            })
            .collect::<Vec<_>>()
            .into()
    }

    fn to_packets(&self) -> Vec<Vec<u8>> {
        self.parts()
            .iter()
            .cloned()
            .map(<Vec<u8>>::from)
            .collect::<Vec<_>>()
    }
}
