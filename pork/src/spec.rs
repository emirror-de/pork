use std::collections::HashMap;
use std::path::PathBuf;

use pork_proto::protocol::PorkControlCodec;

use crate::DEFAULT_BOOTSTRAP_ENV;

/// Configuration used to start and manage a child process.
///
/// `ProcessSpec` is a builder-style type used by [`crate::orchestrator::ProcessOrchestrator`]
/// to describe how a child process should be spawned, named, and connected back
/// to the host process.
#[derive(Debug, Clone)]
pub struct ProcessSpec {
    pub(crate) executable: PathBuf,
    pub(crate) managed_name: Option<String>,
    pub(crate) control_codec: PorkControlCodec,
    pub(crate) args: Vec<String>,
    pub(crate) current_dir: Option<PathBuf>,
    pub(crate) env: HashMap<String, String>,
    pub(crate) bootstrap_env: String,
    pub(crate) capture_stdout: bool,
    pub(crate) capture_stderr: bool,
    /// Managed names of processes that must be [`PorkChildStatus::Running`] before
    /// this process is spawned. Dependencies are specified using [`Self::depends_on`]
    /// or [`Self::depends_on_all`]. All names must be registered with the same
    /// [`crate::orchestrator::ProcessOrchestrator`].
    pub(crate) depends_on: Vec<String>,
}

impl ProcessSpec {
    /// Creates a new process specification for the given executable path.
    ///
    /// By default, the process has no managed name, uses the default control
    /// codec, inherits the default bootstrap environment variable name, and
    /// does not capture stdout or stderr.
    pub fn new(executable: impl Into<PathBuf>) -> Self {
        Self {
            executable: executable.into(),
            managed_name: None,
            control_codec: PorkControlCodec::default(),
            args: Vec::new(),
            current_dir: None,
            env: HashMap::new(),
            bootstrap_env: DEFAULT_BOOTSTRAP_ENV.to_owned(),
            capture_stdout: false,
            capture_stderr: false,
            depends_on: Vec::new(),
        }
    }

    /// Returns the executable path used to spawn the child process.
    pub fn executable(&self) -> &PathBuf {
        &self.executable
    }

    /// Returns the configured managed name, if one was assigned.
    pub fn managed_name_ref(&self) -> Option<&str> {
        self.managed_name.as_deref()
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

    /// Returns the environment variable name used for the bootstrap handshake.
    pub fn bootstrap_env_ref(&self) -> &str {
        &self.bootstrap_env
    }

    /// Returns whether stdout capture is enabled.
    pub fn captures_stdout(&self) -> bool {
        self.capture_stdout
    }

    /// Returns whether stderr capture is enabled.
    pub fn captures_stderr(&self) -> bool {
        self.capture_stderr
    }

    /// Assigns a stable managed name to the child process.
    ///
    /// Managed names allow you to look up and restart processes by name through
    /// the orchestrator.
    pub fn managed_name(mut self, value: impl Into<String>) -> Self {
        self.managed_name = Some(value.into());
        self
    }

    /// Removes any previously assigned managed name.
    pub fn without_managed_name(mut self) -> Self {
        self.managed_name = None;
        self
    }

    /// Selects the control-message codec used between host and child.
    pub fn control_codec(mut self, value: PorkControlCodec) -> Self {
        self.control_codec = value;
        self
    }

    /// Appends a single command-line argument.
    pub fn arg(mut self, value: impl Into<String>) -> Self {
        self.args.push(value.into());
        self
    }

    /// Appends multiple command-line arguments in order.
    pub fn args<I, S>(mut self, values: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.args.extend(values.into_iter().map(Into::into));
        self
    }

    /// Sets the working directory for the child process.
    pub fn current_dir(mut self, value: impl Into<PathBuf>) -> Self {
        self.current_dir = Some(value.into());
        self
    }

    /// Adds or overrides an environment variable for the child process.
    pub fn env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.env.insert(key.into(), value.into());
        self
    }

    /// Extends the child environment with multiple key-value pairs.
    pub fn envs<I, K, V>(mut self, values: I) -> Self
    where
        I: IntoIterator<Item = (K, V)>,
        K: Into<String>,
        V: Into<String>,
    {
        self.env.extend(
            values
                .into_iter()
                .map(|(key, value)| (key.into(), value.into())),
        );
        self
    }

    /// Overrides the environment variable name used for bootstrap handshake data.
    ///
    /// The default value is [`DEFAULT_BOOTSTRAP_ENV`].
    pub fn bootstrap_env(mut self, value: impl Into<String>) -> Self {
        self.bootstrap_env = value.into();
        self
    }

    /// Enables or disables stdout capture for the child process.
    pub fn capture_stdout(mut self, value: bool) -> Self {
        self.capture_stdout = value;
        self
    }

    /// Enables or disables stderr capture for the child process.
    pub fn capture_stderr(mut self, value: bool) -> Self {
        self.capture_stderr = value;
        self
    }

    /// Enables both stdout and stderr capture.
    pub fn capture_output(mut self) -> Self {
        self.capture_stdout = true;
        self.capture_stderr = true;
        self
    }

    /// Disables both stdout and stderr capture.
    pub fn without_output_capture(mut self) -> Self {
        self.capture_stdout = false;
        self.capture_stderr = false;
        self
    }

    /// Returns the managed names this process depends on.
    ///
    /// All named processes must be [`pork_proto::protocol::PorkChildStatus::Running`]
    /// before this process is spawned.
    pub fn depends_on_ref(&self) -> &[String] {
        &self.depends_on
    }

    /// Declares that this process depends on the named process.
    ///
    /// The orchestrator will wait for every declared dependency to reach
    /// [`pork_proto::protocol::PorkChildStatus::Running`] before spawning this
    /// process. Dependencies are identified by their managed name.
    pub fn depends_on(mut self, name: impl Into<String>) -> Self {
        self.depends_on.push(name.into());
        self
    }

    /// Declares that this process depends on all of the given named processes.
    ///
    /// Each name must correspond to a managed process registered with the same
    /// [`crate::orchestrator::ProcessOrchestrator`].
    pub fn depends_on_all<I, S>(mut self, names: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.depends_on.extend(names.into_iter().map(Into::into));
        self
    }
}
