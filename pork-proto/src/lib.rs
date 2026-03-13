//! Shared protocol primitives for applications built on top of `pork`.
//!
//! This crate defines the control-plane messages exchanged between a parent
//! process and a managed child process, plus optional built-in codecs for
//! serializing those messages.
//!
//! # What this crate gives you
//!
//! - `PorkControlMessage` for framework-level coordination
//! - `PorkIpcMessage<T>` for wrapping your own protocol payloads
//! - `PorkControlCodec` for choosing how control messages are encoded
//! - feature-gated `json` and `postcard` helpers for custom payloads
//! - environment helpers so parent and child agree on the selected codec
//!
//! # Feature flags
//!
//! - `codec-json` enables the JSON codec helpers
//! - `codec-postcard` enables the Postcard codec helpers
//!
//! You can enable one or both features depending on the transport format you
//! want to expose to consumers.
//!
//! # Example: wrap your own messages
//!
//! ```rust
//! use pork_proto::{PorkControlMessage, PorkIpcMessage};
//! use serde::{Deserialize, Serialize};
//!
//! #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
//! struct AppMessage {
//!     command: String,
//! }
//!
//! let app_message = PorkIpcMessage::Custom(AppMessage {
//!     command: "ping".to_owned(),
//! });
//!
//! let shutdown = PorkIpcMessage::<AppMessage>::Control(PorkControlMessage::GracefulShutdown);
//!
//! assert!(matches!(app_message, PorkIpcMessage::Custom(_)));
//! assert!(matches!(shutdown, PorkIpcMessage::Control(PorkControlMessage::GracefulShutdown)));
//! ```
//!
//! # Example: encode and decode control messages
//!
//! ```rust
//! use pork_proto::{decode_control_message, encode_control_message, PorkControlCodec, PorkControlMessage};
//!
//! let codec = PorkControlCodec::Json;
//! let bytes = encode_control_message(PorkControlMessage::GracefulShutdown, codec)?;
//! let message = decode_control_message(&bytes, codec)?;
//!
//! assert_eq!(message, PorkControlMessage::GracefulShutdown);
//! # Ok::<(), pork_proto::PorkProtoCodecError>(())
//! ```
//!
//! # Example: serialize your own payload with the JSON codec
//!
//! ```rust
//! # #[cfg(feature = "codec-json")]
//! # {
//! use pork_proto::{json, PorkIpcMessage};
//! use serde::{Deserialize, Serialize};
//!
//! #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
//! struct AppMessage {
//!     command: String,
//! }
//!
//! let original = PorkIpcMessage::Custom(AppMessage {
//!     command: "reload".to_owned(),
//! });
//!
//! let bytes = json::encode(&original)?;
//! let decoded: PorkIpcMessage<AppMessage> = json::decode(&bytes)?;
//!
//! assert_eq!(decoded, original);
//! # Ok::<(), pork_proto::PorkProtoCodecError>(())
//! # }
//! ```
//!
//! # Example: agree on a codec through the environment
//!
//! The parent process can write the selected codec into
//! `PORK_CONTROL_CODEC_ENV`, and the child can read it back with
//! `control_codec_from_env()`.
//!
//! ```rust
//! use pork_proto::{control_codec_from_env, PORK_CONTROL_CODEC_ENV, PorkControlCodec};
//!
//! // In a real process bootstrap flow, the parent would set this before spawning the child.
//! unsafe {
//!     std::env::set_var(PORK_CONTROL_CODEC_ENV, "postcard");
//! }
//!
//! let codec = control_codec_from_env()?;
//! assert_eq!(codec, PorkControlCodec::Postcard);
//!
//! unsafe {
//!     std::env::remove_var(PORK_CONTROL_CODEC_ENV);
//! }
//! # Ok::<(), pork_proto::ParsePorkControlCodecError>(())
//! ```
//!
//! If the environment variable is missing, `control_codec_from_env()` falls back
//! to `PorkControlCodec::Json`.
//!
//! The crate also documents each public item individually so generated API docs
//! are useful when browsing the crate from docs.rs or an IDE.
#![deny(missing_docs)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(unsafe_code)]
use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

/// Compatibility module for protocol helpers shared with client-side consumers.
pub mod client;

/// Environment variable used by parent and child processes to agree on the
/// control-message codec.
pub const PORK_CONTROL_CODEC_ENV: &str = "PORK_CONTROL_CODEC";

