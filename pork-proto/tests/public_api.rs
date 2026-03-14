use pork_proto::protocol::{
    PorkControlCodec, PorkControlMessage, PorkIpcMessage, PorkProtoCodecError,
};

#[test]
fn pork_ipc_message_control_helpers_expose_control_payload() {
    let message = PorkIpcMessage::<String>::control(PorkControlMessage::GracefulShutdown);

    assert!(message.is_control());
    assert!(!message.is_custom());
    assert_eq!(
        message.as_control(),
        Some(&PorkControlMessage::GracefulShutdown)
    );
    assert_eq!(message.as_custom(), None);
}

#[test]
fn pork_ipc_message_custom_helpers_expose_custom_payload() {
    let message = PorkIpcMessage::custom(String::from("ping"));

    assert!(message.is_custom());
    assert!(!message.is_control());
    assert_eq!(message.as_control(), None);
    assert_eq!(message.as_custom().map(String::as_str), Some("ping"));
    assert_eq!(message.clone().into_custom(), Some(String::from("ping")));
}

#[test]
fn pork_control_codec_env_values_and_display_are_stable() {
    assert_eq!(PorkControlCodec::Json.as_env_value(), "json");
    assert_eq!(PorkControlCodec::Postcard.as_env_value(), "postcard");
    assert_eq!(PorkControlCodec::Json.to_string(), "json");
    assert_eq!(PorkControlCodec::Postcard.to_string(), "postcard");
}

#[test]
fn pork_control_codec_parse_accepts_supported_values() {
    assert_eq!("json".parse(), Ok(PorkControlCodec::Json));
    assert_eq!("postcard".parse(), Ok(PorkControlCodec::Postcard));
}

#[test]
fn pork_control_codec_parse_rejects_unknown_values_with_original_input() {
    let error = "xml"
        .parse::<PorkControlCodec>()
        .expect_err("unsupported codec value should fail");

    assert_eq!(error, "xml".parse::<PorkControlCodec>().unwrap_err());
    assert_eq!(error.value(), "xml");
    assert_eq!(error.to_string(), "unsupported control codec 'xml'");
}

#[test]
fn graceful_shutdown_helper_matches_control_message() {
    assert!(PorkControlCodec::Json.is_graceful_shutdown(&PorkControlMessage::GracefulShutdown));
    assert!(PorkControlCodec::Postcard.is_graceful_shutdown(&PorkControlMessage::GracefulShutdown));
}

#[test]
fn available_codecs_match_feature_flags() {
    let available = PorkControlCodec::available();

    #[cfg(feature = "codec-json")]
    {
        assert!(available.contains(&PorkControlCodec::Json));
        assert!(PorkControlCodec::Json.is_available());
    }

    #[cfg(not(feature = "codec-json"))]
    {
        assert!(!available.contains(&PorkControlCodec::Json));
        assert!(!PorkControlCodec::Json.is_available());
    }

    #[cfg(feature = "codec-postcard")]
    {
        assert!(available.contains(&PorkControlCodec::Postcard));
        assert!(PorkControlCodec::Postcard.is_available());
    }

    #[cfg(not(feature = "codec-postcard"))]
    {
        assert!(!available.contains(&PorkControlCodec::Postcard));
        assert!(!PorkControlCodec::Postcard.is_available());
    }
}

#[test]
fn encode_control_message_reports_unavailable_codec_features() {
    #[cfg(not(feature = "codec-json"))]
    {
        let error = PorkControlCodec::Json
            .encode_control_message(PorkControlMessage::GracefulShutdown)
            .expect_err("json codec should be unavailable without feature");

        assert!(matches!(error, PorkProtoCodecError::UnsupportedCodec));
    }

    #[cfg(not(feature = "codec-postcard"))]
    {
        let error = PorkControlCodec::Postcard
            .encode_control_message(PorkControlMessage::GracefulShutdown)
            .expect_err("postcard codec should be unavailable without feature");

        assert!(matches!(error, PorkProtoCodecError::UnsupportedCodec));
    }
}

