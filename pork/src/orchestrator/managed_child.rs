use std::sync::Arc;

use ipc_channel::ipc::IpcSender;
use tokio::sync::{Mutex as AsyncMutex, mpsc};

use crate::error::{ProcessId, Result};

/// Handle for interacting with a managed child process.
///
/// A `ManagedChild` lets you send raw IPC messages, receive responses, and
/// control the child lifecycle after it has been started by a
/// [`ProcessOrchestrator`](super::ProcessOrchestrator).
#[derive(Debug, Clone)]
pub struct ManagedChild {
    process_id: ProcessId,
    name: Option<String>,
    sender: IpcSender<Vec<u8>>,
    receiver: Arc<AsyncMutex<mpsc::Receiver<Vec<u8>>>>,
}

impl ManagedChild {
    pub(crate) fn new(
        process_id: ProcessId,
        name: Option<String>,
        sender: IpcSender<Vec<u8>>,
        receiver: Arc<AsyncMutex<mpsc::Receiver<Vec<u8>>>>,
    ) -> Self {
        Self {
            process_id,
            name,
            sender,
            receiver,
        }
    }

    /// Returns the stable process identifier assigned by the orchestrator.
    pub fn id(&self) -> ProcessId {
        self.process_id
    }

    /// Returns the configured managed name, if one was assigned.
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    /// Sends a raw message payload to the child process.
    ///
    /// The payload format is application-defined. If you want a shared envelope
    /// for control messages and custom payloads, encode a
    /// `pork_proto::PorkIpcMessage<T>` before sending it.
    pub fn send(&self, message: Vec<u8>) -> Result<()> {
        self.sender.send(message)?;
        Ok(())
    }

    /// Waits asynchronously for the next message from the child process.
    ///
    /// Returns `None` when the inbound channel has been closed.
    pub async fn recv(&self) -> Option<Vec<u8>> {
        let mut receiver = self.receiver.lock().await;
        receiver.recv().await
    }
}
