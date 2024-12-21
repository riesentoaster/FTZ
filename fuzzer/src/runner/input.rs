use libafl::{
    corpus::{CorpusId, Testcase},
    inputs::{BytesInput, Input, MultipartInput},
    HasMetadata,
};
use serde::Serialize;

pub type ZephyrInput = MultipartInput<BytesInput>;

#[derive(Serialize)]
struct DumpFormat<'a, I, M> {
    input: &'a I,
    metadata: &'a M,
}

pub fn serialize_input<S>(testcase: &Testcase<ZephyrInput>, _state: &S) -> Vec<u8> {
    // let testcase = state.current_testcase().unwrap();
    let metadata = testcase.metadata_map();
    let input = testcase.input().as_ref().unwrap();

    serde_json::to_string_pretty(&DumpFormat { input, metadata })
        .unwrap()
        .as_bytes()
        .to_vec()
}

pub fn generate_filename(testcase: &Testcase<ZephyrInput>, id: &CorpusId) -> String {
    format!(
        "{}-{}.json",
        id,
        testcase.input().as_ref().unwrap().generate_name(Some(*id))
    )
}
