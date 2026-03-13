use ipc_channel::ipc::{self, IpcReceiver, IpcSender};

use crate::error::{OrchestratorError, Result};
use crate::ipc::HandshakeChannels;
use crate::{PORK_CONTROL_CODEC_ENV, ParsePorkControlCodecError, PorkControlCodec};

pub fn child_bootstrap_env_value(env_name: &str) -> Result<String> {
    std::env::var(env_name).map_err(|_| OrchestratorError::MissingBootstrapValue)
}

pub fn child_control_codec_from_env() -> Result<PorkControlCodec> {
    let value = std::env::var(PORK_CONTROL_CODEC_ENV)
        .map_err(|_| OrchestratorError::MissingControlCodec)?;
    value.parse().map_err(|error: ParsePorkControlCodecError| {
        OrchestratorError::UnsupportedControlCodec(error.value().to_owned())
    })
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
