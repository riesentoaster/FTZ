use std::{
    process::{Command, ExitStatus},
    sync::mpsc::Sender,
    thread::{self, sleep},
    time::Duration,
};

use wait_timeout::ChildExt as _;

/// Starts zephyr in a different thread.
///
/// `cmd`, `args` and `envs` are passed to [`Command`].
///
/// `tx` receives the exit status of the command once it finishes or is killed.
///
/// The thread waits for `startup_delay` until the command starting zephyr is run. The command is killed after `exec_timeout`.
#[allow(clippy::too_many_arguments)]
pub fn start_zephyr(
    cmd: &str,
    args: &[&str],
    envs: &[(&str, &str)],
    tx: Sender<Option<ExitStatus>>,
    startup_delay: Duration,
    exec_timeout: Duration,
) {
    let cmd = cmd.to_string();
    let envs = envs
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect::<Vec<_>>();
    let args = args.iter().map(|e| e.to_string()).collect::<Vec<_>>();

    thread::spawn(move || {
        sleep(startup_delay);
        let result = Command::new(cmd.to_string().clone())
            .envs(envs)
            .args(args)
            .spawn()
            .unwrap()
            .wait_timeout(exec_timeout)
            .unwrap();
        tx.send(result).unwrap();
    });
}
