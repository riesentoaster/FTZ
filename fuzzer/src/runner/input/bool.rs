use std::borrow::Cow;

use libafl::{
    mutators::{MutationResult, Mutator},
    Error,
};
use libafl_bolts::Named;

pub struct BoolMutator;

impl<S> Mutator<bool, S> for BoolMutator {
    fn mutate(&mut self, _state: &mut S, input: &mut bool) -> Result<MutationResult, Error> {
        *input = !*input;
        Ok(MutationResult::Mutated)
    }
}

impl Named for BoolMutator {
    fn name(&self) -> &Cow<'static, str> {
        &Cow::Borrowed("BoolMutator")
    }
}
