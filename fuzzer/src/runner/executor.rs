use std::{
    ffi::CStr,
    fmt::Debug,
    marker::PhantomData,
    os::unix::process::ExitStatusExt as _,
    path::PathBuf,
    process::{Command, Stdio},
    thread::sleep,
};

use libafl::{
    executors::{Executor, ExitKind, HasObservers},
    inputs::HasMutatorBytes,
    observers::ObserversTuple,
    state::{HasExecutions, State, UsesState},
    Error,
};
use libafl_bolts::{
    shmem::ShMemDescription,
    tuples::{Handle, MatchName, MatchNameRef, RefIndexable},
};

use crate::runner::INTER_SEND_WAIT;

use crate::smoltcp::shmem_net_device::ShmemNetworkDevice;

use super::{input::ZephyrInput, metadata::PacketObserver};

pub struct ZepyhrExecutor<S, OT> {
    observers: OT,
    packet_observer: Handle<PacketObserver>,
    device: ShmemNetworkDevice,
    envs: Vec<(String, String)>,
    zephyr_exec_path: PathBuf,
    phantom: PhantomData<S>,
}

impl<S, OT> ZepyhrExecutor<S, OT> {
    pub fn new(
        observers: OT,
        packet_observer: Handle<PacketObserver>,
        cov_shmem_desc: &ShMemDescription,
        zephyr_exec_path: PathBuf,
        network_buf_size: usize,
    ) -> Result<Self, Error> {
        let device = ShmemNetworkDevice::new(network_buf_size)?;
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
            phantom: PhantomData,
        })
    }
}

impl<EM, Z, S, OT> Executor<EM, Z> for ZepyhrExecutor<S, OT>
where
    Z: UsesState<State = S>,
    EM: UsesState<State = S>,
    S: State<Input = ZephyrInput> + HasExecutions,
    OT: Debug + MatchName + MatchNameRef + ObserversTuple<ZephyrInput, S>,
{
    fn run_target(
        &mut self,
        _fuzzer: &mut Z,
        state: &mut Self::State,
        _mgr: &mut EM,
        input: &Self::Input,
    ) -> Result<ExitKind, Error> {
        log::debug!("Starting input run on target");
        *state.executions_mut() += 1;

        self.observers.pre_exec_child_all(state, input)?;
        let packets_observer = self
            .observers
            .get_mut(&self.packet_observer)
            .ok_or(Error::illegal_argument(
            "Could not retrieve PacketObserver, make sure you pass it to the executor in the OT.",
        ))?;

        log::debug!("Preparing Zephyr");

        self.device.reset();

        let mut child = Command::new(self.zephyr_exec_path.clone())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .envs(self.envs.to_owned())
            .spawn()
            .map_err(|e| Error::unknown(format!("Could not start command: {e:?}")))?;

        self.device
            .init_zephyr(|packet| packets_observer.add_packet(packet.inner()))?;

        log::debug!("Started Zephyr");

        for e in input.parts() {
            packets_observer.add_packet(e.bytes().to_vec());
            self.device.send(e.bytes());
            sleep(INTER_SEND_WAIT);
            while let Some(incoming) = self.device.try_recv() {
                packets_observer.add_packet(incoming);
                sleep(INTER_SEND_WAIT);
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

        if res == ExitKind::Crash {
            log::info!("Got crash!");
        }

        log::debug!("Zephyr exited with ExitKind::{:#?}", res);

        self.observers.post_exec_child_all(state, input, &res)?;

        Ok(res)
    }
}

impl<S, OT> UsesState for ZepyhrExecutor<S, OT>
where
    S: State,
{
    type State = S;
}

impl<S, OT> HasObservers for ZepyhrExecutor<S, OT> {
    type Observers = OT;

    fn observers(&self) -> RefIndexable<&Self::Observers, Self::Observers> {
        RefIndexable::from(&self.observers)
    }

    fn observers_mut(&mut self) -> RefIndexable<&mut Self::Observers, Self::Observers> {
        RefIndexable::from(&mut self.observers)
    }
}

fn get_path(shmem_desc: &ShMemDescription) -> Result<&str, Error> {
    CStr::from_bytes_until_nul(&shmem_desc.id)
        .map_err(|e| {
            Error::illegal_argument(format!("Error parsing path from shmem desc: {:?}", e))
        })?
        .to_str()
        .map_err(|e| {
            Error::illegal_argument(format!("Could not parse string from shmmem path: {:?}", e))
        })
}
