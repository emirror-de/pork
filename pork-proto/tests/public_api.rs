use pork_proto::protocol::{
    PorkControlCodec, PorkControlMessage, PorkIpcMessage, PorkProtoCodecError,
};

#[cfg(any(feature = "codec-json", feature = "codec-postcard"))]
use pork_proto::protocol::{PorkChildStatus, PorkStatusUpdate};

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
    let parsed = "xml".parse::<PorkControlCodec>();
    assert!(parsed.is_err());

    let error = match parsed {
        Ok(codec) => panic!("unsupported codec value should fail, got {codec:?}"),
        Err(error) => error,
    };

    assert_eq!(error.value(), "xml");
    assert_eq!(error.to_string(), "unsupported control codec 'xml'");
}

#[test]
fn lifecycle_helpers_match_control_messages() {
    assert!(PorkControlCodec::Json.is_graceful_shutdown(&PorkControlMessage::GracefulShutdown));
    assert!(PorkControlCodec::Postcard.is_graceful_shutdown(&PorkControlMessage::GracefulShutdown));
    assert!(PorkControlCodec::Json.is_restart(&PorkControlMessage::Restart));
    assert!(PorkControlCodec::Postcard.is_restart(&PorkControlMessage::Restart));
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
        let result =
            PorkControlCodec::Json.encode_control_message(PorkControlMessage::GracefulShutdown);

        match result {
            Ok(_) => panic!("json codec should be unavailable without feature"),
            Err(error) => assert!(matches!(error, PorkProtoCodecError::UnsupportedCodec)),
        }
    }

    #[cfg(not(feature = "codec-postcard"))]
    {
        let result =
            PorkControlCodec::Postcard.encode_control_message(PorkControlMessage::GracefulShutdown);

        match result {
            Ok(_) => panic!("postcard codec should be unavailable without feature"),
            Err(error) => assert!(matches!(error, PorkProtoCodecError::UnsupportedCodec)),
        }
    }
}

#[test]
fn decode_control_message_reports_unavailable_codec_features() {
    #[cfg(not(feature = "codec-json"))]
    {
        let bytes = b"not-a-valid-control-message";
        let result = PorkControlCodec::Json.decode_control_message(bytes);

        match result {
            Ok(_) => panic!("json codec should be unavailable without feature"),
            Err(error) => assert!(matches!(error, PorkProtoCodecError::UnsupportedCodec)),
        }
        assert!(!PorkControlCodec::Json.is_graceful_shutdown_message(bytes));
    }

    #[cfg(not(feature = "codec-postcard"))]
    {
        let bytes = b"not-a-valid-control-message";
        let result = PorkControlCodec::Postcard.decode_control_message(bytes);

        match result {
            Ok(_) => panic!("postcard codec should be unavailable without feature"),
            Err(error) => assert!(matches!(error, PorkProtoCodecError::UnsupportedCodec)),
        }
        assert!(!PorkControlCodec::Postcard.is_graceful_shutdown_message(bytes));
    }
}

#[cfg(feature = "codec-json")]
#[test]
fn json_codec_round_trips_graceful_shutdown_control_messages() {
    let encoded = PorkControlCodec::Json.encode_graceful_shutdown();
    assert!(encoded.is_ok());

    let bytes = match encoded {
        Ok(bytes) => bytes,
        Err(error) => panic!("json graceful shutdown encoding should succeed: {error}"),
    };

    let decoded_result = PorkControlCodec::Json.decode_control_message(&bytes);
    assert!(decoded_result.is_ok());

    let decoded = match decoded_result {
        Ok(message) => message,
        Err(error) => panic!("json graceful shutdown decoding should succeed: {error}"),
    };

    assert_eq!(decoded, PorkControlMessage::GracefulShutdown);
    assert!(PorkControlCodec::Json.is_graceful_shutdown(&decoded));
    assert!(PorkControlCodec::Json.is_graceful_shutdown_message(&bytes));
}

