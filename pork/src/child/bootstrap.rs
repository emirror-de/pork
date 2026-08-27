use futures_util::StreamExt;
use ipc_channel::ipc::{self, IpcReceiver, IpcSender};
use tokio::sync::{Mutex as AsyncMutex, mpsc};
use tokio::task::JoinHandle;

use crate::error::{OrchestratorError, Result};
use crate::types::{BootstrapEnvName, DataPayload};
use crate::{CONTROL_BOOTSTRAP_ENV, DEFAULT_BOOTSTRAP_ENV};
use pork_proto::protocol::{
    PORK_CONTROL_CODEC_ENV, ParsePorkControlCodecError, PorkControlCodec, PorkControlMessage,
};

const DATA_QUEUE_CAPACITY: usize = 1024;
const CONTROL_QUEUE_CAPACITY: usize = 16;

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

/// Data messages sent from a child to its host.
pub type DataSender = IpcSender<Vec<u8>>;

type DataHandshakeTuple = (DataSender, IpcReceiver<Vec<u8>>);
type RawControlSender = IpcSender<Vec<u8>>;
type RawControlReceiver = IpcReceiver<Vec<u8>>;
type ControlHandshakeTuple = (RawControlSender, RawControlReceiver);
type ControlQueueItem = Result<PorkControlMessage>;

struct RawChildChannels {
    data_sender: DataSender,
    data_receiver: IpcReceiver<Vec<u8>>,
    control_sender: RawControlSender,
    control_receiver: RawControlReceiver,
}

/// Sends codec-encoded control messages from a child to its host.
///
/// This type is typically cloned from [`ChildBootstrapChannels::control_sender`] and
/// used for status updates or other framework-level control messages.
#[derive(Debug, Clone)]
pub struct ControlSender {
    sender: RawControlSender,
    codec: PorkControlCodec,
}

impl ControlSender {
    pub(crate) fn new(sender: RawControlSender, codec: PorkControlCodec) -> Self {
        Self { sender, codec }
    }

    /// Sends one control message using the codec negotiated during bootstrap.
    pub fn send(&self, message: PorkControlMessage) -> Result<()> {
        let payload = self.codec.encode_control_message(message)?;
        self.sender.send(payload)?;
        Ok(())
    }

    /// Returns the codec used by this control sender.
    pub fn codec(&self) -> PorkControlCodec {
        self.codec
    }
}

/// Connected child-side data and control channels.
///
/// Data and control reception run in independent Tokio tasks with separate bounded
/// queues. Slow data consumers therefore apply backpressure only to the data plane,
/// and slow control consumers do not block application payload reception.
///
/// Use [`Self::send_data`] and [`Self::recv_data`] for application-defined payloads.
/// Use [`Self::send_control`] and [`Self::recv_control`] for framework-level
/// lifecycle traffic.
///
/// Dropping this value cancels both receive workers.
#[derive(Debug)]
pub struct ChildBootstrapChannels {
    data_sender: DataSender,
    data_receiver: AsyncMutex<mpsc::Receiver<Vec<u8>>>,
    control_sender: ControlSender,
    control_receiver: AsyncMutex<mpsc::Receiver<ControlQueueItem>>,
    worker_handles: [JoinHandle<()>; 2],
}

impl ChildBootstrapChannels {
    /// Sends an application payload to the host.
    ///
    /// Callers can pass a [`DataPayload`] directly, or rely on the provided
    /// conversions from common byte and text types such as `Vec<u8>`, `&[u8]`,
    /// `String`, and `&str`.
    pub fn send_data(&self, message: impl Into<DataPayload>) -> Result<()> {
        self.data_sender.send(message.into().into_inner())?;
        Ok(())
    }

    /// Receives the next application payload from the host.
    ///
    /// Concurrent calls are serialized to preserve message order. Cancellation does
    /// not consume a queued message.
    pub async fn recv_data(&self) -> Option<DataPayload> {
        self.data_receiver
            .lock()
            .await
            .recv()
            .await
            .map(DataPayload::from)
    }

