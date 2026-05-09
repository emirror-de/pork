use std::fmt;

use ipc_channel::IpcError;
use pork_proto::protocol::PorkProtoCodecError;

use crate::types::{BootstrapEnvName, ManagedChildName, ProcessId};

/// Convenience result type used throughout the `pork` orchestration API.
pub type Result<T> = std::result::Result<T, OrchestratorError>;

/// Errors returned by the process orchestration API.
#[derive(Debug)]
pub enum OrchestratorError {
    /// I/O error.
    Io(std::io::Error),
    /// IPC error.
    Ipc(IpcError),
    /// Child process with the given id was not found.
    ProcessNotFound(ProcessId),
    /// Child process with the given managed name was not found.
    ProcessNameNotFound(ManagedChildName),
    /// A managed name was already registered for another child process.
    DuplicateProcessName(ManagedChildName),
    /// Required bootstrap environment variable was missing.
    MissingBootstrapEnv(BootstrapEnvName),
    /// Required bootstrap value was expected but not present.
    MissingBootstrapValue,
    /// Required control codec environment variable was missing.
    MissingControlCodec,
    /// The provided control codec value is not supported.
    UnsupportedControlCodec(String),
    /// The configured control codec could not encode or decode a control message.
    ControlCodec(PorkProtoCodecError),
    /// Timed out waiting for a child process to shut down gracefully.
    GracefulShutdownTimeout(ProcessId),
    /// Timed out waiting for one or more dependencies to become ready.
    DependencyTimeout(Vec<ManagedChildName>),
    /// A declared dependency is not known to the orchestrator.
    UnknownDependency(ManagedChildName),
    /// A declared dependency name was not found in the registry.
    DependencyNotFound(ManagedChildName),
    /// A dependency cycle was detected in the managed-child graph.
    DependencyCycle(Vec<ManagedChildName>),
}

impl fmt::Display for OrchestratorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(f, "io error: {error}"),
            Self::Ipc(error) => write!(f, "ipc error: {error}"),
            Self::ProcessNotFound(process_id) => {
                write!(f, "process with id {process_id} not found")
            }
            Self::ProcessNameNotFound(name) => write!(f, "process with name {name} not found"),
            Self::DuplicateProcessName(name) => {
                write!(f, "process with name {name} already exists")
            }
            Self::MissingBootstrapEnv(name) => {
                write!(f, "missing bootstrap environment variable {name}")
            }
            Self::MissingBootstrapValue => write!(f, "missing bootstrap value"),
            Self::MissingControlCodec => write!(f, "missing control codec"),
            Self::UnsupportedControlCodec(value) => {
                write!(f, "unsupported control codec: {value}")
            }
            Self::ControlCodec(error) => write!(f, "control codec error: {error}"),
            Self::GracefulShutdownTimeout(process_id) => {
                write!(f, "timed out waiting for process {process_id} to shut down")
            }
            Self::DependencyTimeout(names) => {
                write!(f, "timed out waiting for dependencies: {names:?}")
            }
            Self::UnknownDependency(name) => write!(f, "unknown dependency: {name}"),
            Self::DependencyNotFound(name) => write!(f, "dependency not found: {name}"),
            Self::DependencyCycle(names) => {
                write!(f, "dependency cycle detected involving {names:?}")
            }
        }
    }
}

impl std::error::Error for OrchestratorError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            Self::Ipc(error) => Some(error),
            Self::ControlCodec(error) => Some(error),
            Self::ProcessNotFound(_)
            | Self::ProcessNameNotFound(_)
            | Self::DuplicateProcessName(_)
            | Self::MissingBootstrapEnv(_)
            | Self::MissingBootstrapValue
            | Self::MissingControlCodec
            | Self::UnsupportedControlCodec(_)
            | Self::GracefulShutdownTimeout(_)
            | Self::DependencyTimeout(_)
            | Self::UnknownDependency(_)
            | Self::DependencyNotFound(_)
            | Self::DependencyCycle(_) => None,
        }
    }
}

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

impl From<PorkProtoCodecError> for OrchestratorError {
    fn from(value: PorkProtoCodecError) -> Self {
        Self::ControlCodec(value)
    }
}
