use std::path::Path;

use base64::{prelude::BASE64_STANDARD, Engine as _};
use libafl::{
    generators::Generator,
    inputs::{BytesInput, MultipartInput},
    state::HasRand,
    Error,
};
use libafl_bolts::shmem::ShMemDescription;

use crate::smoltcp::shmem_net_device::ShmemNetworkDevice;

use super::input::ZephyrInput;

static FIXED: [&str; 7] = [
    "////////AABeAFP/CAYAAQgABgQAAQAAXgBT/8AAAgL////////AAAIB",
    "AgBeAFMxAABeAFP/CABFAAA0AABAAEAGtsDAAAICwAACATRBEJJ3NoxQAAAAAIAC//+lxwAAAgQFpgMDAQQCAAAA",
    "AgBeAFMxAABeAFP/CAYAAQgABgQAAgAAXgBT/8AAAgICAF4AUzHAAAIB",
    "AgBeAFMxAABeAFP/CABFAAAoAABAAEAGtszAAAICwAACATRBEJJ3NoxRBIh75VAQ//9jCAAA",
    "AgBeAFMxAABeAFP/CABFAABXAABAAEAGtp3AAAICwAACATRBEJJ3NoxRBIh75VAY///eZAAAekosc0dWKzowUGNUQ10lRHB8LndkaSt3bzxNeSlSQWMjJiEjZjVLI1hnUTM0XEU=",
    "AgBeAFMxAABeAFP/CABFAAAoAABAAEAGtszAAAICwAACATRBEJJ3NoyABIh8FFAR//9iqQAA",
    "AgBeAFMxAABeAFP/CABFAAAoAABAAEAGtszAAAICwAACATRBEJJ3NoyBBIh8FVAQ//9iqAAA"
];

#[allow(dead_code)]
pub struct ZephyrInteractionGenerator<'a> {
    device: ShmemNetworkDevice,
    cov_shmem_description: &'a ShMemDescription,
    zephyr_exec_path: &'a Path,
}

impl<'a> ZephyrInteractionGenerator<'a> {
    pub fn new(
        network_buf_size: usize,
        cov_shmem_description: &'a ShMemDescription,
        zephyr_exec_path: &'a Path,
    ) -> Result<Self, Error> {
        let device = ShmemNetworkDevice::new(network_buf_size)?;
        Ok(Self {
            device,
            cov_shmem_description,
            zephyr_exec_path,
        })
    }
}

impl<'a, S> Generator<ZephyrInput, S> for ZephyrInteractionGenerator<'a>
where
    S: HasRand,
{
    fn generate(&mut self, _state: &mut S) -> Result<ZephyrInput, Error> {
        let mut input = MultipartInput::new();
        // log::info!("Starting Zephyr");

        // let handle = prepare_zephyr(
        //     self.zephyr_exec_path,
        //     self.cov_shmem_description,
        //     Duration::from_secs(10),
        //     &mut self.device,
        //     |_| {},
        // )?;

        // log::info!("Prepared Zephyr");

        // let rand = state.rand_mut();
        // let message_len = rand.between(1, 100);
        // let message = (0..message_len)
        //     .map(|_| rand.between(b' '.into(), b'~'.into()) as u8)
        //     .collect::<Vec<_>>();

        // let wait = |handle: &JoinHandle<_>| match handle.is_finished() {
        //     true => WaitResult::Exit,
        //     false => {
        //         sleep(Duration::from_millis(10));
        //         WaitResult::Continue
        //     }
        // };

        // manually_connect_to_zephyr(&mut self.device, wait, &handle, &message)?;

        // log::info!("Zephyr finished");

        // input.add_part("test".to_string(), BytesInput::new(vec![1, 2, 3]));
        FIXED
            .iter()
            .map(|e| BASE64_STANDARD.decode(e).unwrap())
            .map(BytesInput::from)
            .enumerate()
            .for_each(|(i, e)| input.add_part(i.to_string(), e));
        Ok(input)
    }
}