    /// Sends a framework control message to the host through the control channel.
    ///
    /// Control messages are encoded into a dedicated [`crate::types::ControlPayload`]
    /// using the codec negotiated during bootstrap.
    pub fn send_control(&self, message: PorkControlMessage) -> Result<()> {
        self.control_sender.send(message)
    }

    /// Receives and decodes the next framework control message from the host.
    ///
    /// Concurrent calls are serialized to preserve message order. Codec failures are
    /// returned without stopping the independent control receive worker, so callers
    /// can report or skip malformed control payloads and continue receiving later
    /// messages.
    pub async fn recv_control(&self) -> Result<Option<PorkControlMessage>> {
        match self.control_receiver.lock().await.recv().await {
            Some(message) => message.map(Some),
            None => Ok(None),
        }
    }

    /// Returns a cloneable sender for periodic status reporting or other control traffic.
    pub fn control_sender(&self) -> ControlSender {
        self.control_sender.clone()
    }

    /// Returns the control codec negotiated with the host.
    pub fn control_codec(&self) -> PorkControlCodec {
        self.control_sender.codec()
    }
}

impl Drop for ChildBootstrapChannels {
    fn drop(&mut self) {
        for handle in &self.worker_handles {
            handle.abort();
        }
    }
}

/// Bootstrap coordinator for child-side connection setup.
///
/// Connects to the host's bootstrap servers, exchanges channel endpoints, resolves
/// the negotiated control codec from the environment, and constructs the child-side
/// data/control API.
pub struct ChildBootstrap {
    data_env_name: BootstrapEnvName,
    control_env_name: BootstrapEnvName,
}

impl ChildBootstrap {
    /// Creates a new child bootstrap coordinator from environment variables.
    ///
    /// Reads the server names from the given environment variables.
    pub fn from_env(
        data_env_name: impl Into<BootstrapEnvName>,
        control_env_name: impl Into<BootstrapEnvName>,
    ) -> Result<Self> {
        let data_env_name = data_env_name.into();
        let control_env_name = control_env_name.into();
        std::env::var(data_env_name.as_str())
            .map_err(|_| OrchestratorError::MissingBootstrapValue)?;
        std::env::var(control_env_name.as_str())
            .map_err(|_| OrchestratorError::MissingBootstrapValue)?;
        Ok(Self {
            data_env_name,
            control_env_name,
        })
    }

    /// Creates a new child bootstrap coordinator using Pork's default bootstrap
    /// environment variable names.
    pub fn from_default_env() -> Result<Self> {
        Self::from_env(DEFAULT_BOOTSTRAP_ENV, CONTROL_BOOTSTRAP_ENV)
    }

    /// Creates a new child bootstrap coordinator from explicit environment variable names.
    ///
    /// Useful for testing or when env var names are provided through other means.
    pub fn new(
        data_env_name: impl Into<BootstrapEnvName>,
        control_env_name: impl Into<BootstrapEnvName>,
    ) -> Self {
        Self {
            data_env_name: data_env_name.into(),
            control_env_name: control_env_name.into(),
        }
    }

    /// Connects to both data and control bootstrap servers.
    ///
    /// The returned connection owns two independent receive workers backed by
    /// bounded queues. The handshake itself is completed on a blocking worker thread
    /// because `ipc-channel` setup is synchronous; after that, `recv_data` and
    /// `recv_control` are asynchronous and can be awaited independently.
    pub async fn connect(self) -> Result<ChildBootstrapChannels> {
        let data_server_name = std::env::var(self.data_env_name.as_str())
            .map_err(|_| OrchestratorError::MissingBootstrapValue)?;
        let control_server_name = std::env::var(self.control_env_name.as_str())
            .map_err(|_| OrchestratorError::MissingBootstrapValue)?;
        let control_codec = child_control_codec_from_env()?;

        let channels = tokio::task::spawn_blocking(move || {
            connect_channels(data_server_name, control_server_name)
        })
        .await
        .map_err(|error| OrchestratorError::Io(std::io::Error::other(error)))??;

        let (data_receiver, data_worker) =
            spawn_data_worker(channels.data_receiver, DATA_QUEUE_CAPACITY);
        let (control_receiver, control_worker) = spawn_control_worker(
            channels.control_receiver,
            control_codec,
            CONTROL_QUEUE_CAPACITY,
        );

        Ok(ChildBootstrapChannels {
            data_sender: channels.data_sender,
            data_receiver: AsyncMutex::new(data_receiver),
            control_sender: ControlSender::new(channels.control_sender, control_codec),
            control_receiver: AsyncMutex::new(control_receiver),
            worker_handles: [data_worker, control_worker],
        })
    }
}

