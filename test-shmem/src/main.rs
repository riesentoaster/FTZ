use libafl::Error;
use libafl_bolts::shmem::{MmapShMemProvider, ShMemProvider, StdShMemProvider};

fn main() {
    main2().unwrap()
}
fn main2() -> Result<(), Error> {
    let mut provider = MmapShMemProvider::new().unwrap();
    let mut shmems = vec![];
    let mut index = 0;
    loop {
        index += 1;
        match provider
            .new_shmem_with_id(10, format!("shmem-{index}").as_bytes())
            .and_then(|shmem| shmem.persist())
        {
            Ok(shmem) => {
                shmems.push(shmem);
            }
            Err(e) => {
                println!("{e}");
                break;
            }
        }
    }

    println!("{}", shmems.len());
    drop(shmems);
    let mut provider = StdShMemProvider::new().unwrap();
    let mut shmems = vec![];
    loop {
        index += 1;
        match provider.new_shmem(10) {
            Ok(shmem) => {
                shmems.push(shmem);
            }
            Err(e) => {
                println!("{e}");
                break;
            }
        }
    }

    println!("{}", shmems.len());
    drop(shmems);
    MmapShMemProvider::new()?.new_shmem(10)?.persist()?;
    Ok(())
}
