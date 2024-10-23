use std::{
    ffi::CStr,
    process::{Command, ExitStatus},
    sync::mpsc::Sender,
    thread::{self, sleep},
    time::Duration,
};

use wait_timeout::ChildExt as _;

/// Starts zephyr in a different thread.
///
/// `cmd` and `args` are typically set to the zephyr executable path and an empty [`Vec`], but can be changed to use debugging tools such as `gdb`.
///
/// `shmem_path` and `shmem_len` are passed as environemnt variables according to the documentation in the global README of this project.
///
/// `tx` receives the exit status of the command once it finishes or is killed.
///
/// The thread waits for `startup_delay` until the command starting zephyr is run. The command is killed after `exec_timeout`.
pub fn start_zephyr(
    cmd: &str,
    args: Vec<&str>,
    shmem_path: &[u8; 20],
    shmem_len: usize,
    tx: Sender<Option<ExitStatus>>,
    startup_delay: Duration,
    exec_timeout: Duration,
) {
    let cmd = cmd.to_string();
    let args = args.iter().map(|e| e.to_string()).collect::<Vec<_>>();
    let shmem_path = shmem_path.to_owned();

    thread::spawn(move || {
        sleep(startup_delay);
        let result = Command::new(cmd.to_string().clone())
            .args(args)
            .env(
                "SHMEM_ETH_INTERFACE_NAME",
                CStr::from_bytes_until_nul(&shmem_path)
                    .unwrap()
                    .to_str()
                    .unwrap(),
            )
            .env("SHMEM_ETH_INTERFACE_SIZE", shmem_len.to_string())
            .spawn()
            .unwrap()
            .wait_timeout(exec_timeout)
            .unwrap();
        tx.send(result).unwrap();
    });
}
