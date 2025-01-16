use libafl::{
    corpus::HasCurrentCorpusId,
    stages::{calibrate::UnstableEntriesMetadata, RetryCountRestartHelper, Stage},
    Error, HasMetadata, HasNamedMetadata,
};
use std::{
    borrow::Cow,
    fs::{self, OpenOptions},
    io::Write,
    thread::sleep,
    time::Duration,
};

pub struct CalibrationLogStage {
    filename: &'static str,
    lock_filename: String,
    name: Cow<'static, str>,
}

impl CalibrationLogStage {
    pub fn new(filename: &'static str) -> Self {
        Self {
            lock_filename: format!("{}.lock", filename),
            filename,
            name: Cow::Borrowed("CalibrationLogStage"),
        }
    }
}

impl<E, EM, S, Z> Stage<E, EM, S, Z> for CalibrationLogStage
where
    S: HasNamedMetadata + HasCurrentCorpusId + HasMetadata,
{
    fn should_restart(&mut self, state: &mut S) -> Result<bool, Error> {
        RetryCountRestartHelper::no_retry(state, &self.name)
    }

    fn clear_progress(&mut self, state: &mut S) -> Result<(), Error> {
        RetryCountRestartHelper::clear_progress(state, &self.name)
    }

    fn perform(
        &mut self,
        _fuzzer: &mut Z,
        _executor: &mut E,
        state: &mut S,
        _manager: &mut EM,
    ) -> Result<(), Error> {
        let unstable_entries = state
            .metadata_map()
            .get::<UnstableEntriesMetadata>()
            .ok_or(Error::illegal_state("UnstableEntriesMetadata not found"))?
            .unstable_entries();

        let lock_path = &self.lock_filename;
        while OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(lock_path)
            .is_err()
        {
            sleep(Duration::from_micros(10));
        }

        // Now we have the lock, perform the write operation
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(self.filename)
            .unwrap();
        unstable_entries.iter().for_each(|item| {
            writeln!(file, "{}", item / size_of::<i32>()).unwrap();
        });

        // Remove the lock file
        let _ = fs::remove_file(lock_path);
        Ok(())
    }
}
