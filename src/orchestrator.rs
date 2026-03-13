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

use crate::error::{OrchestratorError, ProcessId, Result};
use crate::ipc::HandshakeChannels;
use crate::{PORK_CONTROL_CODEC_ENV, PorkControlCodec, ProcessSpec};

const DEFAULT_MESSAGE_BUFFER_SIZE: usize = 1024;
const DEFAULT_GRACEFUL_SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(5);

/// Coordinates child-process startup, messaging, lookup, restart, and shutdown.
///
/// A `ProcessOrchestrator` owns the parent side of the bootstrap handshake and
/// keeps track of running managed processes by both numeric id and optional
/// managed name.
#[derive(Debug, Clone)]
pub struct ProcessOrchestrator {
    inner: Arc<OrchestratorInner>,
}

#[derive(Debug)]
struct OrchestratorInner {
    next_process_id: AtomicU64,
    message_buffer_size: usize,
    graceful_shutdown_timeout: Duration,
    processes: Mutex<HashMap<ProcessId, ProcessEntry>>,
    process_names: Mutex<HashMap<String, ProcessId>>,
}

#[derive(Debug)]
struct ProcessEntry {
    child: Child,
    sender: IpcSender<Vec<u8>>,
    inbound_thread: Option<JoinHandle<()>>,
    managed_name: Option<String>,
    control_codec: PorkControlCodec,
    spec: ProcessSpec,
}

impl Default for ProcessOrchestrator {
    fn default() -> Self {
        Self::new()
    }
}

impl ProcessOrchestrator {
    /// Creates an orchestrator with the default buffer size and graceful-shutdown timeout.
    pub fn new() -> Self {
        Self::builder().build()
    }

    /// Creates an orchestrator with a custom inbound message buffer size.
    ///
    /// This controls the size of the Tokio channel used to forward child-to-host
    /// messages from the IPC receiver thread into async code.
    pub fn with_message_buffer_size(message_buffer_size: usize) -> Self {
        Self::builder()
            .message_buffer_size(message_buffer_size)
            .build()
    }

    /// Creates an orchestrator with a custom graceful-shutdown timeout.
    ///
    /// The timeout is used by [`Self::graceful_shutdown_process`] and the
    /// corresponding convenience methods on [`ManagedChild`].
    pub fn with_graceful_shutdown_timeout(graceful_shutdown_timeout: Duration) -> Self {
        Self::builder()
            .graceful_shutdown_timeout(graceful_shutdown_timeout)
            .build()
    }

    /// Returns a builder for configuring a [`ProcessOrchestrator`].
    pub fn builder() -> ProcessOrchestratorBuilder {
        ProcessOrchestratorBuilder::default()
    }

    /// Returns the default timeout used for graceful shutdown operations.
    pub fn graceful_shutdown_timeout(&self) -> Duration {
        self.inner.graceful_shutdown_timeout
    }

    /// Starts a new managed child process from the given [`ProcessSpec`].
    ///
    /// On success, returns a [`ManagedChild`] handle that can be used for
    /// messaging and lifecycle operations.
    pub fn start_process(&self, spec: ProcessSpec) -> Result<ManagedChild> {
        self.start_process_inner(spec, true)
    }

    /// Restarts an existing managed process by numeric id.
    ///
    /// This first performs a graceful shutdown using the orchestrator's default
    /// timeout and then starts a new process using the original [`ProcessSpec`].
    pub fn restart_process(&self, process_id: ProcessId) -> Result<ManagedChild> {
        let spec = {
            let processes = self
                .inner
                .processes
                .lock()
                .map_err(|_| OrchestratorError::LockPoisoned("processes"))?;
            let entry = processes
                .get(&process_id)
                .ok_or(OrchestratorError::ProcessNotFound(process_id))?;
            entry.spec.clone()
        };

        let _ = self.graceful_shutdown_process(process_id)?;
        self.start_process_inner(spec, false)
    }

