use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

pub mod client;

pub const PORK_CONTROL_CODEC_ENV: &str = "PORK_CONTROL_CODEC";

/// Host-to-child and child-to-host control-plane messages shared by all Pork-based
/// IPC protocols.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PorkControlMessage {
    GracefulShutdown,
}

/// Shared wrapper for Pork IPC traffic.
///
/// `Control` is reserved for framework-level coordination between host and child.
/// `Custom` allows applications to embed their own protocol payloads while reusing
/// the shared control plane.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PorkIpcMessage<T> {
    Control(PorkControlMessage),
    Custom(T),
}

/// Errors returned by built-in codec helpers.
#[derive(Debug)]
pub enum PorkProtoCodecError {
    #[cfg(feature = "codec-json")]
    Json(serde_json::Error),
    #[cfg(feature = "codec-postcard")]
    Postcard(String),
    UnsupportedCodec,
}

impl fmt::Display for PorkProtoCodecError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            #[cfg(feature = "codec-json")]
            Self::Json(error) => write!(f, "json codec error: {error}"),
            #[cfg(feature = "codec-postcard")]
            Self::Postcard(error) => write!(f, "postcard codec error: {error}"),
            Self::UnsupportedCodec => write!(f, "requested codec is not enabled"),
        }
    }
}

impl std::error::Error for PorkProtoCodecError {}

#[cfg(feature = "codec-json")]
impl From<serde_json::Error> for PorkProtoCodecError {
    fn from(value: serde_json::Error) -> Self {
        Self::Json(value)
    }
}

#[cfg(feature = "codec-postcard")]
impl PorkProtoCodecError {
    fn from_postcard_error<E>(value: E) -> Self
    where
        E: fmt::Display,
    {
        Self::Postcard(value.to_string())
    }
}

/// Built-in control-message codec selection shared between host and child.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum PorkControlCodec {
    #[default]
    Json,
    Postcard,
}

impl PorkControlCodec {
    pub fn as_env_value(self) -> &'static str {
        match self {
            Self::Json => "json",
            Self::Postcard => "postcard",
        }
    }

    pub fn encode_control_message(
        self,
        message: PorkControlMessage,
    ) -> Result<Vec<u8>, PorkProtoCodecError> {
        let message = PorkIpcMessage::<Vec<u8>>::Control(message);

        match self {
            Self::Json => {
                #[cfg(feature = "codec-json")]
                {
                    json::encode(&message)
                }
                #[cfg(not(feature = "codec-json"))]
                {
                    let _ = message;
                    Err(PorkProtoCodecError::UnsupportedCodec)
                }
            }
            Self::Postcard => {
                #[cfg(feature = "codec-postcard")]
                {
                    postcard::encode(&message)
                }
                #[cfg(not(feature = "codec-postcard"))]
                {
                    let _ = message;
                    Err(PorkProtoCodecError::UnsupportedCodec)
                }
            }
        }
    }

    pub fn decode_control_message(
        self,
        bytes: &[u8],
    ) -> Result<PorkControlMessage, PorkProtoCodecError> {
        let message: PorkIpcMessage<Vec<u8>> = match self {
            Self::Json => {
                #[cfg(feature = "codec-json")]
                {
                    json::decode(bytes)?
                }
                #[cfg(not(feature = "codec-json"))]
                {
                    let _ = bytes;
                    return Err(PorkProtoCodecError::UnsupportedCodec);
                }
            }
            Self::Postcard => {
                #[cfg(feature = "codec-postcard")]
                {
                    postcard::decode(bytes)?
                }
                #[cfg(not(feature = "codec-postcard"))]
                {
                    let _ = bytes;
                    return Err(PorkProtoCodecError::UnsupportedCodec);
                }
            }
        };

        match message {
            PorkIpcMessage::Control(control) => Ok(control),
            PorkIpcMessage::Custom(_) => Err(PorkProtoCodecError::UnsupportedCodec),
        }
    }

    pub fn encode_graceful_shutdown(self) -> Result<Vec<u8>, PorkProtoCodecError> {
        self.encode_control_message(PorkControlMessage::GracefulShutdown)
    }

    pub fn is_graceful_shutdown_message(self, bytes: &[u8]) -> bool {
        matches!(
            self.decode_control_message(bytes),
            Ok(PorkControlMessage::GracefulShutdown)
        )
    }
}

impl fmt::Display for PorkControlCodec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_env_value())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsePorkControlCodecError {
    value: String,
}

impl ParsePorkControlCodecError {
    pub fn value(&self) -> &str {
        &self.value
    }
}

impl fmt::Display for ParsePorkControlCodecError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "unsupported control codec '{}'", self.value)
    }
}

impl std::error::Error for ParsePorkControlCodecError {}

