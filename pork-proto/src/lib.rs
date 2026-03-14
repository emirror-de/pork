//! Shared protocol primitives for applications built on top of `pork`.
//!
//! This crate defines the control-plane messages exchanged between a parent
//! process and a managed child process, plus optional built-in codecs for
//! serializing those messages.
//!
//! # What this crate gives you
//!
//! - `protocol::PorkControlMessage` for framework-level coordination
//! - `protocol::PorkIpcMessage<T>` for wrapping your own protocol payloads
//! - `protocol::PorkControlCodec` for choosing how control messages are encoded
//! - feature-gated codec implementations in [`codecs`] for custom payloads
//! - environment helpers so parent and child agree on the selected codec
//!
//! # Feature flags
//!
//! - `codec-json` enables the JSON codec implementation
//! - `codec-postcard` enables the Postcard codec implementation
//!
//! You can enable one or both features depending on the transport format you
//! want to expose to consumers.
//!
//! # Example: wrap your own messages
//!
//! ```rust
//! use pork_proto::protocol::{PorkControlMessage, PorkIpcMessage};
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
//! use pork_proto::protocol::{PorkControlCodec, PorkControlMessage};
//!
//! let codec = PorkControlCodec::Json;
//! let bytes = codec.encode_control_message(PorkControlMessage::GracefulShutdown)?;
//! let message = codec.decode_control_message(&bytes)?;
//!
//! assert_eq!(message, PorkControlMessage::GracefulShutdown);
//! # Ok::<(), pork_proto::protocol::PorkProtoCodecError>(())
//! ```
//!
//! # Example: serialize your own payload with the JSON codec
//!
//! ```rust
//! # #[cfg(feature = "codec-json")]
//! # fn main() -> Result<(), pork_proto::protocol::PorkProtoCodecError> {
//! use pork_proto::codecs::JsonCodec;
//! use pork_proto::protocol::{PorkCodec, PorkIpcMessage};
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
//! let bytes = JsonCodec::encode(&original)?;
//! let decoded: PorkIpcMessage<AppMessage> = JsonCodec::decode(&bytes)?;
//!
//! assert_eq!(decoded, original);
//! # Ok(())
//! # }
//! ```
//!
//! # Example: agree on a codec through the environment
//!
//! The parent process can write the selected codec into
//! `protocol::PORK_CONTROL_CODEC_ENV`, and the child can read it back with
//! `protocol::control_codec_from_env()`.
//!
//! ```rust
//! use pork_proto::protocol::{control_codec_from_env, PORK_CONTROL_CODEC_ENV, PorkControlCodec};
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
//! # Ok::<(), pork_proto::protocol::ParsePorkControlCodecError>(())
//! ```
//!
//! If the environment variable is missing, `control_codec_from_env()` falls back
//! to `PorkControlCodec::Json`.
#![deny(missing_docs)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(unsafe_code)]

/// Feature-gated codec implementations for encoding and decoding
/// [`protocol::PorkIpcMessage`] payloads through concrete [`protocol::PorkCodec`]
/// implementations such as [`codecs::JsonCodec`] and
/// [`codecs::PostcardCodec`].
pub mod codecs;

/// Shared protocol models, codec selection, parsing, and environment helpers.
pub mod protocol {
    use std::fmt;
    use std::str::FromStr;

    use serde::{Deserialize, Serialize};

    /// Environment variable used by parent and child processes to agree on the
    /// control-message codec.
    pub const PORK_CONTROL_CODEC_ENV: &str = "PORK_CONTROL_CODEC";

    /// Host-to-child and child-to-host control-plane messages shared by all
    /// Pork-based IPC protocols.
    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    pub enum PorkControlMessage {
        /// Ask the remote process to terminate gracefully.
        GracefulShutdown,
    }

    /// Shared wrapper for Pork IPC traffic.
    ///
    /// `Control` is reserved for framework-level coordination between host and
    /// child. `Custom` allows applications to embed their own protocol payloads
    /// while reusing the shared control plane.
    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    pub enum PorkIpcMessage<T> {
        /// A framework-defined control message.
        Control(PorkControlMessage),
        /// An application-defined payload.
        Custom(T),
    }

    impl<T> PorkIpcMessage<T> {
        /// Creates a new framework-level control message wrapper.
        pub fn control(message: PorkControlMessage) -> Self {
            Self::Control(message)
        }

        /// Creates a new application-defined payload wrapper.
        pub fn custom(message: T) -> Self {
            Self::Custom(message)
        }

