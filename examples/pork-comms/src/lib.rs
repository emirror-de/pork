#![deny(missing_docs)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(unsafe_code)]

//! Shared helpers for the `pork-comms` example binaries.
//!
//! The examples intentionally use a tiny line-based UTF-8 protocol so the host
//! and child binaries can demonstrate Pork's raw `Vec<u8>` messaging surface
//! without introducing extra dependencies or distracting serialization code.

use std::fmt;

/// Message sent from the host example to the child example.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HostMessage {
    /// Ask the child to echo the provided text back to the host.
    Echo(String),
    /// Ask the child to report a small status summary.
    Status,
}

impl HostMessage {
    /// Encodes the message into raw bytes for Pork IPC transport.
    pub fn encode(&self) -> Vec<u8> {
        match self {
            Self::Echo(value) => format!("echo:{value}").into_bytes(),
            Self::Status => b"status".to_vec(),
        }
    }

    /// Decodes a host message from raw Pork IPC bytes.
    pub fn decode(bytes: &[u8]) -> Result<Self, MessageError> {
        let text = decode_utf8(bytes)?;
        if text == "status" {
            return Ok(Self::Status);
        }

        if let Some(value) = text.strip_prefix("echo:") {
            return Ok(Self::Echo(value.to_owned()));
        }

        Err(MessageError::UnknownMessage(text))
    }
}

/// Message sent from the child example back to the host example.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChildMessage {
    /// Reports that the child connected successfully and which control codec it sees.
    Ready {
        /// Control codec exported by the host for shared control messages.
        codec: String,
    },
    /// Echo response for a previous host message.
    Echoed(String),
    /// Status summary returned by the child.
    Status {
        /// Operating-system process id of the child process.
        pid: u32,
        /// Number of host messages the child has processed so far.
        handled_messages: usize,
        /// Control codec exported by the host for shared control messages.
        codec: String,
    },
}

impl ChildMessage {
    /// Encodes the message into raw bytes for Pork IPC transport.
    pub fn encode(&self) -> Vec<u8> {
        match self {
            Self::Ready { codec } => format!("ready:{codec}").into_bytes(),
            Self::Echoed(value) => format!("echoed:{value}").into_bytes(),
            Self::Status {
                pid,
                handled_messages,
                codec,
            } => format!("status:{pid}:{handled_messages}:{codec}").into_bytes(),
        }
    }

    /// Decodes a child message from raw Pork IPC bytes.
    pub fn decode(bytes: &[u8]) -> Result<Self, MessageError> {
        let text = decode_utf8(bytes)?;

        if let Some(codec) = text.strip_prefix("ready:") {
            return Ok(Self::Ready {
                codec: codec.to_owned(),
            });
        }

        if let Some(value) = text.strip_prefix("echoed:") {
            return Ok(Self::Echoed(value.to_owned()));
        }

        if let Some(payload) = text.strip_prefix("status:") {
            let mut parts = payload.splitn(3, ':');
            let pid = parts
                .next()
                .ok_or_else(|| MessageError::InvalidStatusPayload(payload.to_owned()))?
                .parse::<u32>()
                .map_err(|_| MessageError::InvalidStatusPayload(payload.to_owned()))?;
            let handled_messages = parts
                .next()
                .ok_or_else(|| MessageError::InvalidStatusPayload(payload.to_owned()))?
                .parse::<usize>()
                .map_err(|_| MessageError::InvalidStatusPayload(payload.to_owned()))?;
            let codec = parts
                .next()
                .ok_or_else(|| MessageError::InvalidStatusPayload(payload.to_owned()))?
                .to_owned();

            return Ok(Self::Status {
                pid,
                handled_messages,
                codec,
            });
        }

        Err(MessageError::UnknownMessage(text))
    }
}

/// Error returned when an example message cannot be decoded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MessageError {
    /// The bytes were not valid UTF-8.
    InvalidUtf8,
    /// The message did not match any known example message shape.
    UnknownMessage(String),
    /// The child status payload was malformed.
    InvalidStatusPayload(String),
}

impl fmt::Display for MessageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidUtf8 => write!(f, "message is not valid UTF-8"),
            Self::UnknownMessage(message) => write!(f, "unknown message: {message}"),
            Self::InvalidStatusPayload(payload) => {
                write!(f, "invalid status payload: {payload}")
            }
        }
    }
}

impl std::error::Error for MessageError {}

fn decode_utf8(bytes: &[u8]) -> Result<String, MessageError> {
    std::str::from_utf8(bytes)
        .map(str::to_owned)
        .map_err(|_| MessageError::InvalidUtf8)
}

#[cfg(test)]
mod tests {
    use super::{ChildMessage, HostMessage, MessageError};

    #[test]
    fn host_echo_round_trip() {
        let encoded = HostMessage::Echo("hello".to_owned()).encode();
        let decoded = HostMessage::decode(&encoded);
        assert_eq!(decoded, Ok(HostMessage::Echo("hello".to_owned())));
    }

    #[test]
    fn host_status_round_trip() {
        let encoded = HostMessage::Status.encode();
        let decoded = HostMessage::decode(&encoded);
        assert_eq!(decoded, Ok(HostMessage::Status));
    }

    #[test]
    fn child_ready_round_trip() {
        let encoded = ChildMessage::Ready {
            codec: "json".to_owned(),
        }
        .encode();
        let decoded = ChildMessage::decode(&encoded);
        assert_eq!(
            decoded,
            Ok(ChildMessage::Ready {
                codec: "json".to_owned()
            })
        );
    }

    #[test]
    fn child_status_round_trip() {
        let encoded = ChildMessage::Status {
            pid: 42,
            handled_messages: 3,
            codec: "postcard".to_owned(),
        }
        .encode();
        let decoded = ChildMessage::decode(&encoded);
        assert_eq!(
            decoded,
            Ok(ChildMessage::Status {
                pid: 42,
                handled_messages: 3,
                codec: "postcard".to_owned(),
            })
        );
    }

    #[test]
    fn decoding_rejects_unknown_messages() {
        let decoded = HostMessage::decode(b"wat");
        assert_eq!(decoded, Err(MessageError::UnknownMessage("wat".to_owned())));
    }

    #[test]
    fn decoding_rejects_invalid_status_payload() {
        let decoded = ChildMessage::decode(b"status:not-a-pid:3:json");
        assert_eq!(
            decoded,
            Err(MessageError::InvalidStatusPayload(
                "not-a-pid:3:json".to_owned()
            ))
        );
    }
}
