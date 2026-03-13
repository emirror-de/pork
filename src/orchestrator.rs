mod managed_child;

pub use managed_child::ManagedChild;

use std::collections::HashMap;
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

use ipc_channel::ipc::{IpcOneShotServer, IpcReceiver, IpcSender};
use tokio::sync::{Mutex as AsyncMutex, mpsc};

use crate::error::{OrchestratorError, ProcessId, Result};
use crate::ipc::HandshakeChannels;
use crate::spec::ProcessSpec;

const DEFAULT_MESSAGE_BUFFER_SIZE: usize = 1024;

#[derive(Debug, Clone)]
pub struct ProcessOrchestrator {
    inner: Arc<OrchestratorInner>,
}

#[derive(Debug)]
struct OrchestratorInner {
    next_process_id: AtomicU64,
    message_buffer_size: usize,
    processes: Mutex<HashMap<ProcessId, ProcessEntry>>,
}

#[derive(Debug)]
struct ProcessEntry {
    child: Child,
    sender: IpcSender<Vec<u8>>,
    inbound_thread: Option<JoinHandle<()>>,
}

impl Default for ProcessOrchestrator {
    fn default() -> Self {
        Self::new()
    }
}

impl ProcessOrchestrator {
    pub fn new() -> Self {
        Self::with_message_buffer_size(DEFAULT_MESSAGE_BUFFER_SIZE)
    }

    pub fn with_message_buffer_size(message_buffer_size: usize) -> Self {
        Self {
            inner: Arc::new(OrchestratorInner {
                next_process_id: AtomicU64::new(1),
                message_buffer_size,
                processes: Mutex::new(HashMap::new()),
            }),
        }
    }

    pub fn start_process(&self, spec: ProcessSpec) -> Result<ManagedChild> {
        let process_id = self.inner.next_process_id.fetch_add(1, Ordering::Relaxed);

        let (bootstrap_server, bootstrap_name) = IpcOneShotServer::<HandshakeChannels>::new()?;

        let mut command = Command::new(&spec.executable);
        command.args(&spec.args);

        if let Some(current_dir) = &spec.current_dir {
            command.current_dir(current_dir);
        }

        for (key, value) in &spec.env {
            command.env(key, value);
        }

        command.env(&spec.bootstrap_env, bootstrap_name);

        if spec.capture_stdout {
            command.stdout(Stdio::piped());
        }

        if spec.capture_stderr {
            command.stderr(Stdio::piped());
        }

        let child = command.spawn()?;
        let (_, handshake) = bootstrap_server.accept()?;

        let sender = handshake.to_child;
        let receiver = handshake.from_child;

        let (message_tx, message_rx) = mpsc::channel(self.inner.message_buffer_size);
        let inbound_thread = spawn_forwarder_thread(receiver, message_tx);

        let managed_child = ManagedChild::new(
            process_id,
            sender.clone(),
            Arc::new(AsyncMutex::new(message_rx)),
            self.clone(),
        );

        let entry = ProcessEntry {
            child,
            sender,
            inbound_thread: Some(inbound_thread),
        };

        let mut processes = self
            .inner
            .processes
            .lock()
            .map_err(|_| OrchestratorError::LockPoisoned("processes"))?;
        processes.insert(process_id, entry);

        Ok(managed_child)
    }

    pub fn send(&self, process_id: ProcessId, message: Vec<u8>) -> Result<()> {
        let sender = {
            let processes = self
                .inner
                .processes
                .lock()
                .map_err(|_| OrchestratorError::LockPoisoned("processes"))?;
            let entry = processes
                .get(&process_id)
                .ok_or(OrchestratorError::ProcessNotFound(process_id))?;
            entry.sender.clone()
        };

        sender.send(message)?;
        Ok(())
    }

    pub fn stop_process(&self, process_id: ProcessId) -> Result<ExitStatus> {
        let mut entry = {
            let mut processes = self
                .inner
                .processes
                .lock()
                .map_err(|_| OrchestratorError::LockPoisoned("processes"))?;
            processes
                .remove(&process_id)
                .ok_or(OrchestratorError::ProcessNotFound(process_id))?
        };

        let _ = entry.child.kill();
        let status = entry.child.wait()?;

        if let Some(handle) = entry.inbound_thread.take() {
            let _ = handle.join();
        }

        Ok(status)
    }

    pub fn try_wait(&self, process_id: ProcessId) -> Result<Option<ExitStatus>> {
        let mut processes = self
            .inner
            .processes
            .lock()
            .map_err(|_| OrchestratorError::LockPoisoned("processes"))?;
        let entry = processes
            .get_mut(&process_id)
            .ok_or(OrchestratorError::ProcessNotFound(process_id))?;
        Ok(entry.child.try_wait()?)
    }

    pub fn process_ids(&self) -> Result<Vec<ProcessId>> {
        let processes = self
            .inner
            .processes
            .lock()
            .map_err(|_| OrchestratorError::LockPoisoned("processes"))?;
        Ok(processes.keys().copied().collect())
    }
}

fn spawn_forwarder_thread(
    receiver: IpcReceiver<Vec<u8>>,
    outbound: mpsc::Sender<Vec<u8>>,
) -> JoinHandle<()> {
    std::thread::spawn(move || {
        while let Ok(message) = receiver.recv() {
            if outbound.blocking_send(message).is_err() {
                break;
            }
        }
    })
}
