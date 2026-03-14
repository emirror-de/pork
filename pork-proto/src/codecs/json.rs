/// JSON codec implementation for [`crate::protocol::PorkIpcMessage`] payloads.
use serde::{Deserialize, Serialize};

use crate::protocol::{PorkCodec, PorkIpcMessage, PorkProtoCodecError};

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