fn connect_channels(
    data_server_name: String,
    control_server_name: String,
) -> Result<RawChildChannels> {
    let data_handshake = IpcSender::<DataHandshakeTuple>::connect(data_server_name)?;
    let (data_to_host_sender, data_to_host_receiver) = ipc::channel::<Vec<u8>>()?;
    let (data_from_host_sender, data_from_host_receiver) = ipc::channel::<Vec<u8>>()?;
    data_handshake.send((data_from_host_sender, data_to_host_receiver))?;

    let control_handshake = IpcSender::<ControlHandshakeTuple>::connect(control_server_name)?;
    let (control_to_host_sender, control_to_host_receiver) = ipc::channel::<Vec<u8>>()?;
    let (control_from_host_sender, control_from_host_receiver) = ipc::channel::<Vec<u8>>()?;
    control_handshake.send((control_from_host_sender, control_to_host_receiver))?;

    Ok(RawChildChannels {
        data_sender: data_to_host_sender,
        data_receiver: data_from_host_receiver,
        control_sender: control_to_host_sender,
        control_receiver: control_from_host_receiver,
    })
}

fn spawn_data_worker(
    receiver: IpcReceiver<Vec<u8>>,
    capacity: usize,
) -> (mpsc::Receiver<Vec<u8>>, JoinHandle<()>) {
    let (sender, queue) = mpsc::channel(capacity);
    let handle = tokio::spawn(async move {
        let mut stream = receiver.to_stream();
        while let Some(message) = stream.next().await {
            let Ok(message) = message else {
                break;
            };
            if sender.send(message).await.is_err() {
                break;
            }
        }
    });
    (queue, handle)
}

