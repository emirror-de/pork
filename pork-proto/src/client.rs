use std::fmt;
use std::str::FromStr;

use crate::{PorkControlMessage, PorkIpcMessage, PorkProtoCodecError};

pub const PORK_CONTROL_CODEC_ENV: &str = "PORK_CONTROL_CODEC";

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
            Self::Json => crate::json::encode(&message),
            Self::Postcard => crate::postcard::encode(&message),
        }
    }

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

pub fn graceful_shutdown_message(codec: PorkControlCodec) -> Vec<u8> {
    codec
        .encode_graceful_shutdown()
        .expect("serializing graceful shutdown control message should never fail")
}

pub fn is_graceful_shutdown_message(message: &[u8], codec: PorkControlCodec) -> bool {
    codec.is_graceful_shutdown_message(message)
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
