use std::borrow::Cow;

use libafl::{
    generators::Generator,
    inputs::MultipartInput,
    mutators::{MutationResult, Mutator},
    Error,
};
use libafl_bolts::{tuples::MappingFunctor, Named};

use super::{list::ListInput, stateful::ReplayingStatefulInput};

pub struct AppendingMutator<G> {
    generator: G,
}

impl<G> AppendingMutator<G> {
    pub fn new(generator: G) -> Self {
        Self { generator }
    }
}

impl<G, I, S> Mutator<ReplayingStatefulInput<I>, S> for AppendingMutator<G>
where
    G: Generator<I, S>,
{
    fn mutate(
        &mut self,
        state: &mut S,
        input: &mut ReplayingStatefulInput<I>,
    ) -> Result<MutationResult, Error> {
        let new_part = self.generator.generate(state).unwrap();
        input.parts_mut().push(new_part);
        Ok(MutationResult::Mutated)
    }
}

impl<G, I, S> Mutator<ListInput<I>, S> for AppendingMutator<G>
where
    G: Generator<I, S>,
{
    fn mutate(&mut self, state: &mut S, input: &mut ListInput<I>) -> Result<MutationResult, Error> {
        let new_part = self.generator.generate(state).unwrap();
        input.parts_mut().push(new_part);
        Ok(MutationResult::Mutated)
    }
}

impl<G, I, S> Mutator<MultipartInput<I>, S> for AppendingMutator<G>
where
    G: Generator<I, S>,
{
    fn mutate(
        &mut self,
        state: &mut S,
        input: &mut MultipartInput<I>,
    ) -> Result<MutationResult, Error> {
        let new_part = self.generator.generate(state)?;
        let len = input.names().len();
        input.add_part(format!("{len}"), new_part);
        Ok(MutationResult::Mutated)
    }
}

impl<G> Named for AppendingMutator<G> {
    fn name(&self) -> &Cow<'static, str> {
        &Cow::Borrowed("AppendingMutator")
    }
}

pub struct ToAppendingMutatorWrapper;

impl<G> MappingFunctor<G> for ToAppendingMutatorWrapper {
    type Output = AppendingMutator<G>;

    fn apply(&mut self, from: G) -> Self::Output {
        AppendingMutator::new(from)
    }
}

#[cfg(test)]
mod tests {
    use libafl::{
        inputs::ValueInput,
        mutators::{numeric::IncMutator, MutationResult, Mutator},
        state::NopState,
    };
    use libafl_bolts::tuples::tuple_list;

    use super::{AppendingMutator, ReplayingStatefulInput};

    #[test]
    fn wrap_simple_mutator() {
        let inner = tuple_list!(IncMutator);
        let mut muts = ReplayingStatefulInput::<ValueInput<i32>>::map_mutators(inner);
        let mut state: NopState<ReplayingStatefulInput<ValueInput<i32>>> = NopState::new();
        let mut input_raw = [1, 2, 3];
        let mut input: ReplayingStatefulInput<_> = input_raw
            .iter_mut()
            .map(ValueInput::new)
            .collect::<Vec<_>>()
            .into();
        assert_eq!(
            MutationResult::Mutated,
            muts.0.mutate(&mut state, &mut input).unwrap()
        );
        assert_eq!([1, 2, 4], input_raw);
    }

    #[test]
    fn appending_mutator() {
        let mut state: NopState<ReplayingStatefulInput<i32>> = NopState::new();
        let mut input = ReplayingStatefulInput::new(vec![]);
        let generator = 0..;
        let mut mutator = AppendingMutator::new(generator);
        assert_eq!(
            MutationResult::Mutated,
            mutator.mutate(&mut state, &mut input).unwrap()
        );
        assert_eq!(input.parts(), vec![0]);

        assert_eq!(
            MutationResult::Mutated,
            mutator.mutate(&mut state, &mut input).unwrap()
        );
        assert_eq!(input.parts(), vec![0, 1]);

        assert_eq!(
            MutationResult::Mutated,
            mutator.mutate(&mut state, &mut input).unwrap()
        );
        assert_eq!(input.parts(), vec![0, 1, 2]);
    }
}
