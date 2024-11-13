use std::path::PathBuf;

use libafl::{
    corpus::{Corpus, InMemoryCorpus, OnDiskCorpus},
    inputs::Input,
    Error,
};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub enum CorpusEnum<I> {
    OnDisk(OnDiskCorpus<I>),
    InMemory(InMemoryCorpus<I>),
}

impl<I> CorpusEnum<I> {
    pub fn new(path: Option<&PathBuf>) -> Result<Self, Error> {
        match path {
            Some(path) => Ok(Self::OnDisk(OnDiskCorpus::new(path)?)),
            None => Ok(Self::InMemory(InMemoryCorpus::new())),
        }
    }
}

impl<I> Corpus for CorpusEnum<I>
where
    I: Input,
{
    type Input = I;

    fn count(&self) -> usize {
        match self {
            CorpusEnum::OnDisk(e) => e.count(),
            CorpusEnum::InMemory(e) => e.count(),
        }
    }

    fn count_disabled(&self) -> usize {
        match self {
            CorpusEnum::OnDisk(e) => e.count_disabled(),
            CorpusEnum::InMemory(e) => e.count_disabled(),
        }
    }

    fn count_all(&self) -> usize {
        match self {
            CorpusEnum::OnDisk(e) => e.count_all(),
            CorpusEnum::InMemory(e) => e.count_all(),
        }
    }

    fn add(
        &mut self,
        testcase: libafl::corpus::Testcase<Self::Input>,
    ) -> Result<libafl::corpus::CorpusId, Error> {
        match self {
            CorpusEnum::OnDisk(e) => e.add(testcase),
            CorpusEnum::InMemory(e) => e.add(testcase),
        }
    }

    fn add_disabled(
        &mut self,
        testcase: libafl::corpus::Testcase<Self::Input>,
    ) -> Result<libafl::corpus::CorpusId, Error> {
        match self {
            CorpusEnum::OnDisk(e) => e.add_disabled(testcase),
            CorpusEnum::InMemory(e) => e.add_disabled(testcase),
        }
    }

    fn replace(
        &mut self,
        id: libafl::corpus::CorpusId,
        testcase: libafl::corpus::Testcase<Self::Input>,
    ) -> Result<libafl::corpus::Testcase<Self::Input>, Error> {
        match self {
            CorpusEnum::OnDisk(e) => e.replace(id, testcase),
            CorpusEnum::InMemory(e) => e.replace(id, testcase),
        }
    }

    fn remove(
        &mut self,
        id: libafl::corpus::CorpusId,
    ) -> Result<libafl::corpus::Testcase<Self::Input>, Error> {
        match self {
            CorpusEnum::OnDisk(e) => e.remove(id),
            CorpusEnum::InMemory(e) => e.remove(id),
        }
    }

    fn get(
        &self,
        id: libafl::corpus::CorpusId,
    ) -> Result<&std::cell::RefCell<libafl::corpus::Testcase<Self::Input>>, Error> {
        match self {
            CorpusEnum::OnDisk(e) => e.get(id),
            CorpusEnum::InMemory(e) => e.get(id),
        }
    }

    fn get_from_all(
        &self,
        id: libafl::corpus::CorpusId,
    ) -> Result<&std::cell::RefCell<libafl::corpus::Testcase<Self::Input>>, Error> {
        match self {
            CorpusEnum::OnDisk(e) => e.get_from_all(id),
            CorpusEnum::InMemory(e) => e.get_from_all(id),
        }
    }

    fn current(&self) -> &Option<libafl::corpus::CorpusId> {
        match self {
            CorpusEnum::OnDisk(e) => e.current(),
            CorpusEnum::InMemory(e) => e.current(),
        }
    }

    fn current_mut(&mut self) -> &mut Option<libafl::corpus::CorpusId> {
        match self {
            CorpusEnum::OnDisk(e) => e.current_mut(),
            CorpusEnum::InMemory(e) => e.current_mut(),
        }
    }

    fn next(&self, id: libafl::corpus::CorpusId) -> Option<libafl::corpus::CorpusId> {
        match self {
            CorpusEnum::OnDisk(e) => e.next(id),
            CorpusEnum::InMemory(e) => e.next(id),
        }
    }

    fn peek_free_id(&self) -> libafl::corpus::CorpusId {
        match self {
            CorpusEnum::OnDisk(e) => e.peek_free_id(),
            CorpusEnum::InMemory(e) => e.peek_free_id(),
        }
    }

    fn prev(&self, id: libafl::corpus::CorpusId) -> Option<libafl::corpus::CorpusId> {
        match self {
            CorpusEnum::OnDisk(e) => e.prev(id),
            CorpusEnum::InMemory(e) => e.prev(id),
        }
    }

    fn first(&self) -> Option<libafl::corpus::CorpusId> {
        match self {
            CorpusEnum::OnDisk(e) => e.first(),
            CorpusEnum::InMemory(e) => e.first(),
        }
    }

    fn last(&self) -> Option<libafl::corpus::CorpusId> {
        match self {
            CorpusEnum::OnDisk(e) => e.last(),
            CorpusEnum::InMemory(e) => e.last(),
        }
    }

    fn nth_from_all(&self, nth: usize) -> libafl::corpus::CorpusId {
        match self {
            CorpusEnum::OnDisk(e) => e.nth_from_all(nth),
            CorpusEnum::InMemory(e) => e.nth_from_all(nth),
        }
    }

    fn load_input_into(
        &self,
        testcase: &mut libafl::corpus::Testcase<Self::Input>,
    ) -> Result<(), Error> {
        match self {
            CorpusEnum::OnDisk(e) => e.load_input_into(testcase),
            CorpusEnum::InMemory(e) => e.load_input_into(testcase),
        }
    }

    fn store_input_from(
        &self,
        testcase: &libafl::corpus::Testcase<Self::Input>,
    ) -> Result<(), Error> {
        match self {
            CorpusEnum::OnDisk(e) => e.store_input_from(testcase),
            CorpusEnum::InMemory(e) => e.store_input_from(testcase),
        }
    }
}
