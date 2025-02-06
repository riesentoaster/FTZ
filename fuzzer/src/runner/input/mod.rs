use crate::runner::{
    generator::fixed::FixedZephyrInputGenerator,
    input::{
        appending::ToAppendingMutatorWrapper,
        stateful::{ReplayingStatefulInput, ToReplayingStatefulMutator},
    },
};
use etherparse::TcpMutators;
use libafl::{
    corpus::{CorpusId, Testcase},
    generators::RandBytesGenerator,
    inputs::{BytesInput, Input, MultipartInput},
    mutators::{havoc_mutations, HavocMutationsType},
    nonzero, HasMetadata,
};

use libafl_bolts::{
    map_tuple_list_type,
    tuples::{tuple_list, tuple_list_type, Map, Merge},
};
use serde::Serialize;

pub mod appending;
pub mod bool;
pub mod etherparse;
pub mod parsed;
pub mod stateful;

#[allow(dead_code)]
type HavocStatefulInput = ReplayingStatefulInput<BytesInput>;
#[allow(dead_code)]
type HavocMultipartInput = MultipartInput<BytesInput>;
#[allow(dead_code)]
type ParsedStatefulInput = ReplayingStatefulInput<ParsedZephyrInput>;
#[allow(dead_code)]
type ParsedMultipartInput = MultipartInput<ParsedZephyrInput>;
#[allow(dead_code)]
type EtherparseStatefulInput = ReplayingStatefulInput<EtherparseInput>;

pub type ZephyrInputType = EtherparseStatefulInput;

use super::feedback::input_len::HasLen;

pub use {etherparse::EtherparseInput, parsed::ParsedZephyrInput};

pub trait ZephyrInputPart: Sized
where
    Vec<u8>: From<Self>,
{
    type Mutators;
    type Generators;
    fn mutators() -> Self::Mutators;
    fn generator() -> Self::Generators;
}

impl ZephyrInputPart for BytesInput {
    type Mutators = HavocMutationsType;
    type Generators = tuple_list_type!(RandBytesGenerator);

    fn mutators() -> Self::Mutators {
        havoc_mutations()
    }
    fn generator() -> Self::Generators {
        tuple_list!(RandBytesGenerator::new(nonzero!(50)))
    }
}

impl ZephyrInputPart for ParsedZephyrInput {
    type Mutators = ();
    type Generators = tuple_list_type!();

    fn mutators() -> Self::Mutators {}
    fn generator() -> Self::Generators {
        tuple_list!()
    }
}

impl ZephyrInputPart for EtherparseInput {
    type Mutators = TcpMutators;
    type Generators = tuple_list_type!();

    fn mutators() -> Self::Mutators {
        EtherparseInput::mutators()
    }

    fn generator() -> Self::Generators {
        tuple_list!()
    }
}

pub trait ZephyrInput<I>: HasLen
where
    Vec<u8>: From<I>,
    I: ZephyrInputPart,
{
    type NonAppendingMutators;
    fn non_appending_mutators() -> Self::NonAppendingMutators;
    fn to_packets(&self) -> Vec<Vec<u8>>;
    fn parse(input: &[Vec<u8>]) -> Self;
    fn fixed_generator(fixed: Vec<Vec<u8>>, restart: bool) -> FixedZephyrInputGenerator<Self>
    where
        Self: Sized,
    {
        FixedZephyrInputGenerator::new(fixed, restart)
    }
}

impl<I> ZephyrInput<I> for MultipartInput<I>
where
    I: ZephyrInputPart + TryFrom<Vec<u8>> + Clone,
    Vec<u8>: From<I>,
{
    type NonAppendingMutators = I::Mutators;

    fn non_appending_mutators() -> Self::NonAppendingMutators {
        I::mutators()
    }

    fn parse(input: &[Vec<u8>]) -> Self {
        input
            .iter()
            .enumerate()
            .map(|(i, e)| {
                (
                    i.to_string(),
                    I::try_from(e.clone())
                        .map_err(|_| "Could not parse to ZephyrInputPart")
                        .unwrap(),
                )
            })
            .into()
    }

    fn to_packets(&self) -> Vec<Vec<u8>> {
        self.parts().iter().cloned().map(<Vec<u8>>::from).collect()
    }
}

