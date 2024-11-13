use std::{
    io,
    path::PathBuf,
    process::{Child, Command, ExitStatus, Stdio},
    sync::mpsc::{self, Receiver, RecvError, Sender},
    thread,
};

use libafl::Error;

pub struct RunnerThread {
    start_tx: Sender<()>,
    cancel_tx: Sender<()>,
    res_rx: Receiver<Result<Option<ExitStatus>, ResultError>>,
}

#[derive(Debug)]
enum RecvErrorTime {
    Start,
    End,
}

#[allow(dead_code)]
#[derive(Debug)]
enum ResultError {
    Recv(RecvErrorTime),
    Wait(io::Error),
    Kill(io::Error),
}

impl RunnerThread {
    pub fn new(cmd: PathBuf, envs: &[(&str, &str)]) -> Self {
        let envs: Vec<_> = envs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        let (cancel_tx, cancel_rx) = mpsc::channel();
        let (start_tx, start_rx) = mpsc::channel();
        let (res_tx, res_rx) = mpsc::channel();

        thread::spawn(move || loop {
            // wait until start is sent
            if start_rx.recv().is_err() {
                let _ = res_tx.send(Err(ResultError::Recv(RecvErrorTime::Start)));
                continue;
            }

            // spawn cmd
            let child = Command::new(cmd.clone())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .envs(envs.clone())
                .spawn()
                .unwrap();

            let res = get_res(child, &cancel_rx);

            let _ = res_tx.send(res);
        });

        Self {
            start_tx,
            cancel_tx,
            res_rx,
        }
    }

    pub fn start(&mut self) -> Result<(), Error> {
        self.start_tx
            .send(())
            .map_err(|_| Error::unknown("Could not send start signal to thread running Zephyr"))
    }

    pub fn kill(&mut self) -> Result<Option<ExitStatus>, Error> {
        self.cancel_tx
            .send(())
            .map_err(|_| Error::unknown("Could not send cancel signal to thread running Zephyr"))?;

        match self.res_rx.recv() {
            Ok(e) => e.map_err(|e| {
                Error::unknown(format!("Thread running Zephyr produced error: {e:?}"))
            }),
            Err(RecvError) => Err(Error::unknown(
                "Could not receive result from thread running Zephyr",
            )),
        }
    }
}

fn get_res(mut child: Child, cancel_rx: &Receiver<()>) -> Result<Option<ExitStatus>, ResultError> {
    cancel_rx
        .recv()
        .map_err(|_| ResultError::Recv(RecvErrorTime::End))?;
    let try_res = child.try_wait().map_err(ResultError::Wait)?;
    child.kill().map_err(ResultError::Kill)?;
    child.wait().map_err(ResultError::Wait)?; // necessary to prevent zombies
    Ok(try_res)
}
