/// Managed child process handles and child-facing interaction types.
pub mod managed_child;

use std::collections::{HashMap, HashSet};
use std::process::{ExitStatus, Stdio};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use pork_proto::protocol::PorkChildStatus;

#[cfg(unix)]
use nix::sys::signal::{Signal, kill};
#[cfg(unix)]
use nix::unistd::Pid;

use ipc_channel::ipc::{IpcOneShotServer, IpcSender};

use crate::error::{OrchestratorError, ProcessId, Result};
use crate::ipc::HandshakeChannels;
use crate::spec::ProcessSpec;
use pork_proto::protocol::{PORK_CONTROL_CODEC_ENV, PorkControlCodec};

pub use managed_child::ManagedChild;

const DEFAULT_MESSAGE_BUFFER_SIZE: usize = 1024;
const DEFAULT_GRACEFUL_SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(5);
/// Default time the orchestrator waits for declared dependencies to reach
/// [`PorkChildStatus::Running`] before returning [`OrchestratorError::DependencyTimeout`].
const DEFAULT_DEPENDENCY_TIMEOUT: Duration = Duration::from_secs(30);
/// Interval between dependency-readiness polls while waiting.
const DEPENDENCY_POLL_INTERVAL: Duration = Duration::from_millis(50);

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
    graceful_shutdown_timeout: Duration,
    /// Maximum time to wait for declared dependencies to reach `Running`.
    dependency_timeout: Duration,
    processes: tokio::sync::RwLock<HashMap<ProcessId, ProcessEntry>>,
    process_names: tokio::sync::RwLock<HashMap<String, ProcessId>>,
}

#[derive(Debug)]
struct ProcessEntry {
    child: tokio::process::Child,
    sender: IpcSender<Vec<u8>>,
    managed_name: Option<String>,
    control_codec: PorkControlCodec,
    spec: ProcessSpec,
    status: PorkChildStatus,
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

    /// Returns a builder for configuring a [`ProcessOrchestrator`].
    pub fn builder() -> ProcessOrchestratorBuilder {
        ProcessOrchestratorBuilder::default()
    }

    /// Returns the default timeout used for graceful shutdown operations.
    pub fn graceful_shutdown_timeout(&self) -> Duration {
        self.inner.graceful_shutdown_timeout
    }

    /// Returns the default timeout used when waiting for declared dependencies
    /// to reach [`PorkChildStatus::Running`].
    pub fn dependency_timeout(&self) -> Duration {
        self.inner.dependency_timeout
    }

    /// Starts a new managed child process from the given [`ProcessSpec`].
    ///
    /// On success, returns a [`ManagedChild`] handle that can be used for
    /// messaging and lifecycle operations.
    pub async fn start_process(&self, spec: ProcessSpec) -> Result<ManagedChild> {
        self.start_process_inner(spec, true).await
    }

    /// Restarts an existing managed process by numeric id.
    ///
    /// This first performs a graceful shutdown using the orchestrator's default
    /// timeout and then starts a new process using the original [`ProcessSpec`].
    pub async fn restart_process(&self, process_id: ProcessId) -> Result<ManagedChild> {
        let spec = {
            let processes = self.inner.processes.read().await;
            let entry = processes
                .get(&process_id)
                .ok_or(OrchestratorError::ProcessNotFound(process_id))?;
            entry.spec.clone()
        };

        let _ = self.graceful_shutdown_process(process_id).await?;
        self.start_process_inner(spec, false).await
    }

    /// Restarts an existing managed process by numeric id using an explicit timeout.
    ///
    /// This first performs a graceful shutdown with `timeout` and then starts a
    /// new process using the original [`ProcessSpec`].
    pub async fn restart_process_with_timeout(
        &self,
        process_id: ProcessId,
        timeout: Duration,
    ) -> Result<ManagedChild> {
        let spec = {
            let processes = self.inner.processes.read().await;
            let entry = processes
                .get(&process_id)
                .ok_or(OrchestratorError::ProcessNotFound(process_id))?;
            entry.spec.clone()
        };

        let _ = self
            .graceful_shutdown_process_with_timeout(process_id, timeout)
            .await?;
        self.start_process_inner(spec, false).await
    }

