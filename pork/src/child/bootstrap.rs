use ipc_channel::ipc::{self, IpcReceiver, IpcSender};
use tokio::sync::Mutex as AsyncMutex;

use crate::error::{OrchestratorError, Result};
use crate::ipc::HandshakeChannels;
use pork_proto::protocol::{PORK_CONTROL_CODEC_ENV, ParsePorkControlCodecError, PorkControlCodec};

/// Async child-side receiver for raw host messages.
pub type ChildInboundReceiver = AsyncMutex<IpcReceiver<Vec<u8>>>;
/// Child-side sender for raw messages back to the host.
pub type ChildOutboundSender = IpcSender<Vec<u8>>;
type ChildBootstrapChannels = (ChildInboundReceiver, ChildOutboundSender);

/// Reads the child bootstrap value from the given environment variable.
///
/// This is typically used inside a managed child process to retrieve the
/// one-shot IPC server name that the parent injected before spawning it.
pub fn child_bootstrap_env_value(env_name: &str) -> Result<String> {
    std::env::var(env_name).map_err(|_| OrchestratorError::MissingBootstrapValue)
}

/// Resolves the control codec for the current child process from the environment.
///
/// The parent process sets [`PORK_CONTROL_CODEC_ENV`] before spawn so the child
/// can decode framework control messages with the same codec.
pub fn child_control_codec_from_env() -> Result<PorkControlCodec> {
    let value = std::env::var(PORK_CONTROL_CODEC_ENV)
        .map_err(|_| OrchestratorError::MissingControlCodec)?;
    value.parse().map_err(|error: ParsePorkControlCodecError| {
        OrchestratorError::UnsupportedControlCodec(error.value().to_owned())
    })
}

/// Connects a child process back to the parent using a bootstrap value stored in
/// the provided environment variable.
///
/// On success, returns `(from_host, to_host)` where:
/// - `from_host` receives raw messages sent by the parent
/// - `to_host` sends raw messages back to the parent
pub async fn child_connect_from_env(env_name: &str) -> Result<ChildBootstrapChannels> {
    let bootstrap_value = child_bootstrap_env_value(env_name)?;
    child_connect(&bootstrap_value).await
}

/// Connects a child process back to the parent using an explicit bootstrap value.
///
/// This performs the Pork child-side handshake by connecting to the parent's
/// one-shot bootstrap server, creating the two message channels, and sending the
/// handshake payload back to the parent.
///
/// On success, returns `(from_host, to_host)` where:
/// - `from_host` receives raw messages sent by the parent
/// - `to_host` sends raw messages back to the parent
pub async fn child_connect(bootstrap_value: &str) -> Result<ChildBootstrapChannels> {
    let bootstrap_sender: IpcSender<HandshakeChannels> =
        IpcSender::connect(bootstrap_value.to_owned())?;

    let (to_child_sender, to_child_receiver) = ipc::channel::<Vec<u8>>()?;
    let (from_child_sender, from_child_receiver) = ipc::channel::<Vec<u8>>()?;

    let handshake = HandshakeChannels {
        to_child: to_child_sender,
        from_child: from_child_receiver,
    };

    tokio::task::spawn_blocking(move || bootstrap_sender.send(handshake))
        .await
        .map_err(|error| OrchestratorError::Io(std::io::Error::other(error)))??;

    Ok((AsyncMutex::new(to_child_receiver), from_child_sender))
}
