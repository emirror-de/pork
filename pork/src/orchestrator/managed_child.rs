use std::pin::Pin;
use std::sync::Arc;

use futures_util::StreamExt;
use ipc_channel::asynch::IpcStream;
use ipc_channel::ipc::IpcSender;
use tokio::sync::Mutex as AsyncMutex;

use crate::error::Result;
use crate::types::{DataPayload, ManagedChildName, ProcessId};

type ManagedChildReceiverStream = Pin<Box<IpcStream<Vec<u8>>>>;
type SharedManagedChildReceiver = Arc<AsyncMutex<ManagedChildReceiverStream>>;

/// Stable identity of a managed child process.
///
/// This bundles the numeric process id assigned by the orchestrator with the
/// optional managed name configured in [`crate::spec::ProcessSpec`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ManagedChildIdentity<'a> {
    /// Stable process identifier assigned by the orchestrator.
    pub process_id: ProcessId,
    /// Managed name configured for this child, if any.
    pub managed_name: Option<&'a ManagedChildName>,
}

impl<'a> ManagedChildIdentity<'a> {
    /// Returns the stable process identifier assigned by the orchestrator.
    pub fn process_id(self) -> ProcessId {
        self.process_id
    }

    /// Returns the configured managed name, if one was assigned.
    pub fn managed_name(self) -> Option<&'a ManagedChildName> {
        self.managed_name
    }

    /// Returns `true` when this child has a managed name.
    pub fn has_name(self) -> bool {
        self.managed_name.is_some()
    }
}

/// Handle for interacting with a managed child process.
///
/// A `ManagedChild` lets you send raw IPC messages, receive responses, and
/// inspect the child identity after it has been started by a
/// [`ProcessOrchestrator`](super::ProcessOrchestrator).
#[derive(Clone)]
pub struct ManagedChild {
    process_id: ProcessId,
    name: Option<ManagedChildName>,
    sender: IpcSender<Vec<u8>>,
    receiver: SharedManagedChildReceiver,
}

impl ManagedChild {
    pub(crate) fn new(
        process_id: ProcessId,
        name: Option<ManagedChildName>,
        sender: IpcSender<Vec<u8>>,
        receiver: SharedManagedChildReceiver,
    ) -> Self {
        Self {
            process_id,
            name,
            sender,
            receiver,
        }
    }

    /// Returns the stable process identifier assigned by the orchestrator.
    pub fn process_id(&self) -> ProcessId {
        self.process_id
    }

    /// Returns the configured managed name, if one was assigned.
    pub fn managed_name(&self) -> Option<&ManagedChildName> {
        self.name.as_ref()
    }

    /// Returns `true` when this child has a managed name.
    pub fn has_name(&self) -> bool {
        self.name.is_some()
    }

    /// Returns the stable child identity for this managed process.
    pub fn identity(&self) -> ManagedChildIdentity<'_> {
        ManagedChildIdentity {
            process_id: self.process_id,
            managed_name: self.managed_name(),
        }
    }

    /// Sends a raw message payload to the child process.
    ///
    /// The payload format is application-defined. If you want a shared envelope
    /// for control messages and custom payloads, encode a
    /// `pork_proto::PorkIpcMessage<T>` before sending it.
    ///
    /// Callers can pass a [`DataPayload`] directly, or rely on the provided
    /// conversions from common byte and text types such as `Vec<u8>`, `&[u8]`,
    /// `String`, and `&str`.
    pub fn send(&self, message: impl Into<DataPayload>) -> Result<()> {
        self.sender.send(message.into().into_inner())?;
        Ok(())
    }

    /// Waits asynchronously for the next message from the child process.
    ///
    /// Returns `None` when the inbound channel has been closed. Successful
    /// messages are returned as [`DataPayload`] so the data plane stays distinct
    /// from control-plane bytes.
    pub async fn recv(&self) -> Option<DataPayload> {
        let mut receiver = self.receiver.lock().await;
        receiver
            .next()
            .await
            .and_then(|message| message.ok())
            .map(DataPayload::from)
    }
}
