use std::env;
use std::time::Duration;

use pork::DEFAULT_BOOTSTRAP_ENV;
use pork::child::bootstrap::{child_connect_from_env, child_control_codec_from_env};
use pork::error::OrchestratorError;
use pork::orchestrator::{ManagedChild, ProcessOrchestrator};
use pork::spec::ProcessSpec;
use pork_proto::protocol::{PorkChildStatus, PorkControlCodec};

fn current_exe_spec() -> ProcessSpec {
    let executable = env::current_exe().expect("current test executable path should be available");

    ProcessSpec::new(executable)
        .arg("--exact")
        .arg("restart_test_child_entrypoint")
        .arg("--nocapture")
        .managed_name("restart-test-child")
        .control_codec(PorkControlCodec::Json)
}

async fn recv_utf8(child: &ManagedChild) -> String {
    let message = child.recv().await.expect("child should send a message");
    String::from_utf8(message).expect("child message should be valid utf-8")
}

#[tokio::test]
async fn orchestrator_reports_running_status_for_started_child() {
    let orchestrator = ProcessOrchestrator::builder()
        .graceful_shutdown_timeout(Duration::from_secs(2))
        .build();

    let child = orchestrator
        .start_process(current_exe_spec())
        .await
        .expect("child should start");

    let _ = recv_utf8(&child).await;

    let status_by_id = orchestrator
        .process_status(child.id())
        .await
        .expect("status lookup by id should succeed");
    assert_eq!(status_by_id, PorkChildStatus::Running);

    let status_by_name = orchestrator
        .process_status_by_name("restart-test-child")
        .await
        .expect("status lookup by name should succeed");
    assert_eq!(status_by_name, PorkChildStatus::Running);

    let shutdown_status = orchestrator
        .graceful_shutdown_process(child.id())
        .await
        .expect("shutdown should succeed");
    assert!(
        shutdown_status.success() || shutdown_status.code().is_none(),
        "shutdown should either exit cleanly or terminate by signal: {shutdown_status:?}"
    );
}

#[tokio::test]
async fn orchestrator_reports_stopping_status_after_graceful_shutdown_request() {
    let orchestrator = ProcessOrchestrator::builder()
        .graceful_shutdown_timeout(Duration::from_secs(2))
        .build();

    let child = orchestrator
        .start_process(current_exe_spec())
        .await
        .expect("child should start");

    let _ = recv_utf8(&child).await;

    orchestrator
        .request_graceful_shutdown(child.id())
        .await
        .expect("graceful shutdown request should succeed");

    let status = orchestrator
        .process_status(child.id())
        .await
        .expect("status lookup should succeed while child is still tracked");
    assert_eq!(status, PorkChildStatus::Stopping);

    let shutdown_status = orchestrator
        .graceful_shutdown_process_with_timeout(child.id(), Duration::from_millis(10))
        .await
        .expect("child should exit after graceful shutdown request");
    assert!(
        shutdown_status.success() || shutdown_status.code().is_none(),
        "shutdown should either exit cleanly or terminate by signal: {shutdown_status:?}"
    );

    assert!(matches!(
        orchestrator.process_status(child.id()).await,
        Err(OrchestratorError::ProcessNotFound(id)) if id == child.id()
    ));
}

