use libafl::Error;
use libafl_bolts::shmem::{MmapShMem, MmapShMemProvider};

pub fn get_shmem(size: usize, id: usize, prefix: &str) -> Result<MmapShMem, Error> {
    let id = format!("{}-{}", prefix, id);
    MmapShMemProvider::default()
        .new_shmem_with_id(size, &id)?
        .persist()
}
