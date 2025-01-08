use etherparse::{EtherparseInput, TcpMutators};
use generator::FixedZephyrInputPartGenerator;
use libafl::{
    corpus::{CorpusId, Testcase},
    generators::RandBytesGenerator,
    inputs::{BytesInput, Input, MultipartInput},
    mutators::{havoc_mutations, HavocMutationsType},
    nonzero, HasMetadata,
};

use libafl_bolts::{
    map_tuple_list_type, merge_tuple_list_type,
    tuples::{tuple_list, tuple_list_type, Map, Merge},
};
use serde::Serialize;

mod appending;
mod bool;
mod etherparse;
mod generator;
mod parsed;
mod stateful;
pub use stateful::{ReplayingStatefulInput, ToReplayingStatefulMutator};

use crate::packets::outgoing_tcp_packets;

type HavocStatefulInput = ReplayingStatefulInput<BytesInput>;
type HavocMultipartInput = MultipartInput<BytesInput>;
type ParsedStatefulInput = ReplayingStatefulInput<ParsedZephyrInput>;
type ParsedMultipartInput = MultipartInput<ParsedZephyrInput>;
type EtherparseStatefulInput = ReplayingStatefulInput<EtherparseInput>;

pub type ZephyrInputType = EtherparseStatefulInput;

pub use {
    appending::AppendingMutator, generator::FixedZephyrInputGenerator, parsed::ParsedZephyrInput,
};

pub trait ZephyrInputPart: Sized
where
    Vec<u8>: From<Self>,
{
    type Mutators;
    type Generator;
    fn mutators() -> Self::Mutators;
    fn generator() -> Self::Generator;
}

impl ZephyrInputPart for BytesInput {
    type Mutators = HavocMutationsType;
    type Generator = RandBytesGenerator;

    fn mutators() -> Self::Mutators {
        havoc_mutations()
    }
    fn generator() -> Self::Generator {
        RandBytesGenerator::new(nonzero!(50))
    }
}

impl ZephyrInputPart for ParsedZephyrInput {
    type Mutators = ();
    type Generator = FixedZephyrInputPartGenerator<Self>;

    fn mutators() -> Self::Mutators {}
    fn generator() -> Self::Generator {
        FixedZephyrInputPartGenerator::new(outgoing_tcp_packets(), true)
    }
}

impl ZephyrInputPart for EtherparseInput {
    type Mutators = TcpMutators;
    type Generator = FixedZephyrInputPartGenerator<Self>;

    fn mutators() -> Self::Mutators {
        EtherparseInput::mutators()
    }
    fn generator() -> Self::Generator {
        FixedZephyrInputPartGenerator::new(outgoing_tcp_packets(), true)
    }
}

pub trait ZephyrInput<I>
where
    Vec<u8>: From<I>,
    I: ZephyrInputPart,
{
    type Mutators;
    fn mutators() -> Self::Mutators;
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
    type Mutators = I::Mutators;

    fn mutators() -> Self::Mutators {
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
    <I::Mutators as Map<ToReplayingStatefulMutator>>::MapResult:
        Merge<tuple_list_type!(AppendingMutator<I::Generator>)>,
    Vec<u8>: From<I>,
{
    type Mutators = merge_tuple_list_type!(
        map_tuple_list_type!(I::Mutators, ToReplayingStatefulMutator),
        tuple_list_type!(AppendingMutator<I::Generator>)
    );

    fn mutators() -> Self::Mutators {
        I::mutators()
            .map(ToReplayingStatefulMutator)
            .merge(tuple_list!(AppendingMutator::new(I::generator())))
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
        runner::input::{
            etherparse::EtherparseInput, generator::FixedZephyrInputPartGenerator,
            AppendingMutator, EtherparseStatefulInput, FixedZephyrInputGenerator, ZephyrInput,
            ZephyrInputPart,
        },
    };

    #[test]
    fn generate_etherparse() {
        type I = EtherparseStatefulInput;
        type II = EtherparseInput;
        let mut generator = II::generator();
        let g: &mut dyn Generator<II, _> = &mut generator;
        let _i: II = g.generate(&mut StdRand::new()).unwrap();

        let mut generator = FixedZephyrInputGenerator::new(outgoing_tcp_packets(), true);
        let g: &mut dyn Generator<I, _> = &mut generator;
        let _i: I = g.generate(&mut StdRand::new()).unwrap();
    }

    #[test]
    fn appending_mutator_etherparse() {
        let input = EtherparseStatefulInput::parse(&outgoing_tcp_packets());
        fn take_zephyr_input<I: ZephyrInput<II>, II>(input: I) -> I
        where
            Vec<u8>: From<II>,
            II: ZephyrInputPart,
        {
            input
        }
        let _input = take_zephyr_input(input);
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