impl<I> ZephyrInput<I> for ReplayingStatefulInput<I>
where
    I: ZephyrInputPart + for<'a> TryFrom<&'a [u8]> + Clone,
    I::Mutators: Map<ToReplayingStatefulMutator>,
    I::Generators: Map<ToAppendingMutatorWrapper>,
    map_tuple_list_type!(I::Mutators, ToReplayingStatefulMutator):
        Merge<map_tuple_list_type!(I::Generators, ToAppendingMutatorWrapper)>,
    Vec<u8>: From<I>,
{
    type NonAppendingMutators = map_tuple_list_type!(I::Mutators, ToReplayingStatefulMutator);

    fn non_appending_mutators() -> Self::NonAppendingMutators {
        I::mutators().map(ToReplayingStatefulMutator)
    }

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

#[derive(Serialize)]
struct DumpFormat<'a, I, M> {
    input: &'a I,
    metadata: &'a M,
}

pub fn serialize_input<S, I: Serialize>(testcase: &Testcase<I>, _state: &S) -> Vec<u8> {
    // let testcase = state.current_testcase().unwrap();
    let metadata = testcase.metadata_map();
    let input = testcase.input().as_ref().unwrap();

    serde_json::to_string_pretty(&DumpFormat { input, metadata })
        .unwrap()
        .as_bytes()
        .to_vec()
}

pub fn generate_filename<I: Input>(testcase: &Testcase<I>, id: &CorpusId) -> String {
    format!(
        "{}-{}.json",
        id,
        testcase.input().as_ref().unwrap().generate_name(Some(*id))
    )
}

#[cfg(test)]
mod tests {
    use libafl::{generators::Generator, mutators::Mutator, state::NopState};
    use libafl_bolts::rands::StdRand;

    use crate::{
        packets::outgoing_tcp_packets,
        runner::{
            generator::fixed::FixedZephyrInputPartGenerator,
            input::{
                appending::AppendingMutator, etherparse::EtherparseInput, EtherparseStatefulInput,
                FixedZephyrInputGenerator, ReplayingStatefulInput, ZephyrInput, ZephyrInputPart,
            },
        },
    };

    #[test]
    fn generate_etherparse() {
        type II = EtherparseInput;
        type I = ReplayingStatefulInput<II>;

        let mut generator = FixedZephyrInputGenerator::new(outgoing_tcp_packets(), true);
        let g: &mut dyn Generator<I, _> = &mut generator;
        let _i: I = g.generate(&mut StdRand::new()).unwrap();
    }

    #[test]
    fn appending_mutator_etherparse() {
        let input = ReplayingStatefulInput::<EtherparseInput>::parse(&outgoing_tcp_packets());
        fn take_zephyr_input<I: ZephyrInput<II>, II>(input: I) -> I
        where
            Vec<u8>: From<II>,
            II: ZephyrInputPart,
        {
            input
        }
        let _input = take_zephyr_input::<_, EtherparseInput>(input);
        let mut generator = FixedZephyrInputGenerator::new(outgoing_tcp_packets(), true);
        let mut state: NopState<EtherparseStatefulInput> = NopState::new();
        let input = generator.generate(&mut state).unwrap();
        let mut input: EtherparseStatefulInput = take_zephyr_input(input);

        let mut inner_generator = FixedZephyrInputPartGenerator::new(outgoing_tcp_packets(), true);
        let _input_inner: EtherparseInput = inner_generator.generate(&mut state).unwrap();

        let mut mutator = AppendingMutator::new(inner_generator);
        mutator.mutate(&mut state, &mut input).unwrap();
        println!("{:?}", input);
    }
}