    /// Restarts an existing managed process by numeric id using an explicit timeout.
    ///
    /// This first performs a graceful shutdown with `timeout` and then starts a
    /// new process using the original [`ProcessSpec`].
    pub fn restart_process_with_timeout(
        &self,
        process_id: ProcessId,
        timeout: Duration,
    ) -> Result<ManagedChild> {
        let spec = {
            let processes = self
                .inner
                .processes
                .lock()
                .map_err(|_| OrchestratorError::LockPoisoned("processes"))?;
            let entry = processes
                .get(&process_id)
                .ok_or(OrchestratorError::ProcessNotFound(process_id))?;
            entry.spec.clone()
        };

        let _ = self.graceful_shutdown_process_with_timeout(process_id, timeout)?;
        self.start_process_inner(spec, false)
    }

    /// Restarts an existing managed process by its managed name.
    pub fn restart_process_by_name(&self, name: &str) -> Result<ManagedChild> {
        let process_id = self
            .process_id_by_name(name)?
            .ok_or_else(|| OrchestratorError::ProcessNameNotFound(name.to_owned()))?;
        self.restart_process(process_id)
    }

    fn start_process_inner(&self, spec: ProcessSpec, reserve_name: bool) -> Result<ManagedChild> {
        let managed_name = spec.managed_name.clone();
        let control_codec = spec.control_codec;
        let process_id = self.inner.next_process_id.fetch_add(1, Ordering::Relaxed);

        if reserve_name {
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
            command.env(PORK_CONTROL_CODEC_ENV, control_codec.as_env_value());

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

            let (message_tx, message_rx) =
                tokio::sync::mpsc::channel(self.inner.message_buffer_size);
            let inbound_thread = spawn_forwarder_thread(receiver, message_tx);

            let managed_child = ManagedChild::new(
                process_id,
                managed_name.clone(),
                sender.clone(),
                Arc::new(tokio::sync::Mutex::new(message_rx)),
                self.clone(),
            );

            let entry = ProcessEntry {
                child,
                sender,
                inbound_thread: Some(inbound_thread),
                managed_name: managed_name.clone(),
                control_codec,
                spec,
            };

            let mut processes = self
                .inner
                .processes
                .lock()
                .map_err(|_| OrchestratorError::LockPoisoned("processes"))?;
            processes.insert(process_id, entry);

            if let Some(process_name) = &managed_name {
                let mut process_names = self
                    .inner
                    .process_names
                    .lock()
                    .map_err(|_| OrchestratorError::LockPoisoned("process_names"))?;
                process_names.insert(process_name.clone(), process_id);
            }

            Ok(managed_child)
        })();

        if start_result.is_err() && reserve_name {
            if let Some(process_name) = &managed_name {
                if let Ok(mut process_names) = self.inner.process_names.lock() {
                    process_names.remove(process_name);
                }
            }
        }

