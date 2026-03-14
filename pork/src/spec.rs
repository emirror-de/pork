use std::collections::HashMap;
use std::path::PathBuf;

use pork_proto::PorkControlCodec;

use crate::DEFAULT_BOOTSTRAP_ENV;

/// Configuration used to start and manage a child process.
///
/// `ProcessSpec` is a builder-style type used by [`crate::ProcessOrchestrator`]
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
        }
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
}
