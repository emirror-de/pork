use std::process::ExitStatus;
use std::sync::Arc;
use std::time::Duration;

use ipc_channel::ipc::IpcSender;
use tokio::sync::{Mutex as AsyncMutex, mpsc};

use super::ProcessOrchestrator;
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
    orchestrator: ProcessOrchestrator,
}

impl ManagedChild {
    pub(crate) fn new(
        process_id: ProcessId,
        name: Option<String>,
        sender: IpcSender<Vec<u8>>,
        receiver: Arc<AsyncMutex<mpsc::Receiver<Vec<u8>>>>,
        orchestrator: ProcessOrchestrator,
    ) -> Self {
        Self {
            process_id,
            name,
            sender,
            receiver,
            orchestrator,
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

    /// Requests a graceful shutdown and waits using the orchestrator's default timeout.
    pub fn shutdown(&self) -> Result<ExitStatus> {
        self.orchestrator.graceful_shutdown_process(self.process_id)
    }

    /// Requests a graceful shutdown and waits up to the provided timeout.
    ///
    /// If the child does not exit in time, the orchestrator falls back to a
    /// forceful stop.
    pub fn shutdown_with_timeout(&self, timeout: Duration) -> Result<ExitStatus> {
        self.orchestrator
            .graceful_shutdown_process_with_timeout(self.process_id, timeout)
    }

    /// Restarts the child process using the original [`crate::ProcessSpec`].
    pub fn restart(&self) -> Result<Self> {
        self.orchestrator.restart_process(self.process_id)
    }

    /// Gracefully shuts down the child with a custom timeout and then restarts it
    /// using the original [`crate::ProcessSpec`].
    pub fn restart_with_timeout(&self, timeout: Duration) -> Result<Self> {
        self.orchestrator
            .restart_process_with_timeout(self.process_id, timeout)
    }

    /// Stops the child process immediately and waits for it to exit.
    pub fn stop(&self) -> Result<ExitStatus> {
        self.orchestrator.stop_process(self.process_id)
    }

    /// Checks whether the child process has already exited without blocking.
    ///
    /// Returns `Ok(None)` if the process is still running.
    pub fn try_wait(&self) -> Result<Option<ExitStatus>> {
        self.orchestrator.try_wait(self.process_id)
    }
}
