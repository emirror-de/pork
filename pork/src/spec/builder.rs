use std::path::PathBuf;

use pork_proto::protocol::PorkControlCodec;

use super::{ProcessOutput, ProcessSpec, default_heartbeat_interval};
use crate::types::{
    BootstrapEnvName, HeartbeatInterval, LogFilePath, ManagedChildName, ProcessExecutable,
};

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

    /// Enables or disables stdout capture for the child process.
    pub fn capture_stdout(mut self, value: bool) -> Self {
        let stderr = match &self.spec.output {
            ProcessOutput::Capture { stderr, .. } => *stderr,
            _ => false,
        };
        self.spec.output = ProcessOutput::Capture {
            stdout: value,
            stderr,
        };
        self
    }

    /// Enables or disables stderr capture for the child process.
    pub fn capture_stderr(mut self, value: bool) -> Self {
        let stdout = match &self.spec.output {
            ProcessOutput::Capture { stdout, .. } => *stdout,
            _ => false,
        };
        self.spec.output = ProcessOutput::Capture {
            stdout,
            stderr: value,
        };
        self
    }

    /// Enables both stdout and stderr capture.
    pub fn capture_output(mut self) -> Self {
        self.spec.output = ProcessOutput::Capture {
            stdout: true,
            stderr: true,
        };
        self
    }

    /// Restores inherited stdout and stderr instead of capturing or logging them.
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
    /// use pork::spec::ProcessSpec;
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
    /// use pork::spec::ProcessSpec;
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