    async fn start_process_inner(
        &self,
        spec: ProcessSpec,
        reserve_name: bool,
    ) -> Result<ManagedChild> {
        let managed_name = spec.managed_name.clone();
        let control_codec = spec.control_codec;
        let process_id = self.inner.next_process_id.fetch_add(1, Ordering::Relaxed);

        if reserve_name && let Some(process_name) = &managed_name {
            let mut process_names = self.inner.process_names.write().await;

            if process_names.contains_key(process_name) {
                return Err(OrchestratorError::DuplicateProcessName(
                    process_name.clone(),
                ));
            }

            process_names.insert(process_name.clone(), process_id);
        }

        // Validate declared dependencies and wait for them to become Running,
        // using the orchestrator's configured dependency timeout.
        if !spec.depends_on.is_empty() {
            // Eagerly reject any name that has never been registered so callers
            // get a clear error instead of silently timing out.
            self.check_dependencies_known(&spec.depends_on).await?;

            // Detect cycles before blocking: if this process has a name, check
            // that none of its transitive dependencies eventually declare a
            // dependency back on it.
            if let Some(ref name) = managed_name {
                self.check_dependency_cycle(name, &spec.depends_on).await?;
            }

            let timeout = self.inner.dependency_timeout;
            self.wait_for_dependencies(&spec.depends_on, timeout)
                .await?;
        }

        let start_result = async {
            let (bootstrap_server, bootstrap_name) = IpcOneShotServer::<HandshakeChannels>::new()?;

            let mut command = tokio::process::Command::new(&spec.executable);
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
            let (_, handshake) = tokio::task::spawn_blocking(move || bootstrap_server.accept())
                .await
                .map_err(|error| OrchestratorError::Io(std::io::Error::other(error)))??;

            let sender = handshake.to_child;
            let receiver = handshake.from_child;
            let receiver = Box::pin(receiver.to_stream());

            let managed_child = ManagedChild::new(
                process_id,
                managed_name.clone(),
                sender.clone(),
                Arc::new(tokio::sync::Mutex::new(receiver)),
            );

            let entry = ProcessEntry {
                child,
                sender,
                managed_name: managed_name.clone(),
                control_codec,
                spec,
                status: PorkChildStatus::Running,
            };

            let mut processes = self.inner.processes.write().await;
            processes.insert(process_id, entry);

            if let Some(process_name) = &managed_name {
                let mut process_names = self.inner.process_names.write().await;
                process_names.insert(process_name.clone(), process_id);
            }

            Ok(managed_child)
        }
        .await;

        if start_result.is_err()
            && reserve_name
            && let Some(process_name) = &managed_name
        {
            if let Ok(mut process_names) = self.inner.process_names.try_write() {
                process_names.remove(process_name);
            }
        }

        start_result
    }

    /// Returns an error if any name in `deps` is not currently registered.
    async fn check_dependencies_known(&self, deps: &[String]) -> Result<()> {
        let process_names = self.inner.process_names.read().await;
        for name in deps {
            if !process_names.contains_key(name) {
                return Err(OrchestratorError::DependencyNotFound(name.clone()));
            }
        }
        Ok(())
    }

    /// DFS cycle check: starting from `root`, walk the `depends_on` lists of
    /// every reachable process and report an error if `root` is encountered.
    async fn check_dependency_cycle(&self, root: &str, deps: &[String]) -> Result<()> {
        let processes = self.inner.processes.read().await;

        let mut stack: Vec<&str> = deps.iter().map(String::as_str).collect();
        let mut visited: HashSet<&str> = HashSet::new();

        while let Some(current) = stack.pop() {
            if current == root {
                // Collect the full cycle set for a useful error message.
                let cycle: Vec<String> = deps
                    .iter()
                    .cloned()
                    .chain(std::iter::once(root.to_owned()))
                    .collect();
                return Err(OrchestratorError::DependencyCycle(cycle));
            }

            if !visited.insert(current) {
                continue;
            }

            // Walk into transitive dependencies stored in the retained spec.
            let transitive = processes
                .values()
                .find(|e| e.managed_name.as_deref() == Some(current))
                .map(|e| e.spec.depends_on.as_slice())
                .unwrap_or(&[]);

            for dep in transitive {
                stack.push(dep.as_str());
            }
        }

        Ok(())
    }