fn spawn_control_worker(
    receiver: RawControlReceiver,
    codec: PorkControlCodec,
    capacity: usize,
) -> (mpsc::Receiver<ControlQueueItem>, JoinHandle<()>) {
    let (sender, queue) = mpsc::channel(capacity);
    let handle = tokio::spawn(async move {
        let mut stream = receiver.to_stream();
        while let Some(message) = stream.next().await {
            let Ok(bytes) = message else {
                break;
            };
            let decoded = codec.decode_control_message(&bytes).map_err(Into::into);
            if sender.send(decoded).await.is_err() {
                break;
            }
        }
    });
    (queue, handle)
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use pork_proto::protocol::PorkProtoCodecError;

    use super::*;

    fn test_control_codec() -> Option<PorkControlCodec> {
        PorkControlCodec::available().into_iter().next()
    }

    #[test]
    fn child_bootstrap_new_creates_instance() {
        let bootstrap =
            ChildBootstrap::new("TEST_DATA_ENV".to_string(), "TEST_CONTROL_ENV".to_string());
        assert_eq!(
            bootstrap.data_env_name,
            BootstrapEnvName::from("TEST_DATA_ENV")
        );
        assert_eq!(
            bootstrap.control_env_name,
            BootstrapEnvName::from("TEST_CONTROL_ENV")
        );
    }

    #[test]
    fn child_bootstrap_from_env_with_missing_data_var() {
        let result = ChildBootstrap::from_env(
            "NONEXISTENT_DATA_VAR_12345",
            "NONEXISTENT_CONTROL_VAR_12345",
        );
        assert!(result.is_err(), "Expected error when env vars missing");
    }

    #[test]
    fn child_bootstrap_from_default_env_constructor_is_available() {
        let constructor: fn() -> Result<ChildBootstrap> = ChildBootstrap::from_default_env;
        let _ = constructor;
    }

    #[test]
    fn child_bootstrap_channels_debug_format() {
        let type_name = std::any::type_name::<ChildBootstrapChannels>();
        assert!(type_name.contains("ChildBootstrapChannels"));
    }

    #[test]
    fn control_and_data_sender_types_exist() {
        let _control_sender_type = std::any::type_name::<ControlSender>();
        let _data_sender_type = std::any::type_name::<DataSender>();
    }

    #[tokio::test]
    async fn control_worker_progresses_while_data_queue_is_full() {
        let (data_sender, data_receiver) = match ipc::channel::<Vec<u8>>() {
            Ok(channels) => channels,
            Err(error) => panic!("data channel should be created: {error}"),
        };
        let (control_sender, control_receiver) = match ipc::channel::<Vec<u8>>() {
            Ok(channels) => channels,
            Err(error) => panic!("control channel should be created: {error}"),
        };
        let (_data_queue, data_worker) = spawn_data_worker(data_receiver, 1);
        let codec = test_control_codec().unwrap_or(PorkControlCodec::Json);
        let (mut control_queue, control_worker) = spawn_control_worker(control_receiver, codec, 1);

        if let Err(error) = data_sender.send(vec![1]) {
            panic!("first data message should be sent: {error}");
        }
        if let Err(error) = data_sender.send(vec![2]) {
            panic!("second data message should be sent: {error}");
        }

        let payload = match test_control_codec() {
            Some(codec) => match codec.encode_restart() {
                Ok(message) => message,
                Err(error) => panic!("restart should be encoded: {error}"),
            },
            None => b"control".to_vec(),
        };
        if let Err(error) = control_sender.send(payload) {
            panic!("control payload should be sent: {error}");
        }

        let received = tokio::time::timeout(Duration::from_secs(1), control_queue.recv()).await;
        match (test_control_codec(), received) {
            (Some(_), Ok(Some(Ok(PorkControlMessage::Restart)))) => {}
            (
                None,
                Ok(Some(Err(OrchestratorError::ControlCodec(
                    PorkProtoCodecError::UnsupportedCodec,
                )))),
            ) => {}
            other => panic!("control worker should remain responsive: {other:?}"),
        }

        data_worker.abort();
        control_worker.abort();
    }

    #[tokio::test]
    async fn control_worker_surfaces_invalid_payload_and_keeps_receiving() {
        let (sender, receiver) = match ipc::channel::<Vec<u8>>() {
            Ok(channels) => channels,
            Err(error) => panic!("control channel should be created: {error}"),
        };
        let codec = test_control_codec().unwrap_or(PorkControlCodec::Json);
        let (mut queue, worker) = spawn_control_worker(receiver, codec, 2);

        if let Err(error) = sender.send(b"invalid".to_vec()) {
            panic!("invalid payload should reach the worker: {error}");
        }
        let payload = match test_control_codec() {
            Some(codec) => match codec.encode_restart() {
                Ok(message) => message,
                Err(error) => panic!("restart should be encoded: {error}"),
            },
            None => b"still-unsupported".to_vec(),
        };
        if let Err(error) = sender.send(payload) {
            panic!("control payload should be sent: {error}");
        }

        assert!(matches!(queue.recv().await, Some(Err(_))));
        match test_control_codec() {
            Some(_) => assert!(matches!(
                queue.recv().await,
                Some(Ok(PorkControlMessage::Restart))
            )),
            None => assert!(matches!(
                queue.recv().await,
                Some(Err(OrchestratorError::ControlCodec(
                    PorkProtoCodecError::UnsupportedCodec
                )))
            )),
        }

        worker.abort();
    }
}
