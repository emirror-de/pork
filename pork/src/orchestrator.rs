use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::process::{ExitStatus, Stdio};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use futures_util::StreamExt;
use pork_proto::protocol::{PorkChildStatus, PorkControlMessage, PorkStatusUpdate};

use crate::error::{OrchestratorError, Result};
#[cfg(feature = "host")]
use crate::host::HostBootstrap;
#[cfg(feature = "host")]
use crate::host::channels::{HostControlSender, HostDataSender};
use crate::spec::ProcessSpec;
use crate::types::ManagedChild;
use crate::types::{ControlPayload, DataPayload, ManagedChildName, ProcessId};
use pork_proto::protocol::{PORK_CONTROL_CODEC_ENV, PorkControlCodec};

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
    process_names: tokio::sync::RwLock<HashMap<ManagedChildName, ProcessId>>,
}

#[derive(Debug)]
struct ProcessEntry {
    child: Arc<tokio::sync::Mutex<tokio::process::Child>>,
    data_sender: HostDataSender,
    control_sender: HostControlSender,
    managed_name: Option<ManagedChildName>,
    control_codec: PorkControlCodec,
    spec: ProcessSpec,
    status: PorkChildStatus,
    /// Latest child-reported status update, including the child's timestamp.
    child_reported_status: Arc<tokio::sync::Mutex<Option<PorkStatusUpdate>>>,
    /// Handle to the background control channel receiver task.
    control_task_handle: tokio::task::JoinHandle<()>,
}

async fn shutdown_control_task(control_task_handle: tokio::task::JoinHandle<()>) {
    control_task_handle.abort();
    let _ = control_task_handle.await;
}

async fn configure_process_output(
    spec: &ProcessSpec,
    command: &mut tokio::process::Command,
) -> Result<()> {
    match &spec.output {
        crate::spec::ProcessOutput::Inherit => {}
        crate::spec::ProcessOutput::Capture { stdout, stderr } => {
            if *stdout {
                command.stdout(Stdio::piped());
            }
            if *stderr {
                command.stderr(Stdio::piped());
            }
        }
        crate::spec::ProcessOutput::Log { stdout, stderr } => {
            if let Some(path) = stdout {
                command.stdout(Stdio::from(open_append_log(path.as_path()).await?));
            }
            if let Some(path) = stderr {
                command.stderr(Stdio::from(open_append_log(path.as_path()).await?));
            }
        }
    }

    Ok(())
}

