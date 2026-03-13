use ipc_channel::ipc::{self, IpcReceiver, IpcSender};

use crate::error::{OrchestratorError, Result};
use crate::ipc::HandshakeChannels;

pub const GRACEFUL_SHUTDOWN_MESSAGE: &[u8] = b"__PORK_GRACEFUL_SHUTDOWN__";

pub fn child_bootstrap_env_value(env_name: &str) -> Result<String> {
    std::env::var(env_name).map_err(|_| OrchestratorError::MissingBootstrapValue)
}

pub fn child_connect_from_env(
    env_name: &str,
) -> Result<(IpcReceiver<Vec<u8>>, IpcSender<Vec<u8>>)> {
    let bootstrap_value = child_bootstrap_env_value(env_name)?;
    child_connect(&bootstrap_value)
}

pub fn child_connect(bootstrap_value: &str) -> Result<(IpcReceiver<Vec<u8>>, IpcSender<Vec<u8>>)> {
    let bootstrap_sender: IpcSender<HandshakeChannels> =
        IpcSender::connect(bootstrap_value.to_owned())?;

    let (to_child_sender, to_child_receiver) = ipc::channel::<Vec<u8>>()?;
    let (from_child_sender, from_child_receiver) = ipc::channel::<Vec<u8>>()?;

    let handshake = HandshakeChannels {
        to_child: to_child_sender,
        from_child: from_child_receiver,
    };

    bootstrap_sender.send(handshake)?;
    Ok((to_child_receiver, from_child_sender))
}

pub fn is_graceful_shutdown_message(message: &[u8]) -> bool {
    message == GRACEFUL_SHUTDOWN_MESSAGE
}

pub fn graceful_shutdown_message() -> Vec<u8> {
    GRACEFUL_SHUTDOWN_MESSAGE.to_vec()
}