#[tokio::test]
async fn orchestrator_restart_process_replaces_child_and_preserves_managed_name_via_process_id() {
    let orchestrator = ProcessOrchestrator::builder()
        .graceful_shutdown_timeout(Duration::from_secs(2))
        .build();

    let child = orchestrator
        .start_process(current_exe_spec())
        .await
        .expect("child should start");

    let first_announcement = recv_utf8(&child).await;
    assert!(first_announcement.starts_with("started:"));

    let first_id = child.id();
    let named_id_before = orchestrator
        .process_id_by_name("restart-test-child")
        .await
        .expect("lookup should succeed")
        .expect("managed process should exist");
    assert_eq!(named_id_before, first_id);

    let restarted = orchestrator
        .restart_process(child.id())
        .await
        .expect("restart should succeed");
    let second_announcement = recv_utf8(&restarted).await;
    assert!(second_announcement.starts_with("started:"));

    let second_id = restarted.id();
    assert_ne!(second_id, first_id);

    let named_id_after = orchestrator
        .process_id_by_name("restart-test-child")
        .await
        .expect("lookup should succeed")
        .expect("managed process should exist after restart");
    assert_eq!(named_id_after, second_id);

    let active_process_ids = orchestrator
        .process_ids()
        .await
        .expect("process listing should succeed");
    assert_eq!(active_process_ids, vec![second_id]);

    let shutdown_status = orchestrator
        .graceful_shutdown_process(restarted.id())
        .await
        .expect("shutdown should succeed");
    assert!(
        shutdown_status.success() || shutdown_status.code().is_none(),
        "shutdown should either exit cleanly or terminate by signal: {shutdown_status:?}"
    );

    assert!(
        orchestrator
            .process_id_by_name("restart-test-child")
            .await
            .expect("lookup should succeed")
            .is_none()
    );
    assert!(
        orchestrator
            .process_ids()
            .await
            .expect("process listing should succeed after shutdown")
            .is_empty()
    );
}

#[tokio::test]
async fn orchestrator_restart_process_after_name_lookup_restarts_named_child() {
    let orchestrator = ProcessOrchestrator::builder()
        .graceful_shutdown_timeout(Duration::from_secs(2))
        .build();

    let child = orchestrator
        .start_process(current_exe_spec())
        .await
        .expect("child should start");

    let _ = recv_utf8(&child).await;

    let original_id = child.id();
    let process_id = orchestrator
        .process_id_by_name("restart-test-child")
        .await
        .expect("lookup should succeed")
        .expect("managed process should exist");
    let restarted = orchestrator
        .restart_process(process_id)
        .await
        .expect("restart should succeed");

    let restarted_id = restarted.id();
    assert_ne!(restarted_id, original_id);
    assert_eq!(restarted.name(), Some("restart-test-child"));

    let restart_message = recv_utf8(&restarted).await;
    assert!(restart_message.starts_with("started:"));

    let shutdown_status = orchestrator
        .graceful_shutdown_process(restarted.id())
        .await
        .expect("shutdown should succeed");
    assert!(
        shutdown_status.success() || shutdown_status.code().is_none(),
        "shutdown should either exit cleanly or terminate by signal: {shutdown_status:?}"
    );

    assert!(
        orchestrator
            .process_id_by_name("restart-test-child")
            .await
            .expect("lookup should succeed after shutdown")
            .is_none()
    );
    assert!(
        orchestrator
            .process_ids()
            .await
            .expect("process listing should succeed after shutdown")
            .is_empty()
    );
}

#[tokio::test]
async fn process_id_by_name_returns_not_found_shape_for_unknown_name() {
    let orchestrator = ProcessOrchestrator::new();

    let error = orchestrator
        .process_id_by_name("missing-process")
        .await
        .expect("lookup should succeed")
        .ok_or_else(|| OrchestratorError::ProcessNameNotFound("missing-process".to_owned()))
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
    let runtime = tokio::runtime::Runtime::new().expect("tokio runtime should be created");
    let (receiver, sender) = runtime
        .block_on(child_connect_from_env(DEFAULT_BOOTSTRAP_ENV))
        .expect("child IPC bootstrap should succeed");

    let pid = std::process::id();
    sender
        .send(format!("started:{pid}").into_bytes())
        .expect("child should announce startup");

    loop {
        let message = {
            let receiver = receiver.blocking_lock();
            receiver.recv().expect("child should receive IPC messages")
        };
        if codec.is_graceful_shutdown_message(&message) {
            break;
        }
    }
}
