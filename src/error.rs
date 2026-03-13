use std::fmt;

use ipc_channel::IpcError;

pub type Result<T> = std::result::Result<T, OrchestratorError>;
pub type ProcessId = u64;

#[derive(Debug)]
pub enum OrchestratorError {
    Io(std::io::Error),
    Ipc(IpcError),
    ProcessNotFound(ProcessId),
    DuplicateProcessName(String),
    MissingBootstrapValue,
    LockPoisoned(&'static str),
}

impl fmt::Display for OrchestratorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(f, "io error: {error}"),
            Self::Ipc(error) => write!(f, "ipc error: {error}"),
            Self::ProcessNotFound(process_id) => write!(f, "process not found: {process_id}"),
            Self::DuplicateProcessName(name) => {
                write!(f, "managed process with name '{name}' already exists")
            }
            Self::MissingBootstrapValue => write!(f, "missing bootstrap environment value"),
            Self::LockPoisoned(name) => write!(f, "lock poisoned: {name}"),
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
