use serde::{Deserialize, Serialize};

use crate::protocol::{PorkCodec, PorkIpcMessage, PorkProtoCodecError};

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
