mod child;
mod error;
mod ipc;
mod orchestrator;
mod spec;

pub use child::bootstrap::{
    child_bootstrap_env_value, child_connect, child_connect_from_env, child_control_codec_from_env,
};
pub use error::{OrchestratorError, ProcessId, Result};
pub use orchestrator::{ManagedChild, ProcessOrchestrator, ProcessOrchestratorBuilder};
pub use pork_proto::{
    PORK_CONTROL_CODEC_ENV, ParsePorkControlCodecError, PorkCodec, PorkControlCodec,
    PorkControlMessage, PorkIpcMessage, PorkProtoCodecError, control_codec_from_env,
    decode_control_message, encode_control_message, graceful_shutdown_message,
    is_graceful_shutdown_message, json as proto_json, postcard as proto_postcard,
};
pub use spec::ProcessSpec;

pub const DEFAULT_BOOTSTRAP_ENV: &str = "PORK_IPC_BOOTSTRAP";
