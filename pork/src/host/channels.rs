use ipc_channel::asynch::IpcStream;
use ipc_channel::ipc::{IpcReceiver, IpcSender};

use crate::error::Result;
use crate::types::{ControlPayload, DataPayload};

/// Host-side receiver for encoded control payloads from a child.
#[derive(Debug)]
pub struct HostControlReceiver {
    receiver: IpcReceiver<Vec<u8>>,
}

impl HostControlReceiver {
    pub(crate) fn new(receiver: IpcReceiver<Vec<u8>>) -> Self {
        Self { receiver }
    }

    pub(crate) fn into_inner(self) -> IpcReceiver<Vec<u8>> {
        self.receiver
    }
}

/// Host-side sender for encoded control payloads to a child.
#[derive(Debug, Clone)]
pub struct HostControlSender {
    sender: IpcSender<Vec<u8>>,
}

impl HostControlSender {
    pub(crate) fn new(sender: IpcSender<Vec<u8>>) -> Self {
        Self { sender }
    }

    /// Sends one encoded control payload to the child over the control channel.
    pub fn send(&self, payload: impl Into<ControlPayload>) -> Result<()> {
        self.sender.send(payload.into().into_inner())?;
        Ok(())
    }
}

/// Host-side receiver for application payloads from a child.
#[derive(Debug)]
pub struct HostDataReceiver {
    receiver: IpcReceiver<Vec<u8>>,
}

impl HostDataReceiver {
    pub(crate) fn new(receiver: IpcReceiver<Vec<u8>>) -> Self {
        Self { receiver }
    }

    pub(crate) fn into_stream(self) -> IpcStream<Vec<u8>> {
        self.receiver.to_stream()
    }
}

/// Host-side sender for application payloads to a child.
#[derive(Debug, Clone)]
pub struct HostDataSender {
    sender: IpcSender<Vec<u8>>,
}

impl HostDataSender {
    pub(crate) fn new(sender: IpcSender<Vec<u8>>) -> Self {
        Self { sender }
    }

    /// Sends one application payload to the child over the data channel.
    pub fn send(&self, payload: impl Into<DataPayload>) -> Result<()> {
        self.sender.send(payload.into().into_inner())?;
        Ok(())
    }

    pub(crate) fn clone_inner(&self) -> IpcSender<Vec<u8>> {
        self.sender.clone()
    }
}
