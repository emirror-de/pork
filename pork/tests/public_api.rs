use pork::spec::ProcessSpec;
use pork::DEFAULT_BOOTSTRAP_ENV;
use pork_proto::protocol::PorkControlCodec;

#[test]
fn process_spec_exposes_configured_fields_through_accessors() {
    let spec = ProcessSpec::new("/usr/bin/example-child")
        .managed_name("worker-a")
        .control_codec(PorkControlCodec::Postcard)
        .args(["--serve", "--foreground"])
        .current_dir("/tmp")
        .env("PORK_MODE", "test")
        .envs([("PORK_REGION", "local"), ("PORK_LOG_LEVEL", "debug")])
        .bootstrap_env("CUSTOM_BOOTSTRAP_ENV")
        .capture_stdout(true)
        .capture_stderr(true)
        .depends_on("upstream-service")
        .depends_on_all(["database", "cache"]);

    assert_eq!(
        spec.executable().as_os_str(),
        std::path::Path::new("/usr/bin/example-child").as_os_str()
    );
    assert_eq!(spec.managed_name_ref(), Some("worker-a"));
    assert_eq!(spec.control_codec_ref(), PorkControlCodec::Postcard);
    assert_eq!(spec.args_ref(), ["--serve", "--foreground"]);
    assert_eq!(
        spec.current_dir_ref().map(|path| path.as_os_str()),
        Some(std::path::Path::new("/tmp").as_os_str())
    );
    assert_eq!(
        spec.env_ref().get("PORK_MODE").map(String::as_str),
        Some("test")
    );
    assert_eq!(
        spec.env_ref().get("PORK_REGION").map(String::as_str),
        Some("local")
    );
    assert_eq!(
        spec.env_ref().get("PORK_LOG_LEVEL").map(String::as_str),
        Some("debug")
    );
    assert_eq!(spec.bootstrap_env_ref(), "CUSTOM_BOOTSTRAP_ENV");
    assert!(spec.captures_stdout());
    assert!(spec.captures_stderr());
    assert_eq!(
        spec.depends_on_ref(),
        ["upstream-service", "database", "cache"]
    );
}

#[test]
fn process_spec_defaults_are_predictable_and_documented() {
    let spec = ProcessSpec::new("child-binary");

    assert_eq!(
        spec.executable().as_os_str(),
        std::path::Path::new("child-binary").as_os_str()
    );
    assert_eq!(spec.managed_name_ref(), None);
    assert_eq!(spec.control_codec_ref(), PorkControlCodec::Json);
    assert!(spec.args_ref().is_empty());
    assert!(spec.current_dir_ref().is_none());
    assert!(spec.env_ref().is_empty());
    assert_eq!(spec.bootstrap_env_ref(), DEFAULT_BOOTSTRAP_ENV);
    assert!(!spec.captures_stdout());
    assert!(!spec.captures_stderr());
    assert!(spec.depends_on_ref().is_empty());
}

#[test]
fn process_spec_output_capture_helpers_toggle_both_streams() {
    let captured = ProcessSpec::new("child-binary").capture_output();
    assert!(captured.captures_stdout());
    assert!(captured.captures_stderr());

    let uncaptured = captured.without_output_capture();
    assert!(!uncaptured.captures_stdout());
    assert!(!uncaptured.captures_stderr());
}

#[test]
fn process_spec_without_managed_name_clears_previous_value() {
    let spec = ProcessSpec::new("child-binary")
        .managed_name("temporary-name")
        .without_managed_name();

    assert_eq!(spec.managed_name_ref(), None);
}