    /// Polls every name in `deps` until all report [`PorkChildStatus::Running`]
    /// or `timeout` elapses.
    async fn wait_for_dependencies(&self, deps: &[String], timeout: Duration) -> Result<()> {
        let deadline = tokio::time::Instant::now() + timeout;

        loop {
            let mut not_ready: Vec<String> = Vec::new();

            {
                let processes = self.inner.processes.read().await;
                let process_names = self.inner.process_names.read().await;

                for name in deps {
                    let ready = process_names
                        .get(name)
                        .and_then(|id| processes.get(id))
                        .map(|e| e.status == PorkChildStatus::Running)
                        .unwrap_or(false);

                    if !ready {
                        not_ready.push(name.clone());
                    }
                }
            }

            if not_ready.is_empty() {
                return Ok(());
            }

            if tokio::time::Instant::now() >= deadline {
                return Err(OrchestratorError::DependencyTimeout(not_ready));
            }

            tokio::time::sleep(DEPENDENCY_POLL_INTERVAL).await;
        }
    }

    /// Sends a raw IPC payload to the managed process identified by `process_id`.
    pub async fn send(&self, process_id: ProcessId, message: Vec<u8>) -> Result<()> {
        let sender = {
            let processes = self.inner.processes.read().await;
            let entry = processes
                .get(&process_id)
                .ok_or(OrchestratorError::ProcessNotFound(process_id))?;
            entry.sender.clone()
        };

        sender.send(message)?;
        Ok(())
    }

    /// Returns the current lifecycle status of the managed process identified by `process_id`.
    pub async fn process_status(&self, process_id: ProcessId) -> Result<PorkChildStatus> {
        let processes = self.inner.processes.read().await;
        let entry = processes
            .get(&process_id)
            .ok_or(OrchestratorError::ProcessNotFound(process_id))?;
        Ok(entry.status)
    }

    /// Returns the current lifecycle status of the managed process identified by `name`.
    pub async fn process_status_by_name(&self, name: &str) -> Result<PorkChildStatus> {
        let process_id = {
            let process_names = self.inner.process_names.read().await;
            process_names
                .get(name)
                .copied()
                .ok_or_else(|| OrchestratorError::ProcessNameNotFound(name.to_owned()))?
        };

        self.process_status(process_id).await
    }

    /// Requests a graceful shutdown for the managed process identified by `process_id`.
    ///
    /// This sends the shared Pork control message over IPC and, on Unix, also
    /// sends `SIGTERM` to the process.
    pub async fn request_graceful_shutdown(&self, process_id: ProcessId) -> Result<()> {
        self.set_process_status(process_id, PorkChildStatus::Stopping)
            .await?;
        self.request_ipc_graceful_shutdown(process_id).await?;
        self.request_unix_graceful_shutdown(process_id).await?;
        Ok(())
    }

    /// Gracefully shuts down a managed process using the orchestrator's default timeout.
    pub async fn graceful_shutdown_process(&self, process_id: ProcessId) -> Result<ExitStatus> {
        self.graceful_shutdown_process_with_timeout(process_id, self.graceful_shutdown_timeout())
            .await
    }

    /// Gracefully shuts down a managed process using an explicit timeout.
    ///
    /// If the child does not exit before the timeout expires, the process is
    /// forcibly stopped.
    pub async fn graceful_shutdown_process_with_timeout(
        &self,
        process_id: ProcessId,
        timeout: Duration,
    ) -> Result<ExitStatus> {
        self.request_graceful_shutdown(process_id).await?;

        let sleep = tokio::time::sleep(timeout);
        tokio::pin!(sleep);

        tokio::select! {
            result = self.wait_for_exit(process_id) => {
                let status = result?;
                self.finish_process_shutdown(process_id, status).await
            }
            _ = &mut sleep => self.stop_process(process_id).await,
        }
    }