impl FromStr for PorkControlCodec {
    type Err = ParsePorkControlCodecError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "json" => Ok(Self::Json),
            "postcard" => Ok(Self::Postcard),
            _ => Err(ParsePorkControlCodecError {
                value: value.to_owned(),
            }),
        }
    }
}

pub fn control_codec_from_env() -> Result<PorkControlCodec, ParsePorkControlCodecError> {
    match std::env::var(PORK_CONTROL_CODEC_ENV) {
        Ok(value) => value.parse(),
        Err(_) => Ok(PorkControlCodec::default()),
    }
}

pub fn is_graceful_shutdown_message(message: &[u8], codec: PorkControlCodec) -> bool {
    codec.is_graceful_shutdown_message(message)
}

pub fn graceful_shutdown_message(codec: PorkControlCodec) -> Vec<u8> {
    codec
        .encode_graceful_shutdown()
        .expect("serializing graceful shutdown control message should never fail")
}

pub fn encode_control_message(
    message: PorkControlMessage,
    codec: PorkControlCodec,
) -> Result<Vec<u8>, PorkProtoCodecError> {
    codec.encode_control_message(message)
}

pub fn decode_control_message(
    bytes: &[u8],
    codec: PorkControlCodec,
) -> Result<PorkControlMessage, PorkProtoCodecError> {
    codec.decode_control_message(bytes)
}

pub fn child_bootstrap_env_value(env_name: &str) -> Result<String, std::env::VarError> {
    std::env::var(env_name)
}

pub fn child_control_codec_from_env() -> Result<PorkControlCodec, ParsePorkControlCodecError> {
    control_codec_from_env()
}

/// Shared codec trait for feature-gated built-in encoders.
///
/// Implementations are provided by the built-in codec marker types in the
/// `json` and `postcard` modules when the respective feature is enabled.
pub trait PorkCodec {
    fn encode<T>(message: &PorkIpcMessage<T>) -> Result<Vec<u8>, PorkProtoCodecError>
    where
        T: Serialize;

    fn decode<T>(bytes: &[u8]) -> Result<PorkIpcMessage<T>, PorkProtoCodecError>
    where
        T: for<'de> Deserialize<'de>;
}

#[cfg(feature = "codec-json")]
pub mod json {
    use super::{PorkCodec, PorkIpcMessage, PorkProtoCodecError};
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, Copy, Default)]
    pub struct JsonCodec;

    impl PorkCodec for JsonCodec {
        fn encode<T>(message: &PorkIpcMessage<T>) -> Result<Vec<u8>, PorkProtoCodecError>
        where
            T: Serialize,
        {
            serde_json::to_vec(message).map_err(Into::into)
        }

        fn decode<T>(bytes: &[u8]) -> Result<PorkIpcMessage<T>, PorkProtoCodecError>
        where
            T: for<'de> Deserialize<'de>,
        {
            serde_json::from_slice(bytes).map_err(Into::into)
        }
    }

    pub fn encode<T>(message: &PorkIpcMessage<T>) -> Result<Vec<u8>, PorkProtoCodecError>
    where
        T: Serialize,
    {
        JsonCodec::encode(message)
    }

    pub fn decode<T>(bytes: &[u8]) -> Result<PorkIpcMessage<T>, PorkProtoCodecError>
    where
        T: for<'de> Deserialize<'de>,
    {
        JsonCodec::decode(bytes)
    }
}

#[cfg(feature = "codec-postcard")]
pub mod postcard {
    use super::{PorkCodec, PorkIpcMessage, PorkProtoCodecError};
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, Copy, Default)]
    pub struct PostcardCodec;

    impl PorkCodec for PostcardCodec {
        fn encode<T>(message: &PorkIpcMessage<T>) -> Result<Vec<u8>, PorkProtoCodecError>
        where
            T: Serialize,
        {
            postcard::to_stdvec(message).map_err(PorkProtoCodecError::from_postcard_error)
        }

        fn decode<T>(bytes: &[u8]) -> Result<PorkIpcMessage<T>, PorkProtoCodecError>
        where
            T: for<'de> Deserialize<'de>,
        {
            postcard::from_bytes(bytes).map_err(PorkProtoCodecError::from_postcard_error)
        }
    }

    pub fn encode<T>(message: &PorkIpcMessage<T>) -> Result<Vec<u8>, PorkProtoCodecError>
    where
        T: Serialize,
    {
        PostcardCodec::encode(message)
    }

    pub fn decode<T>(bytes: &[u8]) -> Result<PorkIpcMessage<T>, PorkProtoCodecError>
    where
        T: for<'de> Deserialize<'de>,
    {
        PostcardCodec::decode(bytes)
    }
}