/// Host-to-child and child-to-host control-plane messages shared by all Pork-based
/// IPC protocols.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PorkControlMessage {
    /// Ask the remote process to terminate gracefully.
    GracefulShutdown,
}

/// Shared wrapper for Pork IPC traffic.
///
/// `Control` is reserved for framework-level coordination between host and child.
/// `Custom` allows applications to embed their own protocol payloads while reusing
/// the shared control plane.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PorkIpcMessage<T> {
    /// A framework-defined control message.
    Control(PorkControlMessage),
    /// An application-defined payload.
    Custom(T),
}

/// Errors returned by built-in codec helpers.
#[derive(Debug)]
pub enum PorkProtoCodecError {
    /// JSON encoding or decoding failed.
    #[cfg(feature = "codec-json")]
    Json(serde_json::Error),
    /// Postcard encoding or decoding failed.
    #[cfg(feature = "codec-postcard")]
    Postcard(String),
    /// The requested codec is unavailable because the corresponding feature is disabled
    /// or the payload shape does not match the requested control-message operation.
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

    /// Deserializes a control message using the selected codec.
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

/// Returns `true` when the given bytes decode to a graceful-shutdown control message
/// with the provided codec.
pub fn is_graceful_shutdown_message(message: &[u8], codec: PorkControlCodec) -> bool {
    codec.is_graceful_shutdown_message(message)
}

/// Builds a serialized graceful-shutdown control message for the provided codec.
pub fn graceful_shutdown_message(codec: PorkControlCodec) -> Vec<u8> {
    codec
        .encode_graceful_shutdown()
        .expect("serializing graceful shutdown control message should never fail")
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

/// Reads a child bootstrap value from the given environment variable name.
pub fn child_bootstrap_env_value(env_name: &str) -> Result<String, std::env::VarError> {
    std::env::var(env_name)
}

/// Resolves the child control codec from the process environment.
pub fn child_control_codec_from_env() -> Result<PorkControlCodec, ParsePorkControlCodecError> {
    control_codec_from_env()
}

/// Shared codec trait for feature-gated built-in encoders.
///
/// Implementations are provided by the built-in codec marker types in the
/// `json` and `postcard` modules when the respective feature is enabled.
pub trait PorkCodec {
    /// Serializes a wrapped IPC message into the codec's wire format.
    fn encode<T>(message: &PorkIpcMessage<T>) -> Result<Vec<u8>, PorkProtoCodecError>
    where
        T: Serialize;

    /// Deserializes a wrapped IPC message from the codec's wire format.
    fn decode<T>(bytes: &[u8]) -> Result<PorkIpcMessage<T>, PorkProtoCodecError>
    where
        T: for<'de> Deserialize<'de>;
}

/// JSON codec helpers for [`PorkIpcMessage`] payloads.
#[cfg(feature = "codec-json")]
pub mod json {
    use super::{PorkCodec, PorkIpcMessage, PorkProtoCodecError};
    use serde::{Deserialize, Serialize};

    /// Marker type implementing [`PorkCodec`] with `serde_json`.
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

    /// Serializes a wrapped IPC message as JSON.
    pub fn encode<T>(message: &PorkIpcMessage<T>) -> Result<Vec<u8>, PorkProtoCodecError>
    where
        T: Serialize,
    {
        JsonCodec::encode(message)
    }

    /// Deserializes a wrapped IPC message from JSON.
    pub fn decode<T>(bytes: &[u8]) -> Result<PorkIpcMessage<T>, PorkProtoCodecError>
    where
        T: for<'de> Deserialize<'de>,
    {
        JsonCodec::decode(bytes)
    }
}

/// Postcard codec helpers for [`PorkIpcMessage`] payloads.
#[cfg(feature = "codec-postcard")]
pub mod postcard {
    use super::{PorkCodec, PorkIpcMessage, PorkProtoCodecError};
    use serde::{Deserialize, Serialize};

    /// Marker type implementing [`PorkCodec`] with `postcard`.
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

    /// Serializes a wrapped IPC message as Postcard.
    pub fn encode<T>(message: &PorkIpcMessage<T>) -> Result<Vec<u8>, PorkProtoCodecError>
    where
        T: Serialize,
    {
        PostcardCodec::encode(message)
    }

    /// Deserializes a wrapped IPC message from Postcard.
    pub fn decode<T>(bytes: &[u8]) -> Result<PorkIpcMessage<T>, PorkProtoCodecError>
    where
        T: for<'de> Deserialize<'de>,
    {
        PostcardCodec::decode(bytes)
    }
}
