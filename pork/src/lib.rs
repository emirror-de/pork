//! Process orchestration for building host/child IPC workflows on top of `ipc-channel`.
//!
//! `pork` is the high-level crate in the Pork workspace. It helps you:
//!
//! - spawn and track child processes,
//! - establish the bootstrap handshake between host and child,
//! - exchange raw IPC payloads,
//! - and gracefully shut children down with a shared control protocol.
//!
//! The crate re-exports the core control-plane types from `pork-proto`, so most applications
//! can depend on `pork` alone when building a supervisor and a managed child binary.
//!
//! # What the crate-level attributes mean
//!
//! This crate opts into strict documentation and linting at the crate level:
//!
//! - `#![deny(missing_docs)]` requires public API documentation.
//! - `#![deny(clippy::unwrap_used)]` rejects `unwrap()` in linted code paths.
//! - `#![deny(clippy::expect_used)]` rejects `expect()` in linted code paths.
//!
//! For you as a user, that means the public API is intended to stay discoverable and the
//! examples favor explicit error handling over shortcuts.
//!
//! # Typical architecture
//!
//! A Pork-based setup usually has two sides:
//!
//! 1. A **host** process that creates a [`ProcessOrchestrator`] and starts one or more children.
//! 2. A **child** process that reads bootstrap configuration from the environment and connects
//!    back to the host with [`child_connect_from_env`].
//!
//! The host sends and receives raw `Vec<u8>` payloads. If you want a shared envelope for custom
//! messages plus framework control messages, use [`PorkIpcMessage`] from the re-exported
//! protocol module.
//!
//! # Quick start
//!
//! ```no_run
//! use pork::{ProcessOrchestrator, ProcessSpec};
//!
//! let orchestrator = ProcessOrchestrator::new();
//!
//! let spec = ProcessSpec::new("./my-child-binary")
//!     .managed_name("worker")
//!     .arg("--serve");
//!
//! let child = orchestrator.start_process(spec)?;
//! # let _ = child;
//! # Ok::<(), pork::OrchestratorError>(())
//! ```
//!
//! # Host example
//!
//! This example starts a child, sends one raw message, and then requests a graceful shutdown.
//!
//! ```no_run
//! use pork::{ProcessOrchestrator, ProcessSpec};
//!
//! let orchestrator = ProcessOrchestrator::new();
//!
//! let child = orchestrator.start_process(
//!     ProcessSpec::new("./child-binary")
//!         .managed_name("example-child")
//!         .capture_stdout(true)
//!         .capture_stderr(true),
//! )?;
//!
//! child.send(b"ping".to_vec())?;
//! let _exit_status = orchestrator.graceful_shutdown_process(child.id())?;
//! # Ok::<(), pork::OrchestratorError>(())
//! ```
//!
//! # Child example
//!
//! In the managed child process, connect back to the host by reading the bootstrap value from
//! the configured environment variable.
//!
//! ```no_run
//! use pork::{DEFAULT_BOOTSTRAP_ENV, child_connect_from_env};
//!
//! let (from_host, to_host) = child_connect_from_env(DEFAULT_BOOTSTRAP_ENV)?;
//! # let _ = (from_host, to_host);
//! # Ok::<(), pork::OrchestratorError>(())
//! ```
//!
//! # Using a specific control codec
//!
//! The orchestrator automatically exports the selected control codec to the child process via
//! [`PORK_CONTROL_CODEC_ENV`]. If you want to force a specific codec for control messages, set
//! it on the [`ProcessSpec`] before starting the process.
//!
//! ```no_run
//! use pork::{PorkControlCodec, ProcessOrchestrator, ProcessSpec};
//!
//! let orchestrator = ProcessOrchestrator::new();
//!
//! let spec = ProcessSpec::new("./child-binary")
//!     .managed_name("postcard-child")
//!     .control_codec(PorkControlCodec::Postcard);
//!
//! let _child = orchestrator.start_process(spec)?;
//! # Ok::<(), pork::OrchestratorError>(())
//! ```
//!
//! # Receiving messages asynchronously
//!
//! [`ManagedChild::recv`] is async and integrates naturally into a Tokio application.
//!
//! ```no_run
//! use pork::{ProcessOrchestrator, ProcessSpec};
//!
//! fn main() -> Result<(), pork::OrchestratorError> {
//!     let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should build");
//!     runtime.block_on(async {
//!         let orchestrator = ProcessOrchestrator::new();
//!         let child = orchestrator.start_process(ProcessSpec::new("./child-binary"))?;
//!
//!         if let Some(message) = child.recv().await {
//!             let _ = message;
//!         }
//!
//!         let _exit_status = orchestrator.graceful_shutdown_process(child.id())?;
//!         Ok(())
//!     })
//! }
//! ```
//!
//! # API guide
//!
//! - [`ProcessOrchestrator`] manages child lifecycle and process lookup.
//! - [`ProcessSpec`] configures how a child process is started.
//! - [`ManagedChild`] provides a handle for messaging and child identity.
//! - [`child_connect_from_env`] and [`child_connect`] are the child-side bootstrap helpers.
//! - Re-exported protocol items such as [`PorkControlCodec`] and [`PorkIpcMessage`] let you
//!   share the same control-plane contract across host and child binaries.
//! - Lower-level protocol helper functions and codec modules remain available from `pork-proto`
//!   when you want a narrower dependency on just the shared protocol layer.
#![deny(missing_docs)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(unsafe_code)]

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
    PORK_CONTROL_CODEC_ENV, ParsePorkControlCodecError, PorkControlCodec, PorkControlMessage,
    PorkIpcMessage,
};
pub use spec::ProcessSpec;

/// Default environment variable name used to pass the bootstrap handshake value
/// from the host process to a managed child.
pub const DEFAULT_BOOTSTRAP_ENV: &str = "PORK_IPC_BOOTSTRAP";
