/// Child-side bootstrap helpers and shared child process setup.
///
/// This module contains the functions a managed child process uses to:
/// - read bootstrap information from the environment,
/// - resolve the configured control-message codec,
/// - and connect back to the parent over IPC.
pub mod bootstrap;

pub use bootstrap::{
    child_bootstrap_env_value, child_connect, child_connect_from_env, child_control_codec_from_env,
};
