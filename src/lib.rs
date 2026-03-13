mod child;
mod error;
mod ipc;
mod orchestrator;
mod spec;

pub use child::{
    child_bootstrap_env_value, child_connect, child_connect_from_env, graceful_shutdown_message,
    is_graceful_shutdown_message,
};
pub use error::{OrchestratorError, ProcessId, Result};
pub use orchestrator::{ManagedChild, ProcessOrchestrator};
pub use pork_proto::{PorkControlMessage, PorkIpcMessage};
pub use spec::ProcessSpec;

pub const DEFAULT_BOOTSTRAP_ENV: &str = "PORK_IPC_BOOTSTRAP";
