use std::env;
use std::time::Duration;

use pork::{
    DEFAULT_BOOTSTRAP_ENV, ManagedChild, OrchestratorError, PorkControlCodec, ProcessOrchestrator,
    ProcessSpec, child_connect_from_env, child_control_codec_from_env,
    is_graceful_shutdown_message,
};

fn current_exe_spec() -> ProcessSpec {
    let executable = env::current_exe().expect("current test executable path should be available");

    ProcessSpec::new(executable)
        .arg("--exact")
        .arg("restart_test_child_entrypoint")
        .arg("--nocapture")
        .managed_name("restart-test-child")
        .control_codec(PorkControlCodec::Json)
}

fn recv_utf8(child: &ManagedChild) -> String {
    let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should be created");
    let message = runtime
        .block_on(child.recv())
        .expect("child should send a message");
    String::from_utf8(message).expect("child message should be valid utf-8")
}

#[test]
fn orchestrator_restart_process_replaces_child_and_preserves_managed_name() {
    let orchestrator = ProcessOrchestrator::with_graceful_shutdown_timeout(Duration::from_secs(2));

    let child = orchestrator
        .start_process(current_exe_spec())
        .expect("child should start");

    let first_announcement = recv_utf8(&child);
    assert!(first_announcement.starts_with("started:"));

    let first_id = child.id();
    let named_id_before = orchestrator
        .process_id_by_name("restart-test-child")
        .expect("lookup should succeed")
        .expect("managed process should exist");
    assert_eq!(named_id_before, first_id);

    let restarted = child.restart().expect("restart should succeed");
    let second_announcement = recv_utf8(&restarted);
    assert!(second_announcement.starts_with("started:"));

    let second_id = restarted.id();
    assert_ne!(second_id, first_id);

    let named_id_after = orchestrator
        .process_id_by_name("restart-test-child")
        .expect("lookup should succeed")
        .expect("managed process should exist after restart");
    assert_eq!(named_id_after, second_id);

    let active_process_ids = orchestrator
        .process_ids()
        .expect("process listing should succeed");
    assert_eq!(active_process_ids, vec![second_id]);

    let shutdown_status = restarted.shutdown().expect("shutdown should succeed");
    assert!(
        shutdown_status.success() || shutdown_status.code().is_none(),
        "shutdown should either exit cleanly or terminate by signal: {shutdown_status:?}"
    );

    assert!(
        orchestrator
            .process_id_by_name("restart-test-child")
            .expect("lookup should succeed")
            .is_none()
    );
    assert!(
        orchestrator
            .process_ids()
            .expect("process listing should succeed after shutdown")
            .is_empty()
    );
}

#[test]
fn orchestrator_restart_process_by_name_restarts_named_child() {
    let orchestrator = ProcessOrchestrator::with_graceful_shutdown_timeout(Duration::from_secs(2));

    let child = orchestrator
        .start_process(current_exe_spec())
        .expect("child should start");

    let _ = recv_utf8(&child);

    let original_id = child.id();
    let restarted = orchestrator
        .restart_process_by_name("restart-test-child")
        .expect("named restart should succeed");

    let restarted_id = restarted.id();
    assert_ne!(restarted_id, original_id);
    assert_eq!(restarted.name(), Some("restart-test-child"));

    let restart_message = recv_utf8(&restarted);
    assert!(restart_message.starts_with("started:"));

    let shutdown_status = restarted.shutdown().expect("shutdown should succeed");
    assert!(
        shutdown_status.success() || shutdown_status.code().is_none(),
        "shutdown should either exit cleanly or terminate by signal: {shutdown_status:?}"
    );

    assert!(
        orchestrator
            .process_id_by_name("restart-test-child")
            .expect("lookup should succeed after shutdown")
            .is_none()
    );
    assert!(
        orchestrator
            .process_ids()
            .expect("process listing should succeed after shutdown")
            .is_empty()
    );
}

#[test]
fn orchestrator_restart_process_by_name_returns_not_found_for_unknown_name() {
    let orchestrator = ProcessOrchestrator::new();

    let error = orchestrator
        .restart_process_by_name("missing-process")
        .expect_err("restart should fail for an unknown managed name");

    match error {
        OrchestratorError::ProcessNameNotFound(name) => {
            assert_eq!(name, "missing-process");
        }
        other => panic!("unexpected error: {other}"),
    }
}

#[test]
fn restart_test_child_entrypoint() {
    if env::var(DEFAULT_BOOTSTRAP_ENV).is_err() {
        return;
    }

    let codec = child_control_codec_from_env().expect("control codec should be present");
    let (receiver, sender) =
        child_connect_from_env(DEFAULT_BOOTSTRAP_ENV).expect("child IPC bootstrap should succeed");

    let pid = std::process::id();
    sender
        .send(format!("started:{pid}").into_bytes())
        .expect("child should announce startup");

    loop {
        let message = receiver.recv().expect("child should receive IPC messages");
        if is_graceful_shutdown_message(&message, codec) {
            break;
        }
    }
}