#[cfg(feature = "codec-postcard")]
#[test]
fn postcard_codec_round_trips_graceful_shutdown_control_messages() {
    let encoded = PorkControlCodec::Postcard.encode_graceful_shutdown();
    assert!(encoded.is_ok());

    let bytes = match encoded {
        Ok(bytes) => bytes,
        Err(error) => panic!("postcard graceful shutdown encoding should succeed: {error}"),
    };

    let decoded_result = PorkControlCodec::Postcard.decode_control_message(&bytes);
    assert!(decoded_result.is_ok());

    let decoded = match decoded_result {
        Ok(message) => message,
        Err(error) => panic!("postcard graceful shutdown decoding should succeed: {error}"),
    };

    assert_eq!(decoded, PorkControlMessage::GracefulShutdown);
    assert!(PorkControlCodec::Postcard.is_graceful_shutdown(&decoded));
    assert!(PorkControlCodec::Postcard.is_graceful_shutdown_message(&bytes));
}

#[cfg(feature = "codec-json")]
#[test]
fn json_codec_round_trips_restart_and_status_control_messages() {
    assert_control_round_trip(PorkControlCodec::Json, PorkControlMessage::Restart);
    assert_control_round_trip(
        PorkControlCodec::Json,
        PorkControlMessage::StatusUpdate(PorkStatusUpdate {
            status: PorkChildStatus::Running,
            timestamp_ms: 42,
        }),
    );

    let encoded = PorkControlCodec::Json.encode_restart();
    assert!(matches!(encoded, Ok(bytes) if PorkControlCodec::Json.is_restart_message(&bytes)));
}

#[cfg(feature = "codec-postcard")]
#[test]
fn postcard_codec_round_trips_restart_and_status_control_messages() {
    assert_control_round_trip(PorkControlCodec::Postcard, PorkControlMessage::Restart);
    assert_control_round_trip(
        PorkControlCodec::Postcard,
        PorkControlMessage::StatusUpdate(PorkStatusUpdate {
            status: PorkChildStatus::Running,
            timestamp_ms: 42,
        }),
    );

    let encoded = PorkControlCodec::Postcard.encode_restart();
    assert!(matches!(encoded, Ok(bytes) if PorkControlCodec::Postcard.is_restart_message(&bytes)));
}

#[cfg(any(feature = "codec-json", feature = "codec-postcard"))]
fn assert_control_round_trip(codec: PorkControlCodec, message: PorkControlMessage) {
    let encoded = codec.encode_control_message(message.clone());
    let bytes = match encoded {
        Ok(bytes) => bytes,
        Err(error) => panic!("control-message encoding should succeed: {error}"),
    };
    let decoded = codec.decode_control_message(&bytes);

    assert_eq!(decoded.ok(), Some(message));
}

#[cfg(feature = "codec-json")]
#[test]
fn json_codec_rejects_custom_payloads_as_control_messages() {
    use pork_proto::codecs::json::JsonCodec;
    use pork_proto::protocol::PorkCodec;

    let encoded = JsonCodec::encode(&PorkIpcMessage::custom(String::from("ping")));
    assert!(encoded.is_ok());

    let bytes = match encoded {
        Ok(bytes) => bytes,
        Err(error) => panic!("json custom payload encoding should succeed: {error}"),
    };

    let decoded = PorkControlCodec::Json.decode_control_message(&bytes);
    assert!(decoded.is_err());

    let error = match decoded {
        Ok(message) => panic!("custom payload should not decode as a control message: {message:?}"),
        Err(error) => error,
    };

    assert!(matches!(
        error,
        PorkProtoCodecError::UnsupportedCodec | PorkProtoCodecError::Json(_)
    ));
    assert!(!PorkControlCodec::Json.is_graceful_shutdown_message(&bytes));
}

#[cfg(feature = "codec-postcard")]
#[test]
fn postcard_codec_rejects_custom_payloads_as_control_messages() {
    use pork_proto::codecs::postcard::PostcardCodec;
    use pork_proto::protocol::PorkCodec;

    let encoded = PostcardCodec::encode(&PorkIpcMessage::custom(String::from("ping")));
    assert!(encoded.is_ok());

    let bytes = match encoded {
        Ok(bytes) => bytes,
        Err(error) => panic!("postcard custom payload encoding should succeed: {error}"),
    };

    let decoded = PorkControlCodec::Postcard.decode_control_message(&bytes);
    assert!(decoded.is_err());

    let error = match decoded {
        Ok(message) => panic!("custom payload should not decode as a control message: {message:?}"),
        Err(error) => error,
    };

    assert!(matches!(error, PorkProtoCodecError::UnsupportedCodec));
    assert!(!PorkControlCodec::Postcard.is_graceful_shutdown_message(&bytes));
}
