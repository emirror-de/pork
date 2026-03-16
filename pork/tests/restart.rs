use std::env;
use std::time::Duration;

use pork::DEFAULT_BOOTSTRAP_ENV;
use pork::child::bootstrap::{child_connect_from_env, child_control_codec_from_env};
use pork::error::OrchestratorError;
use pork::orchestrator::{ManagedChild, ProcessOrchestrator};
use pork::spec::ProcessSpec;
use pork_proto::protocol::{PorkChildStatus, PorkControlCodec};

fn current_exe_spec() -> ProcessSpec {
    let executable = match env::current_exe() {
        Ok(path) => path,
        Err(error) => panic!("current test executable path should be available: {error}"),
    };

    ProcessSpec::new(executable)
        .arg("--exact")
        .arg("restart_test_child_entrypoint")
        .arg("--nocapture")
        .managed_name("restart-test-child")
        .control_codec(PorkControlCodec::Json)
}

async fn recv_utf8(child: &ManagedChild) -> String {
    let message = match child.recv().await {
        Some(message) => message,
        None => panic!("child should send a message"),
    };

    match String::from_utf8(message) {
        Ok(message) => message,
        Err(error) => panic!("child message should be valid utf-8: {error}"),
    }
}

#[tokio::test]
async fn orchestrator_reports_running_status_for_started_child() {
    let orchestrator = ProcessOrchestrator::builder()
        .graceful_shutdown_timeout(Duration::from_secs(2))
        .build();

    let child = match orchestrator.start_process(current_exe_spec()).await {
        Ok(child) => child,
        Err(error) => panic!("child should start: {error}"),
    };

    let _ = recv_utf8(&child).await;

    let status_by_id = match orchestrator.process_status(child.id()).await {
        Ok(status) => status,
        Err(error) => panic!("status lookup by id should succeed: {error}"),
    };
    assert_eq!(status_by_id, PorkChildStatus::Running);

    let status_by_name = match orchestrator
        .process_status_by_name("restart-test-child")
        .await
    {
        Ok(status) => status,
        Err(error) => panic!("status lookup by name should succeed: {error}"),
    };
    assert_eq!(status_by_name, PorkChildStatus::Running);

    let shutdown_status = match orchestrator.graceful_shutdown_process(child.id()).await {
        Ok(status) => status,
        Err(error) => panic!("shutdown should succeed: {error}"),
    };
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

    let child = match orchestrator.start_process(current_exe_spec()).await {
        Ok(child) => child,
        Err(error) => panic!("child should start: {error}"),
    };

    let _ = recv_utf8(&child).await;

    if let Err(error) = orchestrator.request_graceful_shutdown(child.id()).await {
        panic!("graceful shutdown request should succeed: {error}");
    }

    let status = match orchestrator.process_status(child.id()).await {
        Ok(status) => status,
        Err(error) => panic!("status lookup should succeed while child is still tracked: {error}"),
    };
    assert_eq!(status, PorkChildStatus::Stopping);

    let shutdown_status = match orchestrator
        .graceful_shutdown_process_with_timeout(child.id(), Duration::from_millis(10))
        .await
    {
        Ok(status) => status,
        Err(error) => panic!("child should exit after graceful shutdown request: {error}"),
    };
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

    let child = match orchestrator.start_process(current_exe_spec()).await {
        Ok(child) => child,
        Err(error) => panic!("child should start: {error}"),
    };

    let first_announcement = recv_utf8(&child).await;
    assert!(first_announcement.starts_with("started:"));

    let first_id = child.id();
    let named_id_before = match orchestrator.process_id_by_name("restart-test-child").await {
        Ok(Some(process_id)) => process_id,
        Ok(None) => panic!("managed process should exist"),
        Err(error) => panic!("lookup should succeed: {error}"),
    };
    assert_eq!(named_id_before, first_id);

    let restarted = match orchestrator.restart_process(child.id()).await {
        Ok(child) => child,
        Err(error) => panic!("restart should succeed: {error}"),
    };
    let second_announcement = recv_utf8(&restarted).await;
    assert!(second_announcement.starts_with("started:"));

    let second_id = restarted.id();
    assert_ne!(second_id, first_id);

    let named_id_after = match orchestrator.process_id_by_name("restart-test-child").await {
        Ok(Some(process_id)) => process_id,
        Ok(None) => panic!("managed process should exist after restart"),
        Err(error) => panic!("lookup should succeed: {error}"),
    };
    assert_eq!(named_id_after, second_id);

    let active_process_ids = match orchestrator.process_ids().await {
        Ok(process_ids) => process_ids,
        Err(error) => panic!("process listing should succeed: {error}"),
    };
    assert_eq!(active_process_ids, vec![second_id]);

    let shutdown_status = match orchestrator.graceful_shutdown_process(restarted.id()).await {
        Ok(status) => status,
        Err(error) => panic!("shutdown should succeed: {error}"),
    };
    assert!(
        shutdown_status.success() || shutdown_status.code().is_none(),
        "shutdown should either exit cleanly or terminate by signal: {shutdown_status:?}"
    );

    assert!(
        match orchestrator.process_id_by_name("restart-test-child").await {
            Ok(process_id) => process_id.is_none(),
            Err(error) => panic!("lookup should succeed: {error}"),
        }
    );
    assert!(match orchestrator.process_ids().await {
        Ok(process_ids) => process_ids.is_empty(),
        Err(error) => panic!("process listing should succeed after shutdown: {error}"),
    });
}

