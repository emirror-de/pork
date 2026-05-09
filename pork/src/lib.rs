//! Process orchestration for building host/child IPC workflows on top of `ipc-channel`.
//!
//! `pork` is a process orchestration library that helps you:
//!
//! - Spawn and track child processes
//! - Establish dual IPC channels (control and data) between host and child
//! - Exchange typed data-plane payloads or structured IPC envelopes
//! - Send encoded lifecycle control messages (`GracefulShutdown`, `Restart`) over a dedicated control channel
//!
//! # Feature flags
//!
//! This crate uses complementary feature flags to separate host and child concerns:
//!
//! - `host` (enabled by default): includes host-side orchestration
//! - `client`: includes child-side bootstrap logic
//! - `codec-json` (enabled by default): JSON codec support via `pork-proto`
//! - `codec-postcard`: Postcard codec support via `pork-proto`
//!
//! Both `host` and `client` features can be enabled in the same binary for testing,
//! but typical deployments use `host` in parent binaries and `client` in child binaries.
//!
//! # Architecture
//!
//! Pork uses a dual-channel bootstrap architecture:
//!
//! 1. **Host-side setup**: Creates one-shot servers for data and control handshakes
//! 2. **Child spawn**: Passes both server names through environment variables
//! 3. **Data handshake**: Child connects and exchanges application channel endpoints
//! 4. **Control handshake**: Child connects and exchanges framework channel endpoints
//! 5. **Full bidirectional**: Both channels are ready for messaging
//! 6. **Independent receive loops**: Child data and control reception run in separate workers
//!
//! # Typical usage: Host side
//!
//! ```rust
//! # #[cfg(feature = "host")]
//! # {
//! use pork::orchestrator::ProcessOrchestrator;
//! use pork::spec::ProcessSpec;
//!
//! // Configure host-side orchestration timeouts before spawning children.
//! let orchestrator = ProcessOrchestrator::builder()
//!     .graceful_shutdown_timeout(std::time::Duration::from_secs(2))
//!     .dependency_timeout(std::time::Duration::from_secs(10))
//!     .build();
//!
//! // Describe how the child should be started and identified.
//! let spec = ProcessSpec::builder("./child-binary")
//!     // Log files also use dedicated wrapper types internally.
//!     .managed_name("example-child")
//!     .log_output("./child.log")
//!     .build();
//!
//! assert_eq!(orchestrator.graceful_shutdown_timeout(), std::time::Duration::from_secs(2));
//! assert_eq!(orchestrator.dependency_timeout(), std::time::Duration::from_secs(10));
//! assert_eq!(
//!     spec.managed_name().map(pork::types::ManagedChildName::as_str),
//!     Some("example-child")
//! );
//! # }
//! ```
//!
//! # Typical usage: Child side
//!
//! ```rust
//! # #[cfg(feature = "client")]
//! # fn main() -> Result<(), pork::error::OrchestratorError> {
//! use pork::child::bootstrap::ChildBootstrap;
//! use pork::{CONTROL_BOOTSTRAP_ENV, DEFAULT_BOOTSTRAP_ENV};
//!
//! // Environment variables are set by host before spawning.
//! unsafe {
//!     std::env::set_var(DEFAULT_BOOTSTRAP_ENV, "data-bootstrap");
//!     std::env::set_var(CONTROL_BOOTSTRAP_ENV, "control-bootstrap");
//! }
//!
//! // Use the default env names when the host also uses `ProcessSpec` defaults.
//! let bootstrap = ChildBootstrap::from_default_env()?;
//! // Later, once connected, you can send typed data payloads like `"ready"` directly.
//! // Or resolve the two env names explicitly when integrating with a custom launcher.
//! let explicit = ChildBootstrap::from_env(DEFAULT_BOOTSTRAP_ENV, CONTROL_BOOTSTRAP_ENV)?;
//! // `new` stores the env variable names for a later `connect()` call.
//! let custom = ChildBootstrap::new(
//!     DEFAULT_BOOTSTRAP_ENV.to_owned(),
//!     CONTROL_BOOTSTRAP_ENV.to_owned(),
//! );
//!
//! let _ = (bootstrap, explicit, custom);
//!
//! unsafe {
//!     std::env::remove_var(DEFAULT_BOOTSTRAP_ENV);
//!     std::env::remove_var(CONTROL_BOOTSTRAP_ENV);
//! }
//!
//! Ok(())
//! # }
//! # #[cfg(not(feature = "client"))]
//! # fn main() {}
//! ```
//!
//! # Public API reference
//!
//! - **Host-side** (requires `host` feature):
//!   - [`orchestrator::ProcessOrchestrator`]: main entry point for managing child processes
//!   - [`orchestrator::managed_child::ManagedChild`]: handle to a running child process
//!   - [`host::HostBootstrap`]: low-level host bootstrap coordination
//!
//! - **Child-side** (requires `client` feature):
//!   - [`child::bootstrap::ChildBootstrap`]: struct-based child bootstrap API
//!   - [`child::bootstrap::ChildBootstrapChannels`]: data/control API with
//!     independent receive workers and bounded queues
//!
//! - **Shared**:
//!   - [`spec::ProcessSpec`] and [`spec::ProcessSpecBuilder`]: child process configuration
//!   - [`error::OrchestratorError`]: error types
//!
//! If neither `host` nor `client` is enabled, only shared types are available.
//! Typical usage enables at least one feature.
//!
#![deny(missing_docs)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(unsafe_code)]

/// Child-side bootstrap helpers and shared child process constants.
#[cfg(feature = "client")]
pub mod child;
/// Error types and convenience aliases used by the orchestration API.
pub mod error;
/// Host-side bootstrap coordination (pure sequential dual-channel strategy).
#[cfg(feature = "host")]
pub mod host;
/// Host-side process lifecycle management.
#[cfg(feature = "host")]
pub mod orchestrator;
/// Child process configuration types and builders.
pub mod spec;
/// Strongly typed domain values used across the public API.
pub mod types;

/// Environment variable name used to pass the data channel handshake server name
/// from the host process to a managed child.
///
/// The child connects to this server and transfers the data-channel endpoints
/// through the one-shot handshake.
pub const DEFAULT_BOOTSTRAP_ENV: &str = "PORK_IPC_BOOTSTRAP";

/// Environment variable name used to pass the control channel handshake server name
/// from the host process to a managed child.
pub const CONTROL_BOOTSTRAP_ENV: &str = "PORK_CONTROL_BOOTSTRAP";
