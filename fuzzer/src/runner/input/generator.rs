use std::marker::PhantomData;

use libafl::{generators::Generator, Error};

use super::{ZephyrInput, ZephyrInputPart};

pub struct FixedZephyrInputPartGenerator<I> {
    fixed: Vec<Vec<u8>>,
    offset: usize,
    phantom: PhantomData<I>,
    restart: bool,
}

impl<I> FixedZephyrInputPartGenerator<I> {
    pub fn new(fixed: Vec<Vec<u8>>, restart: bool) -> Self {
        Self {
            fixed,
            offset: 0,
            restart,
            phantom: PhantomData,
        }
    }
}

impl<I, S> Generator<I, S> for FixedZephyrInputPartGenerator<I>
where
    I: ZephyrInputPart + From<Vec<u8>>,
    Vec<u8>: From<I>,
{
    fn generate(&mut self, _state: &mut S) -> Result<I, libafl::Error> {
        if !self.restart && self.offset >= self.fixed.len() {
            return Err(Error::illegal_state(
                "Attempting to generate more values than provided",
            ));
        }
        let max = self.offset % self.fixed.len();
        let res = self.fixed[max].clone().into();
        self.offset += 1;
        Ok(res)
    }
}

pub struct FixedZephyrInputGenerator<I> {
    fixed: Vec<Vec<u8>>,
    offset: usize,
    phantom: PhantomData<I>,
    restart: bool,
}

impl<I> FixedZephyrInputGenerator<I> {
    pub fn new(fixed: Vec<Vec<u8>>, restart: bool) -> Self {
        Self {
            fixed,
            offset: 0,
            restart,
            phantom: PhantomData,
        }
    }
}

impl<I, S, Z> Generator<Z, S> for FixedZephyrInputGenerator<I>
where
    Z: ZephyrInput<I>,
    I: ZephyrInputPart,
    Vec<u8>: From<I>,
{
    fn generate(&mut self, _state: &mut S) -> Result<Z, libafl::Error> {
        if !self.restart && self.offset >= self.fixed.len() {
            return Err(Error::illegal_state(
                "Attempting to generate more values than provided",
            ));
        }
        let max = self.offset % self.fixed.len();
        let res = Z::parse(&self.fixed[0..max]);
        self.offset += 1;
        Ok(res)
    }
}