        start_result
    }

    /// Sends a raw IPC payload to the managed process identified by `process_id`.
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

    /// Requests a graceful shutdown for the managed process identified by `process_id`.
    ///
    /// This sends the shared Pork control message over IPC and, on Unix, also
    /// sends `SIGTERM` to the process.
    pub fn request_graceful_shutdown(&self, process_id: ProcessId) -> Result<()> {
        self.request_ipc_graceful_shutdown(process_id)?;
        self.request_unix_graceful_shutdown(process_id)?;
        Ok(())
    }

    /// Gracefully shuts down a managed process using the orchestrator's default timeout.
    pub fn graceful_shutdown_process(&self, process_id: ProcessId) -> Result<ExitStatus> {
        self.graceful_shutdown_process_with_timeout(process_id, self.graceful_shutdown_timeout())
    }

    /// Gracefully shuts down a managed process using an explicit timeout.
    ///
    /// If the child does not exit before the timeout expires, the process is
    /// forcibly stopped.
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
        let (sender, codec) = {
            let processes = self
                .inner
                .processes
                .lock()
                .map_err(|_| OrchestratorError::LockPoisoned("processes"))?;
            let entry = processes
                .get(&process_id)
                .ok_or(OrchestratorError::ProcessNotFound(process_id))?;
            (entry.sender.clone(), entry.control_codec)
        };

        let payload = codec
            .encode_graceful_shutdown()
            .map_err(|error| OrchestratorError::Io(std::io::Error::other(error)))?;
        sender.send(payload)?;
        Ok(())
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

    /// Immediately stops a managed process.
    ///
    /// This removes the process from the orchestrator, attempts to kill it, waits
    /// for the child to exit, and then cleans up background forwarding state.
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

    /// Checks whether the managed child has already exited without blocking.
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

    /// Returns the ids of all currently managed processes.
    pub fn process_ids(&self) -> Result<Vec<ProcessId>> {
        let processes = self
            .inner
            .processes
            .lock()
            .map_err(|_| OrchestratorError::LockPoisoned("processes"))?;
        Ok(processes.keys().copied().collect())
    }

    /// Looks up a managed process id by name.
    pub fn process_id_by_name(&self, name: &str) -> Result<Option<ProcessId>> {
        let process_names = self
            .inner
            .process_names
            .lock()
            .map_err(|_| OrchestratorError::LockPoisoned("process_names"))?;
        Ok(process_names.get(name).copied())
    }

    /// Returns `true` when a managed process with the given name exists.
    pub fn has_process_name(&self, name: &str) -> Result<bool> {
        let process_names = self
            .inner
            .process_names
            .lock()
            .map_err(|_| OrchestratorError::LockPoisoned("process_names"))?;
        Ok(process_names.contains_key(name))
    }

    /// Returns the managed names currently registered with the orchestrator.
    pub fn process_names(&self) -> Result<Vec<String>> {
        let process_names = self
            .inner
            .process_names
            .lock()
            .map_err(|_| OrchestratorError::LockPoisoned("process_names"))?;
        Ok(process_names.keys().cloned().collect())
    }
}

/// Builder for configuring a [`ProcessOrchestrator`].
#[derive(Debug, Clone)]
pub struct ProcessOrchestratorBuilder {
    message_buffer_size: usize,
    graceful_shutdown_timeout: Duration,
}

impl Default for ProcessOrchestratorBuilder {
    fn default() -> Self {
        Self {
            message_buffer_size: DEFAULT_MESSAGE_BUFFER_SIZE,
            graceful_shutdown_timeout: DEFAULT_GRACEFUL_SHUTDOWN_TIMEOUT,
        }
    }
}

impl ProcessOrchestratorBuilder {
    /// Sets the size of the async buffer used for inbound child messages.
    pub fn message_buffer_size(mut self, value: usize) -> Self {
        self.message_buffer_size = value;
        self
    }

    /// Sets the default timeout used for graceful shutdown operations.
    pub fn graceful_shutdown_timeout(mut self, value: Duration) -> Self {
        self.graceful_shutdown_timeout = value;
        self
    }

    /// Builds a [`ProcessOrchestrator`] from the current builder configuration.
    pub fn build(self) -> ProcessOrchestrator {
        ProcessOrchestrator {
            inner: Arc::new(OrchestratorInner {
                next_process_id: AtomicU64::new(1),
                message_buffer_size: self.message_buffer_size,
                graceful_shutdown_timeout: self.graceful_shutdown_timeout,
                processes: Mutex::new(HashMap::new()),
                process_names: Mutex::new(HashMap::new()),
            }),
        }
    }
}

fn spawn_forwarder_thread(
    receiver: IpcReceiver<Vec<u8>>,
    outbound: tokio::sync::mpsc::Sender<Vec<u8>>,
) -> JoinHandle<()> {
    std::thread::spawn(move || {
        while let Ok(message) = receiver.recv() {
            if outbound.blocking_send(message).is_err() {
                break;
            }
        }
    })
}
