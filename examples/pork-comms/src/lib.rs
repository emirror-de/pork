#![deny(missing_docs)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(unsafe_code)]

//! Shared helpers for the `pork-comms` example binaries.
//!
//! This example uses `pork-proto`'s shared IPC envelope and codec helpers
//! directly instead of a custom byte framing format. Application messages are
//! wrapped in [`pork_proto::protocol::PorkIpcMessage`] and encoded with the
//! control codec chosen by the host process.

use pork_proto::codecs::{JsonCodec, PostcardCodec};
use pork_proto::protocol::{PorkCodec, PorkControlCodec, PorkIpcMessage, PorkProtoCodecError};
use serde::{Deserialize, Serialize};

/// Message sent from the host example to the child example.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum HostMessage {
    /// Ask the child to echo the provided text back to the host.
    Echo(String),
    /// Ask the child to report a small status summary.
    Status,
}

/// Message sent from the child example back to the host example.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

/// Encodes an application-defined message into Pork's shared IPC envelope using
/// the selected control codec.
pub fn encode_message<T>(
    codec: PorkControlCodec,
    message: T,
) -> Result<Vec<u8>, PorkProtoCodecError>
where
    T: Serialize,
{
    let envelope = PorkIpcMessage::custom(message);

    match codec {
        PorkControlCodec::Json => JsonCodec::encode(&envelope),
        PorkControlCodec::Postcard => PostcardCodec::encode(&envelope),
    }
}

/// Decodes an application-defined message from Pork's shared IPC envelope using
/// the selected control codec.
///
/// Returns `Ok(None)` when the payload is a Pork control message rather than an
/// application-defined payload.
pub fn decode_message<T>(
    codec: PorkControlCodec,
    bytes: &[u8],
) -> Result<Option<T>, PorkProtoCodecError>
where
    T: for<'de> Deserialize<'de>,
{
    let envelope = match codec {
        PorkControlCodec::Json => JsonCodec::decode(bytes)?,
        PorkControlCodec::Postcard => PostcardCodec::decode(bytes)?,
    };

    Ok(envelope.into_custom())
}

#[cfg(test)]
mod tests {
    use super::{ChildMessage, HostMessage, decode_message, encode_message};
    use pork_proto::protocol::PorkControlCodec;

    #[test]
    fn host_echo_round_trip_with_json_codec() {
        let encoded = encode_message(
            PorkControlCodec::Json,
            HostMessage::Echo("hello".to_owned()),
        );

        assert!(encoded.is_ok());

        let decoded =
            encoded.and_then(|bytes| decode_message::<HostMessage>(PorkControlCodec::Json, &bytes));

        assert!(decoded.is_ok());

        let decoded = decoded.ok().flatten();
        assert_eq!(decoded, Some(HostMessage::Echo("hello".to_owned())));
    }

    #[test]
    fn host_status_round_trip_with_postcard_codec() {
        let encoded = encode_message(PorkControlCodec::Postcard, HostMessage::Status);

        assert!(encoded.is_ok());

        let decoded = encoded
            .and_then(|bytes| decode_message::<HostMessage>(PorkControlCodec::Postcard, &bytes));

        assert!(decoded.is_ok());

        let decoded = decoded.ok().flatten();
        assert_eq!(decoded, Some(HostMessage::Status));
    }

    #[test]
    fn child_ready_round_trip_with_json_codec() {
        let encoded = encode_message(
            PorkControlCodec::Json,
            ChildMessage::Ready {
                codec: "json".to_owned(),
            },
        );

        assert!(encoded.is_ok());

        let decoded = encoded
            .and_then(|bytes| decode_message::<ChildMessage>(PorkControlCodec::Json, &bytes));

        assert!(decoded.is_ok());

        let decoded = decoded.ok().flatten();
        assert_eq!(
            decoded,
            Some(ChildMessage::Ready {
                codec: "json".to_owned(),
            })
        );
    }

    #[test]
    fn child_status_round_trip_with_postcard_codec() {
        let encoded = encode_message(
            PorkControlCodec::Postcard,
            ChildMessage::Status {
                pid: 42,
                handled_messages: 3,
                codec: "postcard".to_owned(),
            },
        );

        assert!(encoded.is_ok());

        let decoded = encoded
            .and_then(|bytes| decode_message::<ChildMessage>(PorkControlCodec::Postcard, &bytes));

        assert!(decoded.is_ok());

        let decoded = decoded.ok().flatten();
        assert_eq!(
            decoded,
            Some(ChildMessage::Status {
                pid: 42,
                handled_messages: 3,
                codec: "postcard".to_owned(),
            })
        );
    }

    #[test]
    fn decoding_control_message_returns_none_for_custom_payload() {
        let encoded = PorkControlCodec::Json.encode_graceful_shutdown();

        assert!(encoded.is_ok());

        let decoded =
            encoded.and_then(|bytes| decode_message::<HostMessage>(PorkControlCodec::Json, &bytes));

        assert!(decoded.is_ok());

        let decoded = decoded.ok().flatten();
        assert_eq!(decoded, None);
    }
}
