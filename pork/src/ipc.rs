use ipc_channel::ipc::{IpcReceiver, IpcSender};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct HandshakeChannels {
    pub(crate) to_child: IpcSender<Vec<u8>>,
    pub(crate) from_child: IpcReceiver<Vec<u8>>,
}
