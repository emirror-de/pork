//! Child process specifications and their dedicated builder API.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use pork_proto::protocol::PorkControlCodec;

use crate::types::{
    BootstrapEnvName, HeartbeatInterval, LogFilePath, ManagedChildName, ProcessExecutable,
};
use crate::{CONTROL_BOOTSTRAP_ENV, DEFAULT_BOOTSTRAP_ENV};

/// Default heartbeat interval when status reporting is enabled.
const DEFAULT_HEARTBEAT_INTERVAL: HeartbeatInterval =
    HeartbeatInterval::new(std::time::Duration::from_secs(5));

/// Configuration used to start and manage a child process.
///
/// `ProcessSpec` is the immutable configuration consumed by
/// [`crate::orchestrator::ProcessOrchestrator`] when spawning a managed child.
/// Build it with [`ProcessSpec::builder`] or [`ProcessSpecBuilder::new`].
#[derive(Debug, Clone)]
pub struct ProcessSpec {
    pub(crate) executable: ProcessExecutable,
    pub(crate) managed_name: Option<ManagedChildName>,
    pub(crate) control_codec: PorkControlCodec,
    pub(crate) args: Vec<String>,
    pub(crate) current_dir: Option<PathBuf>,
    pub(crate) env: HashMap<String, String>,
    pub(crate) data_bootstrap_env: BootstrapEnvName,
    pub(crate) control_bootstrap_env: BootstrapEnvName,
    pub(crate) output: ProcessOutput,
    /// Managed names of processes that must be `Running` before this process is spawned.
    pub(crate) depends_on: Vec<ManagedChildName>,
    /// Optional heartbeat interval for automatic status reporting.
    pub(crate) heartbeat_interval: Option<HeartbeatInterval>,
}

/// Output configuration for a managed child process.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum ProcessOutput {
    /// Inherit stdout and stderr from the parent process.
    #[default]
    Inherit,
    /// Pipe selected streams back to the parent process.
    Capture {
        /// Whether stdout is captured.
        stdout: bool,
        /// Whether stderr is captured.
        stderr: bool,
    },
    /// Append stdout and stderr to selected log files.
    Log {
        /// Log file for stdout, if configured.
        stdout: Option<LogFilePath>,
        /// Log file for stderr, if configured.
        stderr: Option<LogFilePath>,
    },
}

/// Managed dependency names required before a child starts.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ManagedChildDependencies(Vec<ManagedChildName>);

impl ManagedChildDependencies {
    /// Creates an empty dependency list.
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the dependencies as a slice.
    pub fn as_slice(&self) -> &[ManagedChildName] {
        &self.0
    }

    /// Adds one dependency name.
    pub fn push(&mut self, name: impl Into<ManagedChildName>) {
        self.0.push(name.into());
    }

    /// Extends the dependency list with more names.
    pub fn extend<I, S>(&mut self, names: I)
    where
        I: IntoIterator<Item = S>,
        S: Into<ManagedChildName>,
    {
        self.0.extend(names.into_iter().map(Into::into));
    }

    /// Returns `true` when no dependencies are configured.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl ProcessSpec {
    /// Creates a builder for a new process specification.
    ///
    /// The resulting builder starts with the same defaults that older inline builder usage had:
    /// no managed name, the default control codec, inherited output, default bootstrap variable
    /// names, and heartbeat reporting disabled.
    pub fn builder(executable: impl Into<ProcessExecutable>) -> ProcessSpecBuilder {
        ProcessSpecBuilder::new(executable)
    }

    pub(crate) fn new_with_defaults(executable: impl Into<ProcessExecutable>) -> Self {
        Self {
            executable: executable.into(),
            managed_name: None,
            control_codec: PorkControlCodec::default(),
            args: Vec::new(),
            current_dir: None,
            env: HashMap::new(),
            data_bootstrap_env: DEFAULT_BOOTSTRAP_ENV.into(),
            control_bootstrap_env: CONTROL_BOOTSTRAP_ENV.into(),
            output: ProcessOutput::Inherit,
            depends_on: Vec::new(),
            heartbeat_interval: None,
        }
    }

    /// Returns the executable path used to spawn the child process.
    pub fn executable(&self) -> &ProcessExecutable {
        &self.executable
    }

    /// Returns the configured managed name, if one was assigned.
    pub fn managed_name(&self) -> Option<&ManagedChildName> {
        self.managed_name.as_ref()
    }

    /// Returns the configured control-message codec.
    pub fn control_codec_ref(&self) -> PorkControlCodec {
        self.control_codec
    }

