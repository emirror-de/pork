mod managed_child;

pub use managed_child::ManagedChild;

use std::collections::HashMap;
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

#[cfg(unix)]
use nix::sys::signal::{Signal, kill};
#[cfg(unix)]
use nix::unistd::Pid;

use ipc_channel::ipc::{IpcOneShotServer, IpcReceiver, IpcSender};
use tokio::sync::{Mutex as AsyncMutex, mpsc};

use crate::error::{OrchestratorError, ProcessId, Result};
use crate::ipc::HandshakeChannels;
use crate::spec::ProcessSpec;

const DEFAULT_MESSAGE_BUFFER_SIZE: usize = 1024;
const DEFAULT_GRACEFUL_SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Debug, Clone)]
pub struct ProcessOrchestrator {
    inner: Arc<OrchestratorInner>,
}

#[derive(Debug)]
struct OrchestratorInner {
    next_process_id: AtomicU64,
    message_buffer_size: usize,
    processes: Mutex<HashMap<ProcessId, ProcessEntry>>,
    process_names: Mutex<HashMap<String, ProcessId>>,
}

#[derive(Debug)]
struct ProcessEntry {
    child: Child,
    sender: IpcSender<Vec<u8>>,
    inbound_thread: Option<JoinHandle<()>>,
    managed_name: Option<String>,
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
                process_names: Mutex::new(HashMap::new()),
            }),
        }
    }

    pub fn start_process(&self, spec: ProcessSpec) -> Result<ManagedChild> {
        let managed_name = spec.managed_name.clone();
        let process_id = self.inner.next_process_id.fetch_add(1, Ordering::Relaxed);

        if let Some(process_name) = &managed_name {
            let mut process_names = self
                .inner
                .process_names
                .lock()
                .map_err(|_| OrchestratorError::LockPoisoned("process_names"))?;

            if process_names.contains_key(process_name) {
                return Err(OrchestratorError::DuplicateProcessName(
                    process_name.clone(),
                ));
            }

            process_names.insert(process_name.clone(), process_id);
        }

        let start_result = (|| -> Result<ManagedChild> {
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
                managed_name.clone(),
                sender.clone(),
                Arc::new(AsyncMutex::new(message_rx)),
                self.clone(),
            );

            let entry = ProcessEntry {
                child,
                sender,
                inbound_thread: Some(inbound_thread),
                managed_name: managed_name.clone(),
            };

            let mut processes = self
                .inner
                .processes
                .lock()
                .map_err(|_| OrchestratorError::LockPoisoned("processes"))?;
            processes.insert(process_id, entry);

            Ok(managed_child)
        })();

        if start_result.is_err() {
            if let Some(process_name) = &managed_name {
                if let Ok(mut process_names) = self.inner.process_names.lock() {
                    process_names.remove(process_name);
                }
            }
        }

        start_result
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

    pub fn request_graceful_shutdown(&self, process_id: ProcessId) -> Result<()> {
        self.request_ipc_graceful_shutdown(process_id)?;
        self.request_unix_graceful_shutdown(process_id)?;
        Ok(())
    }

    pub fn graceful_shutdown_process(&self, process_id: ProcessId) -> Result<ExitStatus> {
        self.graceful_shutdown_process_with_timeout(process_id, DEFAULT_GRACEFUL_SHUTDOWN_TIMEOUT)
    }

    pub fn graceful_shutdown_process_with_timeout(
        &self,
        process_id: ProcessId,
        timeout: Duration,
    ) -> Result<ExitStatus> {
        self.request_graceful_shutdown(process_id)?;

        let deadline = Instant::now() + timeout;

        loop {
            if let Some(status) = self.try_wait(process_id)? {
                return self.finish_process_shutdown(process_id, status);
            }

            if Instant::now() >= deadline {
                return self.stop_process(process_id);
            }

            std::thread::sleep(Duration::from_millis(25));
        }
    }

    fn request_ipc_graceful_shutdown(&self, process_id: ProcessId) -> Result<()> {
        self.send(process_id, crate::graceful_shutdown_message())
    }

    #[cfg(unix)]
    fn request_unix_graceful_shutdown(&self, process_id: ProcessId) -> Result<()> {
        let raw_pid = {
            let processes = self
                .inner
                .processes
                .lock()
                .map_err(|_| OrchestratorError::LockPoisoned("processes"))?;
            let entry = processes
                .get(&process_id)
                .ok_or(OrchestratorError::ProcessNotFound(process_id))?;
            entry.child.id() as i32
        };

        kill(Pid::from_raw(raw_pid), Signal::SIGTERM)
            .map_err(std::io::Error::from)
            .map_err(OrchestratorError::Io)?;
        Ok(())
    }

    #[cfg(not(unix))]
    fn request_unix_graceful_shutdown(&self, _process_id: ProcessId) -> Result<()> {
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

        if let Some(process_name) = &entry.managed_name {
            let mut process_names = self
                .inner
                .process_names
                .lock()
                .map_err(|_| OrchestratorError::LockPoisoned("process_names"))?;
            process_names.remove(process_name);
        }

        Ok(status)
    }

    fn finish_process_shutdown(
        &self,
        process_id: ProcessId,
        status: ExitStatus,
    ) -> Result<ExitStatus> {
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

        if let Some(handle) = entry.inbound_thread.take() {
            let _ = handle.join();
        }

        if let Some(process_name) = &entry.managed_name {
            let mut process_names = self
                .inner
                .process_names
                .lock()
                .map_err(|_| OrchestratorError::LockPoisoned("process_names"))?;
            process_names.remove(process_name);
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

    pub fn process_id_by_name(&self, name: &str) -> Result<Option<ProcessId>> {
        let process_names = self
            .inner
            .process_names
            .lock()
            .map_err(|_| OrchestratorError::LockPoisoned("process_names"))?;
        Ok(process_names.get(name).copied())
    }

    pub fn has_process_name(&self, name: &str) -> Result<bool> {
        let process_names = self
            .inner
            .process_names
            .lock()
            .map_err(|_| OrchestratorError::LockPoisoned("process_names"))?;
        Ok(process_names.contains_key(name))
    }

    pub fn process_names(&self) -> Result<Vec<String>> {
        let process_names = self
            .inner
            .process_names
            .lock()
            .map_err(|_| OrchestratorError::LockPoisoned("process_names"))?;
        Ok(process_names.keys().cloned().collect())
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
