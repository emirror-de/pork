#![cfg(all(feature = "client", feature = "host"))]

use std::env;
use std::io::Write;
use std::path::Path;
use std::time::Duration;

use pork::DEFAULT_BOOTSTRAP_ENV;
use pork::child::bootstrap::ChildBootstrap;
use pork::error::OrchestratorError;
use pork::orchestrator::ProcessOrchestrator;
use pork::orchestrator::spec::ProcessSpec;
use pork::types::ManagedChild;
use pork::types::ManagedChildName;
use pork_proto::protocol::{PorkChildStatus, PorkControlCodec, PorkControlMessage};

const CHILD_MODE_ENV: &str = "PORK_RESTART_TEST_MODE";
const RESTART_TEST_CHILD_NAME: &str = "restart-test-child";
const STUBBORN_TEST_CHILD_NAME: &str = "stubborn-test-child";
const LOGGED_TEST_CHILD_NAME: &str = "logged-test-child";

fn current_exe_spec() -> ProcessSpec {
    let executable = match env::current_exe() {
        Ok(path) => path,
        Err(error) => panic!("current test executable path should be available: {error}"),
    };

    ProcessSpec::builder(executable)
        .arg("--exact")
        .arg("restart_test_child_entrypoint")
        .arg("--nocapture")
        .env(CHILD_MODE_ENV, "cooperative")
        .managed_name(RESTART_TEST_CHILD_NAME)
        .control_codec(PorkControlCodec::Json)
        .build()
}

fn stubborn_exe_spec() -> ProcessSpec {
    let executable = match env::current_exe() {
        Ok(path) => path,
        Err(error) => panic!("current test executable path should be available: {error}"),
    };

    ProcessSpec::builder(executable)
        .arg("--exact")
        .arg("restart_test_child_entrypoint")
        .arg("--nocapture")
        .env(CHILD_MODE_ENV, "stubborn")
        .managed_name(STUBBORN_TEST_CHILD_NAME)
        .control_codec(PorkControlCodec::Json)
        .build()
}

fn logged_exe_spec(path: &Path) -> ProcessSpec {
    let executable = match env::current_exe() {
        Ok(path) => path,
        Err(error) => panic!("current test executable path should be available: {error}"),
    };

    ProcessSpec::builder(executable)
        .arg("--exact")
        .arg("restart_test_child_entrypoint")
        .arg("--nocapture")
        .env(CHILD_MODE_ENV, "logged")
        .managed_name(LOGGED_TEST_CHILD_NAME)
        .control_codec(PorkControlCodec::Json)
        .log_output(path)
        .build()
}