async fn open_append_log(path: &Path) -> Result<std::fs::File> {
    let file = tokio::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .await?;
    Ok(file.into_std().await)
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
    /// This sends an encoded [`PorkControlMessage::Restart`] request over the
    /// dedicated control channel, waits up to the orchestrator's default timeout
    /// for exit, force-kills on timeout, and then starts a replacement process
    /// from the original [`ProcessSpec`].
    pub async fn restart_process(&self, process_id: ProcessId) -> Result<ManagedChild> {
        self.restart_process_with_timeout(process_id, self.graceful_shutdown_timeout())
            .await
    }

    /// Restarts an existing managed process by numeric id using an explicit timeout.
    ///
    /// This sends an encoded [`PorkControlMessage::Restart`] request, waits up to
    /// `timeout` for exit, force-kills on timeout, and then starts a new process
    /// using the original [`ProcessSpec`].
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

        self.request_lifecycle_control(
            process_id,
            PorkChildStatus::Restarting,
            PorkControlMessage::Restart,
        )
        .await?;
        let _ = self.wait_for_exit_or_stop(process_id, timeout).await?;
        self.start_process_inner(spec, false).await
    }

    #[cfg(feature = "host")]
    async fn start_process_inner(
        &self,
        spec: ProcessSpec,
        reserve_name: bool,
    ) -> Result<ManagedChild> {
        let managed_name = spec.managed_name.clone();
        let control_codec = spec.control_codec;
        let process_id =
            ProcessId::from(self.inner.next_process_id.fetch_add(1, Ordering::Relaxed));

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
            let (bootstrap_env, servers) = HostBootstrap::create_servers().await?;

            let mut command = tokio::process::Command::new(spec.executable.as_path());
            command.args(&spec.args);

            if let Some(current_dir) = &spec.current_dir {
                command.current_dir(current_dir);
            }

            for (key, value) in &spec.env {
                command.env(key, value);
            }

            bootstrap_env.apply_to_command(
                &mut command,
                spec.data_bootstrap_env.as_str(),
                spec.control_bootstrap_env.as_str(),
            );
            command.env(PORK_CONTROL_CODEC_ENV, control_codec.as_env_value());
            configure_process_output(&spec, &mut command).await?;

            let child = Arc::new(tokio::sync::Mutex::new(command.spawn()?));
            let channels = HostBootstrap::accept_connections(servers).await?;

            let data_sender = channels.data_sender;
            let data_receiver = channels.data_receiver;
            let control_sender = channels.control_sender;
            let control_receiver = channels.control_receiver;
            let receiver = Box::pin(data_receiver.into_stream());

            let managed_child = ManagedChild::new(
                process_id,
                managed_name.clone(),
                data_sender.clone_inner(),
                Arc::new(tokio::sync::Mutex::new(receiver)),
            );

            // Spawn background task to receive child-reported status updates from the control channel.
            let child_reported_status = Arc::new(tokio::sync::Mutex::new(None));
            let child_reported_status_clone = child_reported_status.clone();
            let control_task_handle = tokio::spawn(async move {
                let mut control_stream = control_receiver.into_inner().to_stream();
                while let Some(Ok(payload)) = control_stream.next().await {
                    let payload = ControlPayload::from(payload);
                    let Ok(message) = control_codec.decode_control_message(payload.as_ref()) else {
                        continue;
                    };
                    if let pork_proto::protocol::PorkControlMessage::StatusUpdate(update) = message
                    {
                        let mut status = child_reported_status_clone.lock().await;
                        *status = Some(update);
                    }
                }
            });

            let entry = ProcessEntry {
                child,
                data_sender,
                control_sender,
                managed_name: managed_name.clone(),
                control_codec,
                spec,
                status: PorkChildStatus::Running,
                child_reported_status,
                control_task_handle,
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
            && let Ok(mut process_names) = self.inner.process_names.try_write()
        {
            process_names.remove(process_name);
        }

        start_result
    }

    #[cfg(not(feature = "host"))]
    async fn start_process_inner(
        &self,
        _spec: ProcessSpec,
        _reserve_name: bool,
    ) -> Result<ManagedChild> {
        Err(OrchestratorError::Io(std::io::Error::new(
            std::io::ErrorKind::Other,
            "host feature not enabled",
        )))
    }

    /// Returns an error if any name in `deps` is not currently registered.
    async fn check_dependencies_known(&self, deps: &[ManagedChildName]) -> Result<()> {
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
    async fn check_dependency_cycle(
        &self,
        root: &ManagedChildName,
        deps: &[ManagedChildName],
    ) -> Result<()> {
        let processes = self.inner.processes.read().await;

        let mut stack: Vec<&ManagedChildName> = deps.iter().collect();
        let mut visited: HashSet<&ManagedChildName> = HashSet::new();

        while let Some(current) = stack.pop() {
            if current == root {
                // Collect the full cycle set for a useful error message.
                let cycle: Vec<ManagedChildName> = deps
                    .iter()
                    .cloned()
                    .chain(std::iter::once(root.clone()))
                    .collect();
                return Err(OrchestratorError::DependencyCycle(cycle));
            }

            if !visited.insert(current) {
                continue;
            }

            // Walk into transitive dependencies stored in the retained spec.
            let transitive = processes
                .values()
                .find(|e| e.managed_name.as_ref() == Some(current))
                .map(|e| e.spec.depends_on.as_slice())
                .unwrap_or(&[]);

            for dep in transitive {
                stack.push(dep);
            }
        }

        Ok(())
    }

    /// Polls every name in `deps` until all report [`PorkChildStatus::Running`]
    /// or `timeout` elapses.
    async fn wait_for_dependencies(
        &self,
        deps: &[ManagedChildName],
        timeout: Duration,
    ) -> Result<()> {
        let deadline = tokio::time::Instant::now() + timeout;

        loop {
            let mut not_ready: Vec<ManagedChildName> = Vec::new();

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
    pub async fn send(&self, process_id: ProcessId, message: impl Into<DataPayload>) -> Result<()> {
        let sender = {
            let processes = self.inner.processes.read().await;
            let entry = processes
                .get(&process_id)
                .ok_or(OrchestratorError::ProcessNotFound(process_id))?;
            entry.data_sender.clone()
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

    /// Returns the latest child-reported status for the managed child.
    ///
    /// The result is `None` until the child sends its first status update. The
    /// returned timestamp is the time recorded by the child when it generated
    /// the update.
    pub async fn child_status(&self, process_id: ProcessId) -> Result<Option<PorkStatusUpdate>> {
        let child_reported_status = {
            let processes = self.inner.processes.read().await;
            processes
                .get(&process_id)
                .ok_or(OrchestratorError::ProcessNotFound(process_id))?
                .child_reported_status
                .clone()
        };

        let status = child_reported_status.lock().await;
        Ok(*status)
    }

    /// Returns the latest child-reported status for the managed process identified by `name`.
    ///
    /// The result is `None` until the child sends its first status update. The
    /// returned timestamp is the time recorded by the child when it generated
    /// the update.
    pub async fn child_status_by_name(
        &self,
        name: &ManagedChildName,
    ) -> Result<Option<PorkStatusUpdate>> {
        let process_id = {
            let process_names = self.inner.process_names.read().await;
            process_names
                .get(name)
                .copied()
                .ok_or_else(|| OrchestratorError::ProcessNameNotFound(name.clone()))?
        };

        self.child_status(process_id).await
    }

    /// Returns the current lifecycle status of the managed process identified by `name`.
    pub async fn process_status_by_name(&self, name: &ManagedChildName) -> Result<PorkChildStatus> {
        let process_id = {
            let process_names = self.inner.process_names.read().await;
            process_names
                .get(name)
                .copied()
                .ok_or_else(|| OrchestratorError::ProcessNameNotFound(name.clone()))?
        };

        self.process_status(process_id).await
    }

    /// Requests a graceful shutdown for the managed process identified by `process_id`.
    ///
    /// The request is codec-encoded and sent only through the dedicated control
    /// channel. This method does not wait for the child to exit or send an OS signal.
    pub async fn request_graceful_shutdown(&self, process_id: ProcessId) -> Result<()> {
        self.request_lifecycle_control(
            process_id,
            PorkChildStatus::Stopping,
            PorkControlMessage::GracefulShutdown,
        )
        .await
    }

    /// Gracefully shuts down a managed process using the orchestrator's default timeout.
    pub async fn graceful_shutdown_process(&self, process_id: ProcessId) -> Result<ExitStatus> {
        self.graceful_shutdown_process_with_timeout(process_id, self.graceful_shutdown_timeout())
            .await
    }

    /// Gracefully shuts down a managed process using an explicit timeout.
    ///
    /// Sends an encoded [`PorkControlMessage::GracefulShutdown`] over the control
    /// channel. If the child does not exit before `timeout` expires, the process
    /// is forcibly stopped.
    pub async fn graceful_shutdown_process_with_timeout(
        &self,
        process_id: ProcessId,
        timeout: Duration,
    ) -> Result<ExitStatus> {
        if self.process_status(process_id).await? != PorkChildStatus::Stopping {
            self.request_graceful_shutdown(process_id).await?;
        }
        self.wait_for_exit_or_stop(process_id, timeout).await
    }

    async fn request_lifecycle_control(
        &self,
        process_id: ProcessId,
        status: PorkChildStatus,
        message: PorkControlMessage,
    ) -> Result<()> {
        self.set_process_status(process_id, status).await?;
        let (sender, codec) = {
            let processes = self.inner.processes.read().await;
            let entry = processes
                .get(&process_id)
                .ok_or(OrchestratorError::ProcessNotFound(process_id))?;
            (entry.control_sender.clone(), entry.control_codec)
        };

        sender.send(ControlPayload::from(codec.encode_control_message(message)?))?;
        Ok(())
    }

    async fn wait_for_exit_or_stop(
        &self,
        process_id: ProcessId,
        timeout: Duration,
    ) -> Result<ExitStatus> {
        match tokio::time::timeout(timeout, self.wait_for_exit(process_id)).await {
            Ok(result) => self.finish_process_shutdown(process_id, result?).await,
            Err(_) => self.stop_process(process_id).await,
        }
    }

    /// Immediately kills, reaps, and removes a managed process.
    pub async fn stop_process(&self, process_id: ProcessId) -> Result<ExitStatus> {
        self.set_process_status(process_id, PorkChildStatus::Stopping)
            .await?;
        let child = self.child_handle(process_id).await?;
        let status = {
            let mut child = child.lock().await;
            match child.try_wait()? {
                Some(status) => status,
                None => {
                    child.start_kill()?;
                    child.wait().await?
                }
            }
        };

        self.finish_process_shutdown(process_id, status).await
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
        shutdown_control_task(entry.control_task_handle).await;

        if let Some(process_name) = &entry.managed_name {
            let mut process_names = self.inner.process_names.write().await;
            process_names.remove(process_name);
        }

        Ok(status)
    }

    async fn wait_for_exit(&self, process_id: ProcessId) -> Result<ExitStatus> {
        let child = self.child_handle(process_id).await?;
        let mut child = child.lock().await;
        Ok(child.wait().await?)
    }

    async fn child_handle(
        &self,
        process_id: ProcessId,
    ) -> Result<Arc<tokio::sync::Mutex<tokio::process::Child>>> {
        let processes = self.inner.processes.read().await;
        processes
            .get(&process_id)
            .map(|entry| entry.child.clone())
            .ok_or(OrchestratorError::ProcessNotFound(process_id))
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
    pub async fn process_id_by_name(&self, name: &ManagedChildName) -> Result<Option<ProcessId>> {
        let process_names = self.inner.process_names.read().await;
        Ok(process_names.get(name).copied())
    }

    /// Returns `true` when a managed process with the given name exists.
    pub async fn has_process_name(&self, name: &ManagedChildName) -> Result<bool> {
        let process_names = self.inner.process_names.read().await;
        Ok(process_names.contains_key(name))
    }

    /// Returns the managed names currently registered with the orchestrator.
    pub async fn process_names(&self) -> Result<Vec<ManagedChildName>> {
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
                next_process_id: AtomicU64::new(ProcessId::new(1).get()),
                graceful_shutdown_timeout: self.graceful_shutdown_timeout,
                dependency_timeout: self.dependency_timeout,
                processes: tokio::sync::RwLock::new(HashMap::new()),
                process_names: tokio::sync::RwLock::new(HashMap::new()),
            }),
        }
    }
}
