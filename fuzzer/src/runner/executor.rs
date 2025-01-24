use std::{
    fmt::Debug,
    fs::OpenOptions,
    io::Write as _,
    marker::PhantomData,
    os::unix::process::ExitStatusExt as _,
    path::PathBuf,
    process::{Command, Stdio},
    thread::sleep,
    time::Instant,
};

use libafl::{
    executors::{Executor, ExitKind, HasObservers},
    observers::ObserversTuple,
    state::HasExecutions,
    Error,
};
use libafl_bolts::{
    shmem::ShMemDescription,
    tuples::{Handle, MatchName, MatchNameRef, RefIndexable},
};

use crate::{
    direction::Source,
    layers::data_link::parse_eth,
    runner::{get_path, INTER_SEND_WAIT},
};

use crate::smoltcp::shmem_net_device::ShmemNetworkDevice;

use super::{
    input::{ZephyrInput, ZephyrInputPart},
    observer::packet::PacketObserver,
};

pub struct ZepyhrExecutor<'a, S, OT, II> {
    observers: &'a mut OT,
    packet_observer: Handle<PacketObserver>,
    device: ShmemNetworkDevice,
    envs: Vec<(String, String)>,
    zephyr_exec_path: PathBuf,
    zephyr_out_path: Option<PathBuf>,
    phantom: PhantomData<(S, II)>,
}

impl<'a, S, OT, II> ZepyhrExecutor<'a, S, OT, II> {
    pub fn new(
        observers: &'a mut OT,
        packet_observer: Handle<PacketObserver>,
        cov_shmem_desc: &ShMemDescription,
        zephyr_exec_path: PathBuf,
        zephyr_out_path: Option<PathBuf>,
        network_buf_size: usize,
        id: usize,
    ) -> Result<Self, Error> {
        let device = ShmemNetworkDevice::new(network_buf_size, id)?;
        let net_shmem_desc = device.get_shmem_description();

        let envs = ([
            (&"SHMEM_ETH_INTERFACE_SIZE", &net_shmem_desc.size),
            (&"SHMEM_ETH_INTERFACE_NAME", &get_path(&net_shmem_desc)?),
            (&"SHMEM_COVERAGE_SIZE", &cov_shmem_desc.size),
            (&"SHMEM_COVERAGE_NAME", &get_path(cov_shmem_desc)?),
        ] as [(&dyn ToString, &dyn ToString); 4])
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();

        Ok(Self {
            observers,
            packet_observer,
            device,
            envs,
            zephyr_exec_path,
            zephyr_out_path,
            phantom: PhantomData,
        })
    }
}

impl<EM, Z, S, OT, I, II> Executor<EM, I, S, Z> for ZepyhrExecutor<'_, S, OT, II>
where
    S: HasExecutions,
    OT: Debug + MatchName + MatchNameRef + ObserversTuple<I, S>,
    I: ZephyrInput<II>,
    II: ZephyrInputPart,
    Vec<u8>: From<II>,
{
    fn run_target(
        &mut self,
        _fuzzer: &mut Z,
        state: &mut S,
        _mgr: &mut EM,
        input: &I,
    ) -> Result<ExitKind, Error> {
        log::debug!("Starting input run #{} on target", state.executions());
        *state.executions_mut() += 1;

        self.observers.pre_exec_child_all(state, input)?;
        let packet_observer = self
            .observers
            .get_mut(&self.packet_observer)
            .ok_or(Error::illegal_argument(
            "Could not retrieve PacketObserver, make sure you pass it to the executor in the OT.",
        ))?;

        log::debug!("Preparing Zephyr");

        self.device.reset();

        let (stdout, stderr) = self
            .zephyr_out_path
            .as_ref()
            .map(|path| {
                let mut file = OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(path)
                    .expect("Failed to open file");
                writeln!(file, "----------------------------------------").unwrap();
                (
                    Stdio::from(file.try_clone().expect("Could not clone zephyr outfile")),
                    Stdio::from(file),
                )
            })
            .unwrap_or((Stdio::null(), Stdio::null()));

        let mut child = Command::new(self.zephyr_exec_path.clone())
            .stdout(stdout)
            .stderr(stderr)
            .envs(self.envs.clone())
            .spawn()
            .map_err(|e| Error::unknown(format!("Could not start command: {e:?}")))?;

        self.device.init_zephyr(|p| packet_observer.add_packet(p))?;

        let packets = input.to_packets();

        log::debug!("Started Zephyr, now sending {} packets", packets.len());

        for e in packets {
            self.device.send(&e);
            packet_observer.add_packet(Source::Client(e));
            let mut last_packet_time = Instant::now();
            while last_packet_time.elapsed() < INTER_SEND_WAIT {
                if let Some(incoming) = self.device.try_recv() {
                    let parsed = parse_eth(&incoming)
                        .map_err(|e| Error::illegal_argument(format!("{e:?}")))?;
                    packet_observer.add_packet(Source::Server(incoming));
                    if let Some(manual_response_res) = ShmemNetworkDevice::respond_manually(parsed)
                    {
                        let manual_response = manual_response_res?;
                        self.device.send(&manual_response);
                        packet_observer.add_packet(Source::Client(manual_response));
                    }

                    last_packet_time = Instant::now();
                }
                sleep(INTER_SEND_WAIT / 5);
            }
        }

        let res = child.try_wait().unwrap();
        child.kill().unwrap();
        child.wait().unwrap();

        let res = match res.map(|status| status.signal()) {
            Some(Some(_)) => ExitKind::Crash,
            Some(None) => ExitKind::Ok,
            None => ExitKind::Ok,
        };

        self.observers.post_exec_child_all(state, input, &res)?;

        log::debug!("Zephyr exited with ExitKind::{:#?}", res);

        Ok(res)
    }
}

impl<S, OT, II> HasObservers for ZepyhrExecutor<'_, S, OT, II> {
    type Observers = OT;

    fn observers(&self) -> RefIndexable<&Self::Observers, Self::Observers> {
        RefIndexable::from(&*self.observers)
    }

    fn observers_mut(&mut self) -> RefIndexable<&mut Self::Observers, Self::Observers> {
        RefIndexable::from(&mut *self.observers)
    }
}
