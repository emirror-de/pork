use serde::{Deserialize, Serialize};

/// Host-to-child and child-to-host control-plane messages shared by all Pork-based
/// IPC protocols.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PorkControlMessage {
    GracefulShutdown,
}

/// Shared wrapper for Pork IPC traffic.
///
/// `Control` is reserved for framework-level coordination between host and child.
/// `Custom` allows applications to embed their own protocol payloads while reusing
/// the shared control plane.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PorkIpcMessage<T> {
    Control(PorkControlMessage),
    Custom(T),
}