async fn recv_utf8(child: &ManagedChild) -> String {
    let message = match child.recv().await {
        Some(message) => message,
        None => panic!("child should send a message"),
    };

    match String::from_utf8(message.into()) {
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

    let status_by_id = match orchestrator.process_status(child.process_id()).await {
        Ok(status) => status,
        Err(error) => panic!("status lookup by id should succeed: {error}"),
    };
    assert_eq!(status_by_id, PorkChildStatus::Running);

    let status_by_name = match orchestrator
        .process_status_by_name(&ManagedChildName::from(RESTART_TEST_CHILD_NAME))
        .await
    {
        Ok(status) => status,
        Err(error) => panic!("status lookup by name should succeed: {error}"),
    };
    assert_eq!(status_by_name, PorkChildStatus::Running);

    let shutdown_status = match orchestrator
        .graceful_shutdown_process(child.process_id())
        .await
    {
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

    if let Err(error) = orchestrator
        .request_graceful_shutdown(child.process_id())
        .await
    {
        panic!("graceful shutdown request should succeed: {error}");
    }

    let status = match orchestrator.process_status(child.process_id()).await {
        Ok(status) => status,
        Err(error) => panic!("status lookup should succeed while child is still tracked: {error}"),
    };
    assert_eq!(status, PorkChildStatus::Stopping);

    let shutdown_status = match orchestrator
        .graceful_shutdown_process_with_timeout(child.process_id(), Duration::from_millis(10))
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
        orchestrator.process_status(child.process_id()).await,
        Err(OrchestratorError::ProcessNotFound(id)) if id == child.process_id()
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

    let first_id = child.process_id();
    let named_id_before = match orchestrator
        .process_id_by_name(&ManagedChildName::from(RESTART_TEST_CHILD_NAME))
        .await
    {
        Ok(Some(process_id)) => process_id,
        Ok(None) => panic!("managed process should exist"),
        Err(error) => panic!("lookup should succeed: {error}"),
    };
    assert_eq!(named_id_before, first_id);

    let restarted = match orchestrator.restart_process(child.process_id()).await {
        Ok(child) => child,
        Err(error) => panic!("restart should succeed: {error}"),
    };
    let restart_command = recv_utf8(&child).await;
    assert_eq!(restart_command, "restart");
    let second_announcement = recv_utf8(&restarted).await;
    assert!(second_announcement.starts_with("started:"));

    let second_id = restarted.process_id();
    assert_ne!(second_id, first_id);

    let named_id_after = match orchestrator
        .process_id_by_name(&ManagedChildName::from(RESTART_TEST_CHILD_NAME))
        .await
    {
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

    let shutdown_status = match orchestrator
        .graceful_shutdown_process(restarted.process_id())
        .await
    {
        Ok(status) => status,
        Err(error) => panic!("shutdown should succeed: {error}"),
    };
    assert!(
        shutdown_status.success() || shutdown_status.code().is_none(),
        "shutdown should either exit cleanly or terminate by signal: {shutdown_status:?}"
    );

    assert!(match orchestrator
        .process_id_by_name(&ManagedChildName::from(RESTART_TEST_CHILD_NAME))
        .await
    {
        Ok(process_id) => process_id.is_none(),
        Err(error) => panic!("lookup should succeed: {error}"),
    });
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

    let original_id = child.process_id();
    let process_id = match orchestrator
        .process_id_by_name(&ManagedChildName::from(RESTART_TEST_CHILD_NAME))
        .await
    {
        Ok(Some(process_id)) => process_id,
        Ok(None) => panic!("managed process should exist"),
        Err(error) => panic!("lookup should succeed: {error}"),
    };
    let restarted = match orchestrator.restart_process(process_id).await {
        Ok(child) => child,
        Err(error) => panic!("restart should succeed: {error}"),
    };

    let restarted_id = restarted.process_id();
    assert_ne!(restarted_id, original_id);
    assert_eq!(
        restarted.managed_name().map(ManagedChildName::as_str),
        Some(RESTART_TEST_CHILD_NAME)
    );

    let restart_message = recv_utf8(&restarted).await;
    assert!(restart_message.starts_with("started:"));

    let shutdown_status = match orchestrator
        .graceful_shutdown_process(restarted.process_id())
        .await
    {
        Ok(status) => status,
        Err(error) => panic!("shutdown should succeed: {error}"),
    };
    assert!(
        shutdown_status.success() || shutdown_status.code().is_none(),
        "shutdown should either exit cleanly or terminate by signal: {shutdown_status:?}"
    );

    assert!(match orchestrator
        .process_id_by_name(&ManagedChildName::from(RESTART_TEST_CHILD_NAME))
        .await
    {
        Ok(process_id) => process_id.is_none(),
        Err(error) => panic!("lookup should succeed after shutdown: {error}"),
    });
    assert!(match orchestrator.process_ids().await {
        Ok(process_ids) => process_ids.is_empty(),
        Err(error) => panic!("process listing should succeed after shutdown: {error}"),
    });
}

#[tokio::test]
async fn process_id_by_name_returns_not_found_shape_for_unknown_name() {
    let orchestrator = ProcessOrchestrator::new();

    let lookup_result = match orchestrator
        .process_id_by_name(&ManagedChildName::from("missing-process"))
        .await
    {
        Ok(result) => result,
        Err(error) => panic!("lookup should succeed: {error}"),
    };

    let error = match lookup_result {
        Some(process_id) => {
            panic!("restart should fail for an unknown managed name, got {process_id}")
        }
        None => OrchestratorError::ProcessNameNotFound(ManagedChildName::from("missing-process")),
    };

    match error {
        OrchestratorError::ProcessNameNotFound(name) => {
            assert_eq!(name.as_str(), "missing-process");
        }
        other => panic!("unexpected error: {other}"),
    }
}

#[tokio::test]
async fn logfile_output_is_inspectable_while_child_is_running() {
    let log_path =
        std::env::temp_dir().join(format!("pork-live-output-{}.log", std::process::id()));
    let _ = std::fs::remove_file(&log_path);

    let orchestrator = ProcessOrchestrator::new();
    let child = match orchestrator.start_process(logged_exe_spec(&log_path)).await {
        Ok(child) => child,
        Err(error) => panic!("logged child should start: {error}"),
    };
    let _ = recv_utf8(&child).await;

    let output = match tokio::fs::read_to_string(&log_path).await {
        Ok(output) => output,
        Err(error) => panic!("logfile should be readable while child runs: {error}"),
    };
    assert!(output.contains("pork-live-stdout"));
    assert!(output.contains("pork-live-stderr"));
    assert_eq!(
        orchestrator.process_status(child.process_id()).await.ok(),
        Some(PorkChildStatus::Running)
    );

    if let Err(error) = orchestrator
        .graceful_shutdown_process(child.process_id())
        .await
    {
        panic!("logged child should shut down: {error}");
    }
    let _ = std::fs::remove_file(log_path);
}

#[tokio::test]
async fn graceful_shutdown_timeout_force_kills_and_cleans_up_stubborn_child() {
    let orchestrator = ProcessOrchestrator::new();
    let child = match orchestrator.start_process(stubborn_exe_spec()).await {
        Ok(child) => child,
        Err(error) => panic!("stubborn child should start: {error}"),
    };
    let _ = recv_utf8(&child).await;

    let started = tokio::time::Instant::now();
    let status = orchestrator
        .graceful_shutdown_process_with_timeout(child.process_id(), Duration::from_millis(50))
        .await;

    assert!(
        status.is_ok(),
        "timeout fallback should reap the child: {status:?}"
    );
    assert!(
        started.elapsed() < Duration::from_secs(1),
        "timeout fallback should not hang"
    );
    assert!(matches!(
        orchestrator.process_status(child.process_id()).await,
        Err(OrchestratorError::ProcessNotFound(id)) if id == child.process_id()
    ));
}

#[test]
fn restart_test_child_entrypoint() {
    if env::var(DEFAULT_BOOTSTRAP_ENV).is_err() {
        return;
    }

    let runtime = match tokio::runtime::Runtime::new() {
        Ok(runtime) => runtime,
        Err(error) => panic!("tokio runtime should be created: {error}"),
    };
    let channels =
        match runtime.block_on(async { ChildBootstrap::from_default_env()?.connect().await }) {
            Ok(channels) => channels,
            Err(error) => panic!("child IPC bootstrap should succeed: {error}"),
        };

    runtime.block_on(async {
        let mode = env::var(CHILD_MODE_ENV).unwrap_or_default();
        if mode == "logged" {
            println!("pork-live-stdout");
            eprintln!("pork-live-stderr");
            if let Err(error) = std::io::stdout().flush() {
                panic!("child stdout should flush: {error}");
            }
            if let Err(error) = std::io::stderr().flush() {
                panic!("child stderr should flush: {error}");
            }
        }

        let pid = std::process::id();
        if let Err(error) = channels.send_data(format!("started:{pid}").into_bytes()) {
            panic!("child should announce startup: {error}");
        }

        if mode == "stubborn" {
            std::future::pending::<()>().await;
            return;
        }

        loop {
            tokio::select! {
                control = channels.recv_control() => match control {
                    Ok(Some(PorkControlMessage::GracefulShutdown)) => {
                        if let Err(error) = channels.send_data(b"shutdown".to_vec()) {
                            panic!("child should acknowledge shutdown: {error}");
                        }
                        break;
                    }
                    Ok(Some(PorkControlMessage::Restart)) => {
                        if let Err(error) = channels.send_data(b"restart".to_vec()) {
                            panic!("child should acknowledge restart: {error}");
                        }
                        break;
                    }
                    Ok(Some(PorkControlMessage::StatusUpdate(_))) => {}
                    Ok(None) => break,
                    Err(error) => panic!("child should decode control messages: {error}"),
                },
                message = channels.recv_data() => {
                    if message.is_none() {
                        break;
                    }
                }
            }
        }
    });
}
