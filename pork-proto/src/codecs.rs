use serde::{Deserialize, Serialize};

use crate::{PorkCodec, PorkIpcMessage, PorkProtoCodecError};

/// JSON codec implementations for [`PorkIpcMessage`] payloads.
#[cfg(feature = "codec-json")]
pub mod json {
    use super::{Deserialize, PorkCodec, PorkIpcMessage, PorkProtoCodecError, Serialize};

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
}

/// Postcard codec implementations for [`PorkIpcMessage`] payloads.
#[cfg(feature = "codec-postcard")]
pub mod postcard {
    use super::{Deserialize, PorkCodec, PorkIpcMessage, PorkProtoCodecError, Serialize};

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
}