    /// Returns the configured command-line arguments.
    pub fn args_ref(&self) -> &[String] {
        &self.args
    }

    /// Returns the configured working directory, if one was assigned.
    pub fn current_dir_ref(&self) -> Option<&PathBuf> {
        self.current_dir.as_ref()
    }

    /// Returns the configured child environment overrides.
    pub fn env_ref(&self) -> &HashMap<String, String> {
        &self.env
    }

    /// Returns the environment variable name used for the data-channel bootstrap handshake.
    pub fn data_bootstrap_env_ref(&self) -> &BootstrapEnvName {
        &self.data_bootstrap_env
    }

    /// Returns the environment variable name used for the control-channel bootstrap handshake.
    pub fn control_bootstrap_env_ref(&self) -> &BootstrapEnvName {
        &self.control_bootstrap_env
    }

    /// Returns whether stdout capture is enabled.
    pub fn captures_stdout(&self) -> bool {
        match &self.output {
            ProcessOutput::Capture { stdout, .. } => *stdout,
            _ => false,
        }
    }

    /// Returns whether stderr capture is enabled.
    pub fn captures_stderr(&self) -> bool {
        match &self.output {
            ProcessOutput::Capture { stderr, .. } => *stderr,
            _ => false,
        }
    }

    /// Returns the append-only logfile configured for stdout, if any.
    pub fn stdout_log_ref(&self) -> Option<&Path> {
        match &self.output {
            ProcessOutput::Log { stdout, .. } => stdout.as_ref().map(LogFilePath::as_path),
            _ => None,
        }
    }

    /// Returns the append-only logfile configured for stderr, if any.
    pub fn stderr_log_ref(&self) -> Option<&Path> {
        match &self.output {
            ProcessOutput::Log { stderr, .. } => stderr.as_ref().map(LogFilePath::as_path),
            _ => None,
        }
    }

    /// Returns the configured heartbeat interval, if status reporting is enabled.
    pub fn heartbeat_interval_ref(&self) -> Option<HeartbeatInterval> {
        self.heartbeat_interval
    }

    /// Returns the managed names this process depends on.
    pub fn dependencies_ref(&self) -> &[ManagedChildName] {
        &self.depends_on
    }

    /// Returns the managed dependency list for this process.
    pub fn dependencies(&self) -> ManagedChildDependencies {
        ManagedChildDependencies(self.depends_on.clone())
    }
}

pub(crate) fn default_heartbeat_interval() -> HeartbeatInterval {
    DEFAULT_HEARTBEAT_INTERVAL
}

/// Builder for [`ProcessSpec`].
///
/// This keeps the mutating configuration API separate from the immutable process
/// specification consumed by the orchestrator.
#[derive(Debug, Clone)]
pub struct ProcessSpecBuilder {
    spec: ProcessSpec,
}

impl ProcessSpecBuilder {
    /// Creates a new builder for the given executable path.
    pub fn new(executable: impl Into<ProcessExecutable>) -> Self {
        Self {
            spec: ProcessSpec::new_with_defaults(executable),
        }
    }

    /// Assigns a stable managed name to the child process.
    pub fn managed_name(mut self, value: impl Into<ManagedChildName>) -> Self {
        self.spec.managed_name = Some(value.into());
        self
    }

    /// Removes any previously assigned managed name.
    pub fn without_managed_name(mut self) -> Self {
        self.spec.managed_name = None;
        self
    }

    /// Selects the control-message codec used between host and child.
    pub fn control_codec(mut self, value: PorkControlCodec) -> Self {
        self.spec.control_codec = value;
        self
    }

    /// Appends a single command-line argument.
    pub fn arg(mut self, value: impl Into<String>) -> Self {
        self.spec.args.push(value.into());
        self
    }

    /// Appends multiple command-line arguments in order.
    pub fn args<I, S>(mut self, values: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.spec.args.extend(values.into_iter().map(Into::into));
        self
    }

    /// Sets the working directory for the child process.
    pub fn current_dir(mut self, value: impl Into<PathBuf>) -> Self {
        self.spec.current_dir = Some(value.into());
        self
    }

