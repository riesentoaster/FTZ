use libafl::{events::ClientDescription, Error};
use libafl_bolts::shmem::{MmapShMem, MmapShMemProvider};

pub fn get_shmem(
    size: usize,
    client_description: &ClientDescription,
    prefix: &str,
) -> Result<MmapShMem, Error> {
    let id = format!("{}-{}", prefix, client_description.id());
    MmapShMemProvider::default()
        .new_shmem_with_id(size, &id)?
        .persist()
}