        /// Returns the wrapped control message when this is a control envelope.
        pub fn as_control(&self) -> Option<&PorkControlMessage> {
            match self {
                Self::Control(message) => Some(message),
                Self::Custom(_) => None,
            }
        }

        /// Returns the wrapped custom payload when this is an application envelope.
        pub fn as_custom(&self) -> Option<&T> {
            match self {
                Self::Control(_) => None,
                Self::Custom(message) => Some(message),
            }
        }

        /// Returns `true` when this envelope contains a framework control message.
        pub fn is_control(&self) -> bool {
            matches!(self, Self::Control(_))
        }

        /// Returns `true` when this envelope contains an application-defined payload.
        pub fn is_custom(&self) -> bool {
            matches!(self, Self::Custom(_))
        }

        /// Converts the envelope into the wrapped custom payload, if present.
        pub fn into_custom(self) -> Option<T> {
            match self {
                Self::Control(_) => None,
                Self::Custom(message) => Some(message),
            }
        }
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
        /// The requested codec is unavailable because the corresponding feature
        /// is disabled or the payload shape does not match the requested
        /// control-message operation.
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
        pub(crate) fn from_postcard_error<E>(value: E) -> Self
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
            let message = PorkIpcMessage::<Vec<u8>>::control(message);

            match self {
                Self::Json => {
                    #[cfg(feature = "codec-json")]
                    {
                        crate::codecs::JsonCodec::encode(&message)
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
                        crate::codecs::PostcardCodec::encode(&message)
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
            match self {
                Self::Json => {
                    #[cfg(feature = "codec-json")]
                    {
                        match crate::codecs::JsonCodec::decode::<Vec<u8>>(bytes)? {
                            PorkIpcMessage::Control(control) => Ok(control),
                            PorkIpcMessage::Custom(_) => Err(PorkProtoCodecError::UnsupportedCodec),
                        }
                    }
                    #[cfg(not(feature = "codec-json"))]
                    {
                        let _ = bytes;
                        Err(PorkProtoCodecError::UnsupportedCodec)
                    }
                }
                Self::Postcard => {
                    #[cfg(feature = "codec-postcard")]
                    {
                        match crate::codecs::PostcardCodec::decode::<Vec<u8>>(bytes)? {
                            PorkIpcMessage::Control(control) => Ok(control),
                            PorkIpcMessage::Custom(_) => Err(PorkProtoCodecError::UnsupportedCodec),
                        }
                    }
                    #[cfg(not(feature = "codec-postcard"))]
                    {
                        let _ = bytes;
                        Err(PorkProtoCodecError::UnsupportedCodec)
                    }
                }
            }
        }

        /// Returns `true` when this codec is currently available in the compiled crate.
        ///
        /// A codec is available only when its corresponding feature flag is enabled.
        pub fn is_available(self) -> bool {
            match self {
                Self::Json => cfg!(feature = "codec-json"),
                Self::Postcard => cfg!(feature = "codec-postcard"),
            }
        }

        /// Returns all built-in codecs that are available in the compiled crate.
        pub fn available() -> Vec<Self> {
            [Self::Json, Self::Postcard]
                .into_iter()
                .filter(|codec| codec.is_available())
                .collect()
        }

        /// Returns `true` when the given message is a graceful-shutdown control message.
        pub fn is_graceful_shutdown(self, message: &PorkControlMessage) -> bool {
            let _ = self;
            matches!(message, PorkControlMessage::GracefulShutdown)
        }

        /// Serializes a graceful-shutdown control message using the selected codec.
        pub fn encode_graceful_shutdown(self) -> Result<Vec<u8>, PorkProtoCodecError> {
            self.encode_control_message(PorkControlMessage::GracefulShutdown)
        }

        /// Returns `true` when the given bytes decode to a graceful-shutdown
        /// control message.
        pub fn is_graceful_shutdown_message(self, bytes: &[u8]) -> bool {
            matches!(
                self.decode_control_message(bytes),
                Ok(message) if self.is_graceful_shutdown(&message)
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

    /// Reads a child bootstrap value from the given environment variable name.
    pub fn child_bootstrap_env_value(env_name: &str) -> Result<String, std::env::VarError> {
        std::env::var(env_name)
    }

    /// Shared codec trait for feature-gated built-in encoders.
    ///
    /// Implementations are provided by the built-in codec marker types in the
    /// [`crate::codecs`] module when the respective feature is enabled.
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
}
