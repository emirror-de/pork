#[cfg(feature = "host")]
use pork::error::OrchestratorError;
#[cfg(feature = "host")]
use pork::orchestrator::ProcessOrchestrator;
use pork::orchestrator::managed_child::ManagedChildIdentity;
use pork::spec::ProcessSpec;
use pork::types::{BootstrapEnvName, ManagedChildName, ProcessId};
use pork::{CONTROL_BOOTSTRAP_ENV, DEFAULT_BOOTSTRAP_ENV};
use pork_proto::protocol::PorkControlCodec;

#[cfg(feature = "host")]
#[tokio::test]
async fn child_status_reports_unknown_process() {
    let result = ProcessOrchestrator::new()
        .child_status(ProcessId::new(42))
        .await;

    assert!(matches!(
        result,
        Err(OrchestratorError::ProcessNotFound(process_id)) if process_id == ProcessId::new(42)
    ));
}

#[cfg(feature = "host")]
#[tokio::test]
async fn child_status_by_name_reports_unknown_process() {
    let result = ProcessOrchestrator::new()
        .child_status_by_name(&ManagedChildName::from("missing"))
        .await;

    assert!(matches!(
        result,
        Err(OrchestratorError::ProcessNameNotFound(name)) if name.as_str() == "missing"
    ));
}

#[test]
fn managed_child_identity_accessors_are_predictable() {
    let worker_name = ManagedChildName::from("worker-a");
    let named = ManagedChildIdentity {
        process_id: ProcessId::new(7),
        managed_name: Some(&worker_name),
    };
    assert_eq!(named.process_id(), ProcessId::new(7));
    assert_eq!(
        named.managed_name().map(ManagedChildName::as_str),
        Some("worker-a")
    );
    assert!(named.has_name());

    let unnamed = ManagedChildIdentity {
        process_id: ProcessId::new(8),
        managed_name: None,
    };
    assert_eq!(unnamed.process_id(), ProcessId::new(8));
    assert_eq!(unnamed.managed_name(), None);
    assert!(!unnamed.has_name());
}

#[test]
fn process_spec_exposes_configured_fields_through_accessors() {
    let spec = ProcessSpec::builder("/usr/bin/example-child")
        .managed_name("worker-a")
        .control_codec(PorkControlCodec::Postcard)
        .args(["--serve", "--foreground"])
        .current_dir("/tmp")
        .env("PORK_MODE", "test")
        .envs([("PORK_REGION", "local"), ("PORK_LOG_LEVEL", "debug")])
        .data_bootstrap_env("CUSTOM_DATA_BOOTSTRAP_ENV")
        .control_bootstrap_env("CUSTOM_CONTROL_BOOTSTRAP_ENV")
        .capture_stdout(true)
        .capture_stderr(true)
        .depends_on("upstream-service")
        .depends_on_all(["database", "cache"])
        .build();

    assert_eq!(
        spec.executable().as_path().as_os_str(),
        std::path::Path::new("/usr/bin/example-child").as_os_str()
    );
    assert_eq!(
        spec.managed_name().map(ManagedChildName::as_str),
        Some("worker-a")
    );
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
    assert_eq!(
        spec.data_bootstrap_env_ref(),
        &BootstrapEnvName::from("CUSTOM_DATA_BOOTSTRAP_ENV")
    );
    assert_eq!(
        spec.control_bootstrap_env_ref(),
        &BootstrapEnvName::from("CUSTOM_CONTROL_BOOTSTRAP_ENV")
    );
    assert!(spec.captures_stdout());
    assert!(spec.captures_stderr());
    assert_eq!(
        spec.dependencies_ref(),
        [
            ManagedChildName::from("upstream-service"),
            ManagedChildName::from("database"),
            ManagedChildName::from("cache"),
        ]
    );
}

#[test]
fn process_spec_defaults_are_predictable_and_documented() {
    let spec = ProcessSpec::builder("child-binary").build();

    assert_eq!(
        spec.executable().as_path().as_os_str(),
        std::path::Path::new("child-binary").as_os_str()
    );
    assert_eq!(spec.managed_name().map(ManagedChildName::as_str), None);
    assert_eq!(spec.control_codec_ref(), PorkControlCodec::Json);
    assert!(spec.args_ref().is_empty());
    assert!(spec.current_dir_ref().is_none());
    assert!(spec.env_ref().is_empty());
    assert_eq!(
        spec.data_bootstrap_env_ref(),
        &BootstrapEnvName::from(DEFAULT_BOOTSTRAP_ENV)
    );
    assert_eq!(
        spec.control_bootstrap_env_ref(),
        &BootstrapEnvName::from(CONTROL_BOOTSTRAP_ENV)
    );
    assert!(!spec.captures_stdout());
    assert!(!spec.captures_stderr());
    assert!(spec.stdout_log_ref().is_none());
    assert!(spec.stderr_log_ref().is_none());
    assert!(spec.dependencies_ref().is_empty());
}

#[test]
fn process_spec_output_capture_helpers_toggle_both_streams() {
    let captured = ProcessSpec::builder("child-binary")
        .capture_output()
        .build();
    assert!(captured.captures_stdout());
    assert!(captured.captures_stderr());

    let uncaptured = ProcessSpec::builder("child-binary")
        .inherit_output()
        .build();
    assert!(!uncaptured.captures_stdout());
    assert!(!uncaptured.captures_stderr());
}

#[test]
fn process_spec_log_output_uses_one_append_target_for_both_streams() {
    let logged = ProcessSpec::builder("child-binary")
        .capture_output()
        .log_output("/tmp/pork-child.log")
        .build();

    assert!(!logged.captures_stdout());
    assert!(!logged.captures_stderr());
    assert_eq!(
        logged.stdout_log_ref(),
        Some(std::path::Path::new("/tmp/pork-child.log"))
    );
    assert_eq!(
        logged.stderr_log_ref(),
        Some(std::path::Path::new("/tmp/pork-child.log"))
    );

    let inherited = ProcessSpec::builder("child-binary")
        .inherit_output()
        .build();
    assert!(inherited.stdout_log_ref().is_none());
    assert!(inherited.stderr_log_ref().is_none());
}

#[test]
fn process_spec_without_managed_name_clears_previous_value() {
    let spec = ProcessSpec::builder("child-binary")
        .managed_name("temporary-name")
        .without_managed_name()
        .build();

    assert_eq!(spec.managed_name().map(ManagedChildName::as_str), None);
}