#[test]
fn decode_control_message_reports_unavailable_codec_features() {
    #[cfg(not(feature = "codec-json"))]
    {
        let bytes = b"not-a-valid-control-message";
        let error = PorkControlCodec::Json
            .decode_control_message(bytes)
            .expect_err("json codec should be unavailable without feature");

        assert!(matches!(error, PorkProtoCodecError::UnsupportedCodec));
        assert!(!PorkControlCodec::Json.is_graceful_shutdown_message(bytes));
    }

    #[cfg(not(feature = "codec-postcard"))]
    {
        let bytes = b"not-a-valid-control-message";
        let error = PorkControlCodec::Postcard
            .decode_control_message(bytes)
            .expect_err("postcard codec should be unavailable without feature");

        assert!(matches!(error, PorkProtoCodecError::UnsupportedCodec));
        assert!(!PorkControlCodec::Postcard.is_graceful_shutdown_message(bytes));
    }
}

#[cfg(feature = "codec-json")]
#[test]
fn json_codec_round_trips_graceful_shutdown_control_messages() {
    let bytes = PorkControlCodec::Json
        .encode_graceful_shutdown()
        .expect("json graceful shutdown encoding should succeed");
    let decoded = PorkControlCodec::Json
        .decode_control_message(&bytes)
        .expect("json graceful shutdown decoding should succeed");

    assert_eq!(decoded, PorkControlMessage::GracefulShutdown);
    assert!(PorkControlCodec::Json.is_graceful_shutdown(&decoded));
    assert!(PorkControlCodec::Json.is_graceful_shutdown_message(&bytes));
}

#[cfg(feature = "codec-postcard")]
#[test]
fn postcard_codec_round_trips_graceful_shutdown_control_messages() {
    let bytes = PorkControlCodec::Postcard
        .encode_graceful_shutdown()
        .expect("postcard graceful shutdown encoding should succeed");
    let decoded = PorkControlCodec::Postcard
        .decode_control_message(&bytes)
        .expect("postcard graceful shutdown decoding should succeed");

    assert_eq!(decoded, PorkControlMessage::GracefulShutdown);
    assert!(PorkControlCodec::Postcard.is_graceful_shutdown(&decoded));
    assert!(PorkControlCodec::Postcard.is_graceful_shutdown_message(&bytes));
}

#[cfg(feature = "codec-json")]
#[test]
fn json_codec_rejects_custom_payloads_as_control_messages() {
    use pork_proto::codecs::JsonCodec;
    use pork_proto::protocol::PorkCodec;

    let bytes = JsonCodec::encode(&PorkIpcMessage::custom(String::from("ping")))
        .expect("json custom payload encoding should succeed");

    let error = PorkControlCodec::Json
        .decode_control_message(&bytes)
        .expect_err("custom payload should not decode as a control message");

    assert!(matches!(
        error,
        PorkProtoCodecError::UnsupportedCodec | PorkProtoCodecError::Json(_)
    ));
    assert!(!PorkControlCodec::Json.is_graceful_shutdown_message(&bytes));
}

#[cfg(feature = "codec-postcard")]
#[test]
fn postcard_codec_rejects_custom_payloads_as_control_messages() {
    use pork_proto::codecs::PostcardCodec;
    use pork_proto::protocol::PorkCodec;

    let bytes = PostcardCodec::encode(&PorkIpcMessage::custom(String::from("ping")))
        .expect("postcard custom payload encoding should succeed");

    let error = PorkControlCodec::Postcard
        .decode_control_message(&bytes)
        .expect_err("custom payload should not decode as a control message");

    assert!(matches!(error, PorkProtoCodecError::UnsupportedCodec));
    assert!(!PorkControlCodec::Postcard.is_graceful_shutdown_message(&bytes));
}
