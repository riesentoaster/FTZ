use base64::alphabet::URL_SAFE;
use libafl::Error;
use libafl_bolts::{
    rands::Rand,
    shmem::{MmapShMem, MmapShMemProvider},
};
use std::sync::LazyLock;

static SAFE_CHARS: LazyLock<Vec<u8>> = LazyLock::new(|| URL_SAFE.as_str().as_bytes().to_vec());

fn get_name<R: Rand>(rand: &mut R) -> Vec<u8> {
    let safe_chars = &*SAFE_CHARS;
    (0..18).map(|_| *rand.choose(safe_chars).unwrap()).collect()
}

pub fn get_shmem<R: Rand>(size: usize, rand: &mut R) -> Result<MmapShMem, Error> {
    let id = get_name(rand);
    MmapShMemProvider::default()
        .new_shmem_with_id(size, &id)?
        .persist()
}