    async fn request_ipc_graceful_shutdown(&self, process_id: ProcessId) -> Result<()> {
        let (sender, codec) = {
            let processes = self.inner.processes.read().await;
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
    async fn request_unix_graceful_shutdown(&self, process_id: ProcessId) -> Result<()> {
        let raw_pid = {
            let processes = self.inner.processes.read().await;
            let entry = processes
                .get(&process_id)
                .ok_or(OrchestratorError::ProcessNotFound(process_id))?;
            entry
                .child
                .id()
                .ok_or(OrchestratorError::ProcessNotFound(process_id))? as i32
        };

        kill(Pid::from_raw(raw_pid), Signal::SIGTERM)
            .map_err(std::io::Error::from)
            .map_err(OrchestratorError::Io)?;
        Ok(())
    }

    #[cfg(not(unix))]
    async fn request_unix_graceful_shutdown(&self, _process_id: ProcessId) -> Result<()> {
        Ok(())
    }

    /// Immediately stops a managed process.
    ///
    /// This removes the process from the orchestrator, attempts to kill it, waits
    /// for the child to exit, and then cleans up background forwarding state.
    pub async fn stop_process(&self, process_id: ProcessId) -> Result<ExitStatus> {
        self.set_process_status(process_id, PorkChildStatus::Stopping)
            .await?;

        let mut entry = {
            let mut processes = self.inner.processes.write().await;
            let mut entry = processes
                .remove(&process_id)
                .ok_or(OrchestratorError::ProcessNotFound(process_id))?;
            entry.status = PorkChildStatus::Stopped;
            entry
        };

        let _ = entry.child.kill().await;
        let status = entry.child.wait().await?;

        if let Some(process_name) = &entry.managed_name {
            let mut process_names = self.inner.process_names.write().await;
            process_names.remove(process_name);
        }

        Ok(status)
    }

    async fn finish_process_shutdown(
        &self,
        process_id: ProcessId,
        status: ExitStatus,
    ) -> Result<ExitStatus> {
        let entry = {
            let mut processes = self.inner.processes.write().await;
            let mut entry = processes
                .remove(&process_id)
                .ok_or(OrchestratorError::ProcessNotFound(process_id))?;
            entry.status = PorkChildStatus::Stopped;
            entry
        };

        if let Some(process_name) = &entry.managed_name {
            let mut process_names = self.inner.process_names.write().await;
            process_names.remove(process_name);
        }

        Ok(status)
    }

    async fn wait_for_exit(&self, process_id: ProcessId) -> Result<ExitStatus> {
        let mut entry = {
            let mut processes = self.inner.processes.write().await;
            processes
                .remove(&process_id)
                .ok_or(OrchestratorError::ProcessNotFound(process_id))?
        };

        let status = entry.child.wait().await?;

        let mut processes = self.inner.processes.write().await;
        processes.insert(process_id, entry);

        Ok(status)
    }

    async fn set_process_status(
        &self,
        process_id: ProcessId,
        status: PorkChildStatus,
    ) -> Result<()> {
        let mut processes = self.inner.processes.write().await;
        let entry = processes
            .get_mut(&process_id)
            .ok_or(OrchestratorError::ProcessNotFound(process_id))?;
        entry.status = status;
        Ok(())
    }

    /// Returns the ids of all currently managed processes.
    pub async fn process_ids(&self) -> Result<Vec<ProcessId>> {
        let processes = self.inner.processes.read().await;
        Ok(processes.keys().copied().collect())
    }

    /// Looks up a managed process id by name.
    pub async fn process_id_by_name(&self, name: &str) -> Result<Option<ProcessId>> {
        let process_names = self.inner.process_names.read().await;
        Ok(process_names.get(name).copied())
    }

    /// Returns `true` when a managed process with the given name exists.
    pub async fn has_process_name(&self, name: &str) -> Result<bool> {
        let process_names = self.inner.process_names.read().await;
        Ok(process_names.contains_key(name))
    }

    /// Returns the managed names currently registered with the orchestrator.
    pub async fn process_names(&self) -> Result<Vec<String>> {
        let process_names = self.inner.process_names.read().await;
        Ok(process_names.keys().cloned().collect())
    }
}

/// Builder for configuring a [`ProcessOrchestrator`].
#[derive(Debug, Clone)]
pub struct ProcessOrchestratorBuilder {
    message_buffer_size: usize,
    graceful_shutdown_timeout: Duration,
    dependency_timeout: Duration,
}

impl Default for ProcessOrchestratorBuilder {
    fn default() -> Self {
        Self {
            message_buffer_size: DEFAULT_MESSAGE_BUFFER_SIZE,
            graceful_shutdown_timeout: DEFAULT_GRACEFUL_SHUTDOWN_TIMEOUT,
            dependency_timeout: DEFAULT_DEPENDENCY_TIMEOUT,
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

    /// Sets the default timeout used when waiting for declared dependencies
    /// to reach [`PorkChildStatus::Running`] before spawning a dependent process.
    pub fn dependency_timeout(mut self, value: Duration) -> Self {
        self.dependency_timeout = value;
        self
    }

    /// Builds a [`ProcessOrchestrator`] from the current builder configuration.
    pub fn build(self) -> ProcessOrchestrator {
        ProcessOrchestrator {
            inner: Arc::new(OrchestratorInner {
                next_process_id: AtomicU64::new(1),
                graceful_shutdown_timeout: self.graceful_shutdown_timeout,
                dependency_timeout: self.dependency_timeout,
                processes: tokio::sync::RwLock::new(HashMap::new()),
                process_names: tokio::sync::RwLock::new(HashMap::new()),
            }),
        }
    }
}
