use std::{
    ffi::CStr, fmt::Debug, marker::PhantomData, os::unix::process::ExitStatusExt as _,
    path::PathBuf, thread::sleep,
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

use crate::runner::{zephyr::init_zephyr, INTER_SEND_WAIT};

use crate::smoltcp::shmem_net_device::ShmemNetworkDevice;

use super::{input::ZephyrInput, metadata::PacketObserver, thread::RunnerThread};

pub struct ZepyhrExecutor<S, OT> {
    observers: OT,
    packet_observer: Handle<PacketObserver>,
    device: ShmemNetworkDevice,
    thread: RunnerThread,
    phantom: PhantomData<S>,
}

impl<S, OT> ZepyhrExecutor<S, OT> {
    pub fn new(
        observers: OT,
        packet_observer: Handle<PacketObserver>,
        cov_shmem_description: &ShMemDescription,
        zephyr_exec_path: PathBuf,
        network_buf_size: usize,
    ) -> Result<Self, Error> {
        let device = ShmemNetworkDevice::new(network_buf_size)?;
        let net_shmem_description = device.get_shmem_description();
        let net_shmem_size = net_shmem_description.size.to_string();
        let net_shmem_name = get_path_for_mmap_shmem(&net_shmem_description)?;

        let cov_shmem_size = &cov_shmem_description.size.to_string();
        let cov_shmem_name = get_path_for_mmap_shmem(cov_shmem_description)?;

        let envs: &[(&str, &str)] = &[
            ("SHMEM_ETH_INTERFACE_SIZE", &net_shmem_size.to_string()),
            ("SHMEM_ETH_INTERFACE_NAME", net_shmem_name),
            ("SHMEM_COVERAGE_SIZE", &cov_shmem_size.to_string()),
            ("SHMEM_COVERAGE_NAME", cov_shmem_name),
        ];

        let thread = RunnerThread::new(zephyr_exec_path, envs);

        Ok(Self {
            observers,
            packet_observer,
            device,
            thread,
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

        let start = self.thread.start();

        if let Err(e) = start {
            log::warn!("Received error from start command: {:?}", e);
            return Err(e);
        }

        init_zephyr(&mut self.device, |packet| {
            packets_observer.add_packet(packet.inner())
        })?;

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

        let kill = self.thread.kill();

        if let Err(e) = &kill {
            log::warn!("Received error from kill command: {:?}", e);
        }

        let res = match kill?.map(|status| status.signal()) {
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

fn get_path_for_mmap_shmem(shmem_description: &ShMemDescription) -> Result<&str, Error> {
    CStr::from_bytes_until_nul(&shmem_description.id)
        .map_err(|e| {
            Error::illegal_argument(format!(
                "Error parsing path from shmem description: {:?}",
                e
            ))
        })?
        .to_str()
        .map_err(|e| {
            Error::illegal_argument(format!("Could not parse string from shmmem path: {:?}", e))
        })
}
