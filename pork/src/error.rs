use std::fmt;

use ipc_channel::IpcError;

/// Convenience result type used throughout the `pork` orchestration API.
pub type Result<T> = std::result::Result<T, OrchestratorError>;

/// Stable identifier assigned to a managed child process.
pub type ProcessId = u64;

/// Errors returned by process orchestration, bootstrap, and IPC operations.
#[derive(Debug)]
pub enum OrchestratorError {
    /// An I/O operation failed while spawning, stopping, or signaling a child process.
    Io(std::io::Error),
    /// An IPC channel operation failed.
    Ipc(IpcError),
    /// No managed process exists for the given process ID.
    ProcessNotFound(ProcessId),
    /// No managed process exists for the given process name.
    ProcessNameNotFound(String),
    /// A process was started with a name that is already registered.
    DuplicateProcessName(String),
    /// The child process could not read its bootstrap environment variable.
    MissingBootstrapValue,
    /// The child process could not read the control codec environment variable.
    MissingControlCodec,
    /// The configured control codec value is not supported.
    UnsupportedControlCodec(String),
    /// Internal shared state was poisoned by a panic while holding a lock.
    LockPoisoned(&'static str),
    /// One or more declared dependencies did not reach `Running` within the
    /// configured timeout. The inner `Vec` contains the names that timed out.
    DependencyTimeout(Vec<String>),
    /// A dependency cycle was detected among the declared `depends_on` names.
    /// The inner `Vec` contains the names that form the cycle.
    DependencyCycle(Vec<String>),
    /// A declared dependency name is not registered with the orchestrator.
    DependencyNotFound(String),
}

impl fmt::Display for OrchestratorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(f, "io error: {error}"),
            Self::Ipc(error) => write!(f, "ipc error: {error}"),
            Self::ProcessNotFound(process_id) => write!(f, "process not found: {process_id}"),
            Self::ProcessNameNotFound(name) => {
                write!(f, "managed process with name '{name}' was not found")
            }
            Self::DuplicateProcessName(name) => {
                write!(f, "managed process with name '{name}' already exists")
            }
            Self::MissingBootstrapValue => write!(f, "missing bootstrap environment value"),
            Self::MissingControlCodec => write!(f, "missing control codec environment value"),
            Self::UnsupportedControlCodec(codec) => {
                write!(f, "unsupported control codec '{codec}'")
            }
            Self::LockPoisoned(name) => write!(f, "lock poisoned: {name}"),
            Self::DependencyTimeout(names) => {
                write!(
                    f,
                    "dependencies did not become ready within the timeout: {}",
                    names.join(", ")
                )
            }
            Self::DependencyCycle(names) => {
                write!(f, "dependency cycle detected among: {}", names.join(", "))
            }
            Self::DependencyNotFound(name) => {
                write!(
                    f,
                    "declared dependency '{name}' is not registered with the orchestrator"
                )
            }
        }
    }
}

impl std::error::Error for OrchestratorError {}

impl From<std::io::Error> for OrchestratorError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<IpcError> for OrchestratorError {
    fn from(value: IpcError) -> Self {
        Self::Ipc(value)
    }
}
