use std::fmt;
use std::str::FromStr;

use crate::{PorkControlMessage, PorkIpcMessage, PorkProtoCodecError};

/// Environment variable used by parent and child processes to agree on the
/// control-message codec.
pub const PORK_CONTROL_CODEC_ENV: &str = "PORK_CONTROL_CODEC";

/// Built-in control-message codec selection shared between host and child.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum PorkControlCodec {
    /// Encode control messages as JSON.
    #[default]
    Json,
    /// Encode control messages as Postcard.
    Postcard,
}

impl PorkControlCodec {
    /// Returns the canonical environment-variable value for this codec.
    pub fn as_env_value(self) -> &'static str {
        match self {
            Self::Json => "json",
            Self::Postcard => "postcard",
        }
    }

    /// Serializes a control message using the selected codec.
    pub fn encode_control_message(
        self,
        message: PorkControlMessage,
    ) -> Result<Vec<u8>, PorkProtoCodecError> {
        let message = PorkIpcMessage::<Vec<u8>>::Control(message);

        match self {
            Self::Json => crate::json::encode(&message),
            Self::Postcard => crate::postcard::encode(&message),
        }
    }

    /// Deserializes a control message using the selected codec.
    pub fn decode_control_message(
        self,
        bytes: &[u8],
    ) -> Result<PorkControlMessage, PorkProtoCodecError> {
        let message: PorkIpcMessage<Vec<u8>> = match self {
            Self::Json => crate::json::decode(bytes)?,
            Self::Postcard => crate::postcard::decode(bytes)?,
        };

        match message {
            PorkIpcMessage::Control(control) => Ok(control),
            PorkIpcMessage::Custom(_) => Err(PorkProtoCodecError::UnsupportedCodec),
        }
    }

    /// Serializes a graceful-shutdown control message using the selected codec.
    pub fn encode_graceful_shutdown(self) -> Result<Vec<u8>, PorkProtoCodecError> {
        self.encode_control_message(PorkControlMessage::GracefulShutdown)
    }

    /// Returns `true` when the given bytes decode to a graceful-shutdown control message.
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

/// Error returned when parsing a [`PorkControlCodec`] from a string fails.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsePorkControlCodecError {
    value: String,
}

impl ParsePorkControlCodecError {
    /// Returns the unsupported codec value that failed to parse.
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

/// Resolves the control codec from [`PORK_CONTROL_CODEC_ENV`].
///
/// If the variable is missing, this function returns [`PorkControlCodec::Json`].
pub fn control_codec_from_env() -> Result<PorkControlCodec, ParsePorkControlCodecError> {
    match std::env::var(PORK_CONTROL_CODEC_ENV) {
        Ok(value) => value.parse(),
        Err(_) => Ok(PorkControlCodec::default()),
    }
}

/// Builds a serialized graceful-shutdown control message for the provided codec.
pub fn graceful_shutdown_message(codec: PorkControlCodec) -> Vec<u8> {
    codec
        .encode_graceful_shutdown()
        .expect("serializing graceful shutdown control message should never fail")
}

/// Returns `true` when the given bytes decode to a graceful-shutdown control message
/// with the provided codec.
pub fn is_graceful_shutdown_message(message: &[u8], codec: PorkControlCodec) -> bool {
    codec.is_graceful_shutdown_message(message)
}

/// Serializes a control message with the provided codec.
pub fn encode_control_message(
    message: PorkControlMessage,
    codec: PorkControlCodec,
) -> Result<Vec<u8>, PorkProtoCodecError> {
    codec.encode_control_message(message)
}

/// Deserializes a control message with the provided codec.
pub fn decode_control_message(
    bytes: &[u8],
    codec: PorkControlCodec,
) -> Result<PorkControlMessage, PorkProtoCodecError> {
    codec.decode_control_message(bytes)
}
