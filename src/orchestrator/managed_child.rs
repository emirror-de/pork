use std::process::ExitStatus;
use std::sync::Arc;

use ipc_channel::ipc::IpcSender;
use tokio::sync::{Mutex as AsyncMutex, mpsc};

use super::ProcessOrchestrator;
use crate::error::{ProcessId, Result};

#[derive(Debug, Clone)]
pub struct ManagedChild {
    process_id: ProcessId,
    sender: IpcSender<Vec<u8>>,
    receiver: Arc<AsyncMutex<mpsc::Receiver<Vec<u8>>>>,
    orchestrator: ProcessOrchestrator,
}

impl ManagedChild {
    pub(crate) fn new(
        process_id: ProcessId,
        sender: IpcSender<Vec<u8>>,
        receiver: Arc<AsyncMutex<mpsc::Receiver<Vec<u8>>>>,
        orchestrator: ProcessOrchestrator,
    ) -> Self {
        Self {
            process_id,
            sender,
            receiver,
            orchestrator,
        }
    }

    pub fn id(&self) -> ProcessId {
        self.process_id
    }

    pub fn send(&self, message: Vec<u8>) -> Result<()> {
        self.sender.send(message)?;
        Ok(())
    }

    pub async fn recv(&self) -> Option<Vec<u8>> {
        let mut receiver = self.receiver.lock().await;
        receiver.recv().await
    }

    pub fn stop(&self) -> Result<ExitStatus> {
        self.orchestrator.stop_process(self.process_id)
    }

    pub fn try_wait(&self) -> Result<Option<ExitStatus>> {
        self.orchestrator.try_wait(self.process_id)
    }
}
