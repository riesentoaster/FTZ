use super::input::ZephyrInput;
use base64::{prelude::BASE64_STANDARD, Engine as _};
use libafl::{
    generators::Generator,
    inputs::{BytesInput, MultipartInput},
    state::HasRand,
    Error,
};

static FIXED: [&str; 7] = [
    "////////AABeAFP/CAYAAQgABgQAAQAAXgBT/8AAAgL////////AAAIB",
    "AgBeAFMxAABeAFP/CABFAAA0AABAAEAGtsDAAAICwAACATRBEJJ3NoxQAAAAAIAC//+lxwAAAgQFpgMDAQQCAAAA",
    "AgBeAFMxAABeAFP/CAYAAQgABgQAAgAAXgBT/8AAAgICAF4AUzHAAAIB",
    "AgBeAFMxAABeAFP/CABFAAAoAABAAEAGtszAAAICwAACATRBEJJ3NoxRBIh75VAQ//9jCAAA",
    "AgBeAFMxAABeAFP/CABFAABXAABAAEAGtp3AAAICwAACATRBEJJ3NoxRBIh75VAY///eZAAAekosc0dWKzowUGNUQ10lRHB8LndkaSt3bzxNeSlSQWMjJiEjZjVLI1hnUTM0XEU=",
    "AgBeAFMxAABeAFP/CABFAAAoAABAAEAGtszAAAICwAACATRBEJJ3NoyABIh8FFAR//9iqQAA",
    "AgBeAFMxAABeAFP/CABFAAAoAABAAEAGtszAAAICwAACATRBEJJ3NoyBBIh8FVAQ//9iqAAA"
];

pub struct ZephyrInteractionGenerator;

impl ZephyrInteractionGenerator {
    pub fn new() -> Self {
        Self
    }
}

impl<S> Generator<ZephyrInput, S> for ZephyrInteractionGenerator
where
    S: HasRand,
{
    fn generate(&mut self, _state: &mut S) -> Result<ZephyrInput, Error> {
        let mut input = MultipartInput::new();
        FIXED
            .iter()
            .map(|e| BASE64_STANDARD.decode(e).unwrap())
            .map(BytesInput::from)
            .enumerate()
            .for_each(|(i, e)| input.add_part(i.to_string(), e));
        Ok(input)
    }
}
