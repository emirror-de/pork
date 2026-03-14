/// JSON codec implementation.
#[cfg(feature = "codec-json")]
mod json;

/// Postcard codec implementation.
#[cfg(feature = "codec-postcard")]
mod postcard;

#[cfg(feature = "codec-json")]
pub use json::JsonCodec;
#[cfg(feature = "codec-postcard")]
pub use postcard::PostcardCodec;
