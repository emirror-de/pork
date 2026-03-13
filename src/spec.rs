use std::collections::HashMap;
use std::path::PathBuf;

use crate::DEFAULT_BOOTSTRAP_ENV;

#[derive(Debug, Clone)]
pub struct ProcessSpec {
    pub(crate) executable: PathBuf,
    pub(crate) args: Vec<String>,
    pub(crate) current_dir: Option<PathBuf>,
    pub(crate) env: HashMap<String, String>,
    pub(crate) bootstrap_env: String,
    pub(crate) capture_stdout: bool,
    pub(crate) capture_stderr: bool,
}

impl ProcessSpec {
    pub fn new(executable: impl Into<PathBuf>) -> Self {
        Self {
            executable: executable.into(),
            args: Vec::new(),
            current_dir: None,
            env: HashMap::new(),
            bootstrap_env: DEFAULT_BOOTSTRAP_ENV.to_owned(),
            capture_stdout: false,
            capture_stderr: false,
        }
    }

    pub fn arg(mut self, value: impl Into<String>) -> Self {
        self.args.push(value.into());
        self
    }

    pub fn args<I, S>(mut self, values: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.args.extend(values.into_iter().map(Into::into));
        self
    }

    pub fn current_dir(mut self, value: impl Into<PathBuf>) -> Self {
        self.current_dir = Some(value.into());
        self
    }

    pub fn env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.env.insert(key.into(), value.into());
        self
    }

    pub fn bootstrap_env(mut self, value: impl Into<String>) -> Self {
        self.bootstrap_env = value.into();
        self
    }

    pub fn capture_stdout(mut self, value: bool) -> Self {
        self.capture_stdout = value;
        self
    }

    pub fn capture_stderr(mut self, value: bool) -> Self {
        self.capture_stderr = value;
        self
    }
}
