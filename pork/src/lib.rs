//! Process orchestration for building host/child IPC workflows on top of `ipc-channel`.
//!
//! `pork` is the high-level crate in the Pork workspace. It helps you:
//!
//! - spawn and track child processes,
//! - establish the bootstrap handshake between host and child,
//! - exchange raw IPC payloads,
//! - and gracefully shut children down with a shared control protocol.
//!
//! The crate exposes explicit domain modules so the public API stays navigable:
//!
//! - [`orchestrator`] for process lifecycle management,
//! - [`spec`] for child process configuration,
//! - [`child`] for child-side bootstrap helpers,
//! - [`error`] for orchestration error types.
//!
//! Pair this crate with the companion `pork-proto` crate when you want the
//! shared control-plane types and codec implementations.
//!
//! # Typical architecture
//!
//! A Pork-based setup usually has two sides:
//!
//! 1. A **host** process that creates an [`orchestrator::ProcessOrchestrator`] and starts one or more children.
//! 2. A **child** process that reads bootstrap configuration from the environment and connects
//!    back to the host with [`child::bootstrap::child_connect_from_env`].
//!
//! The host sends and receives raw `Vec<u8>` payloads. If you want a shared envelope for custom
//! messages plus framework control messages, use `pork_proto::protocol::PorkIpcMessage`.
//!
//! # Quick start
//!
//! ```no_run
//! use pork::orchestrator::ProcessOrchestrator;
//! use pork::spec::ProcessSpec;
//!
//! let orchestrator = ProcessOrchestrator::new();
//!
//! let spec = ProcessSpec::new("./my-child-binary")
//!     .managed_name("worker")
//!     .arg("--serve");
//!
//! let child = orchestrator.start_process(spec)?;
//! # let _ = child;
//! # Ok::<(), pork::error::OrchestratorError>(())
//! ```
//!
//! # Host example
//!
//! This example starts a child, sends one raw message, and then requests a graceful shutdown.
//!
//! ```no_run
//! use pork::orchestrator::ProcessOrchestrator;
//! use pork::spec::ProcessSpec;
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
//! # Ok::<(), pork::error::OrchestratorError>(())
//! ```
//!
//! # Child example
//!
//! In the managed child process, connect back to the host by reading the bootstrap value from
//! the configured environment variable.
//!
//! ```no_run
//! use pork::child::bootstrap::child_connect_from_env;
//! use pork::DEFAULT_BOOTSTRAP_ENV;
//!
//! let (from_host, to_host) = child_connect_from_env(DEFAULT_BOOTSTRAP_ENV)?;
//! # let _ = (from_host, to_host);
//! # Ok::<(), pork::error::OrchestratorError>(())
//! ```
//!
//! # Using a specific control codec
//!
//! The orchestrator automatically exports the selected control codec to the child process via
//! `pork_proto::protocol::PORK_CONTROL_CODEC_ENV`. If you want to force a specific codec for
//! control messages, set it on the [`spec::ProcessSpec`] before starting the process.
//!
//! ```no_run
//! use pork::orchestrator::ProcessOrchestrator;
//! use pork::spec::ProcessSpec;
//! use pork_proto::protocol::PorkControlCodec;
//!
//! let orchestrator = ProcessOrchestrator::new();
//!
//! let spec = ProcessSpec::new("./child-binary")
//!     .managed_name("postcard-child")
//!     .control_codec(PorkControlCodec::Postcard);
//!
//! let _child = orchestrator.start_process(spec)?;
//! # Ok::<(), pork::error::OrchestratorError>(())
//! ```
//!
//! # Receiving messages asynchronously
//!
//! [`orchestrator::ManagedChild::recv`] is async and integrates naturally into a Tokio application.
//!
//! ```no_run
//! use pork::orchestrator::ProcessOrchestrator;
//! use pork::spec::ProcessSpec;
//!
//! fn main() -> Result<(), pork::error::OrchestratorError> {
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
//! - [`orchestrator::ProcessOrchestrator`] manages child lifecycle and process lookup.
//! - [`spec::ProcessSpec`] configures how a child process is started.
//! - [`orchestrator::ManagedChild`] provides a handle for messaging and child identity.
//! - [`child::bootstrap::child_connect_from_env`] and [`child::bootstrap::child_connect`] are the child-side bootstrap helpers.
//! - The companion `pork-proto` crate contains the shared control-plane contract and
//!   codec marker types.
#![deny(missing_docs)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(unsafe_code)]

/// Child-side bootstrap helpers and shared child process constants.
pub mod child;
/// Error types and convenience aliases used by the orchestration API.
pub mod error;
mod ipc;
/// Host-side process lifecycle management types.
pub mod orchestrator;
/// Child process configuration types and builders.
pub mod spec;

/// Default environment variable name used to pass the bootstrap handshake value
/// from the host process to a managed child.
pub const DEFAULT_BOOTSTRAP_ENV: &str = "PORK_IPC_BOOTSTRAP";