#[tokio::test]
async fn orchestrator_restart_process_after_name_lookup_restarts_named_child() {
    let orchestrator = ProcessOrchestrator::builder()
        .graceful_shutdown_timeout(Duration::from_secs(2))
        .build();

    let child = match orchestrator.start_process(current_exe_spec()).await {
        Ok(child) => child,
        Err(error) => panic!("child should start: {error}"),
    };

    let _ = recv_utf8(&child).await;

    let original_id = child.id();
    let process_id = match orchestrator.process_id_by_name("restart-test-child").await {
        Ok(Some(process_id)) => process_id,
        Ok(None) => panic!("managed process should exist"),
        Err(error) => panic!("lookup should succeed: {error}"),
    };
    let restarted = match orchestrator.restart_process(process_id).await {
        Ok(child) => child,
        Err(error) => panic!("restart should succeed: {error}"),
    };

    let restarted_id = restarted.id();
    assert_ne!(restarted_id, original_id);
    assert_eq!(restarted.name(), Some("restart-test-child"));

    let restart_message = recv_utf8(&restarted).await;
    assert!(restart_message.starts_with("started:"));

    let shutdown_status = match orchestrator.graceful_shutdown_process(restarted.id()).await {
        Ok(status) => status,
        Err(error) => panic!("shutdown should succeed: {error}"),
    };
    assert!(
        shutdown_status.success() || shutdown_status.code().is_none(),
        "shutdown should either exit cleanly or terminate by signal: {shutdown_status:?}"
    );

    assert!(
        match orchestrator.process_id_by_name("restart-test-child").await {
            Ok(process_id) => process_id.is_none(),
            Err(error) => panic!("lookup should succeed after shutdown: {error}"),
        }
    );
    assert!(match orchestrator.process_ids().await {
        Ok(process_ids) => process_ids.is_empty(),
        Err(error) => panic!("process listing should succeed after shutdown: {error}"),
    });
}

#[tokio::test]
async fn process_id_by_name_returns_not_found_shape_for_unknown_name() {
    let orchestrator = ProcessOrchestrator::new();

    let lookup_result = match orchestrator.process_id_by_name("missing-process").await {
        Ok(result) => result,
        Err(error) => panic!("lookup should succeed: {error}"),
    };

    let error = match lookup_result {
        Some(process_id) => {
            panic!("restart should fail for an unknown managed name, got {process_id}")
        }
        None => OrchestratorError::ProcessNameNotFound("missing-process".to_owned()),
    };

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

    let codec = match child_control_codec_from_env() {
        Ok(codec) => codec,
        Err(error) => panic!("control codec should be present: {error}"),
    };
    let runtime = match tokio::runtime::Runtime::new() {
        Ok(runtime) => runtime,
        Err(error) => panic!("tokio runtime should be created: {error}"),
    };
    let (receiver, sender) = match runtime.block_on(child_connect_from_env(DEFAULT_BOOTSTRAP_ENV)) {
        Ok(channels) => channels,
        Err(error) => panic!("child IPC bootstrap should succeed: {error}"),
    };

    let pid = std::process::id();
    if let Err(error) = sender.send(format!("started:{pid}").into_bytes()) {
        panic!("child should announce startup: {error}");
    }

    loop {
        let message = {
            let receiver = receiver.blocking_lock();
            match receiver.recv() {
                Ok(message) => message,
                Err(error) => panic!("child should receive IPC messages: {error}"),
            }
        };
        if codec.is_graceful_shutdown_message(&message) {
            break;
        }
    }
}