    /// Adds or overrides an environment variable for the child process.
    pub fn env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.spec.env.insert(key.into(), value.into());
        self
    }

    /// Extends the child environment with multiple key-value pairs.
    pub fn envs<I, K, V>(mut self, values: I) -> Self
    where
        I: IntoIterator<Item = (K, V)>,
        K: Into<String>,
        V: Into<String>,
    {
        self.spec.env.extend(
            values
                .into_iter()
                .map(|(key, value)| (key.into(), value.into())),
        );
        self
    }

    /// Overrides the environment variable name used for the data-channel bootstrap handshake.
    pub fn data_bootstrap_env(mut self, value: impl Into<BootstrapEnvName>) -> Self {
        self.spec.data_bootstrap_env = value.into();
        self
    }

    /// Overrides the environment variable name used for the control-channel bootstrap handshake.
    pub fn control_bootstrap_env(mut self, value: impl Into<BootstrapEnvName>) -> Self {
        self.spec.control_bootstrap_env = value.into();
        self
    }

    /// Restores inherited stdout and stderr instead of logging them.
    pub fn inherit_output(mut self) -> Self {
        self.spec.output = ProcessOutput::Inherit;
        self
    }

    /// Appends stdout to a logfile that can be inspected while the child is running.
    pub fn log_stdout(mut self, path: impl Into<LogFilePath>) -> Self {
        let stderr = match &self.spec.output {
            ProcessOutput::Log { stderr, .. } => stderr.clone(),
            _ => None,
        };
        self.spec.output = ProcessOutput::Log {
            stdout: Some(path.into()),
            stderr,
        };
        self
    }

    /// Appends stderr to a logfile that can be inspected while the child is running.
    pub fn log_stderr(mut self, path: impl Into<LogFilePath>) -> Self {
        let stdout = match &self.spec.output {
            ProcessOutput::Log { stdout, .. } => stdout.clone(),
            _ => None,
        };
        self.spec.output = ProcessOutput::Log {
            stdout,
            stderr: Some(path.into()),
        };
        self
    }

    /// Appends both stdout and stderr to one logfile.
    pub fn log_output(mut self, path: impl Into<LogFilePath>) -> Self {
        let path = path.into();
        self.spec.output = ProcessOutput::Log {
            stdout: Some(path.clone()),
            stderr: Some(path),
        };
        self
    }

    /// Declares that this process depends on the named process.
    pub fn depends_on(mut self, name: impl Into<ManagedChildName>) -> Self {
        self.spec.depends_on.push(name.into());
        self
    }

    /// Declares that this process depends on all of the given named processes.
    pub fn depends_on_all<I, S>(mut self, names: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<ManagedChildName>,
    {
        self.spec
            .depends_on
            .extend(names.into_iter().map(Into::into));
        self
    }

    /// Enables automatic heartbeat-based status reporting with a custom interval.
    ///
    /// # Example
    ///
    /// ```rust
    /// use std::time::Duration;
    ///
    /// use pork::orchestrator::spec::ProcessSpec;
    /// use pork::types::HeartbeatInterval;
    ///
    /// let spec = ProcessSpec::builder("./my-child")
    ///     // Choose a shorter interval when the host needs faster progress visibility.
    ///     .enable_heartbeat(Duration::from_secs(5))
    ///     .build();
    ///
    /// assert_eq!(
    ///     spec.heartbeat_interval_ref(),
    ///     Some(HeartbeatInterval::new(Duration::from_secs(5)))
    /// );
    /// ```
    pub fn enable_heartbeat(mut self, interval: impl Into<HeartbeatInterval>) -> Self {
        self.spec.heartbeat_interval = Some(interval.into());
        self
    }

    /// Enables automatic heartbeat-based status reporting with the default 5-second interval.
    ///
    /// # Example
    ///
    /// ```rust
    /// use std::time::Duration;
    ///
    /// use pork::orchestrator::spec::ProcessSpec;
    /// use pork::types::HeartbeatInterval;
    ///
    /// let spec = ProcessSpec::builder("./my-child")
    ///     // The default heartbeat keeps status traffic light while still proving liveness.
    ///     .with_heartbeat()
    ///     .build();
    ///
    /// assert_eq!(
    ///     spec.heartbeat_interval_ref(),
    ///     Some(HeartbeatInterval::new(Duration::from_secs(5)))
    /// );
    /// ```
    pub fn with_heartbeat(mut self) -> Self {
        self.spec.heartbeat_interval = Some(default_heartbeat_interval());
        self
    }

    /// Disables automatic heartbeat-based status reporting.
    pub fn without_heartbeat(mut self) -> Self {
        self.spec.heartbeat_interval = None;
        self
    }

    /// Finalizes the builder and returns the immutable process specification.
    pub fn build(self) -> ProcessSpec {
        self.spec
    }
}

impl From<ProcessSpecBuilder> for ProcessSpec {
    fn from(value: ProcessSpecBuilder) -> Self {
        value.build()
    }
}
