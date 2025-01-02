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
mod generator;
mod parsed;
mod stateful;
use parsed::ParsedZephyrInput;
use stateful::{ReplayingStatefulInput, ToReplayingStatefulMutator};

use crate::packets::outgoing_tcp_packets;

type HavocStatefulInput = ReplayingStatefulInput<BytesInput>;
type HavocMultipartInput = MultipartInput<BytesInput>;
type ParsedStatefulInput = ReplayingStatefulInput<ParsedZephyrInput>;
type ParsedMultipartInput = MultipartInput<ParsedZephyrInput>;

pub type ZephyrInputType = HavocStatefulInput;

pub use {appending::AppendingMutator, generator::FixedZephyrInputGenerator};

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

pub trait ZephyrInput<I>
where
    Vec<u8>: From<I>,
    I: ZephyrInputPart,
{
    type Mutators;
    fn mutators() -> Self::Mutators;
    fn to_packets(&self) -> Vec<Vec<u8>>;
    fn parse(input: &[Vec<u8>]) -> Self;
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
