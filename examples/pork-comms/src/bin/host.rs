use std::path::PathBuf;
use std::time::Duration;

use pork::error::{OrchestratorError, Result};
use pork::orchestrator::ProcessOrchestrator;
use pork::orchestrator::spec::ProcessSpec;
use pork::types::ManagedChild;
use pork::types::ManagedChildName;
use pork_comms::{ChildMessage, HostMessage, decode_message, encode_message};
use pork_proto::protocol::PorkControlCodec;

const CHILD_MANAGED_NAME: &str = "example-child";
const HEARTBEAT_OBSERVATION_WINDOW: Duration = Duration::from_secs(5);
const HEARTBEAT_POLL_INTERVAL: Duration = Duration::from_millis(200);

fn child_managed_name() -> ManagedChildName {
    ManagedChildName::from(CHILD_MANAGED_NAME)
}

#[tokio::main]
async fn main() -> Result<()> {
    let orchestrator = ProcessOrchestrator::new();
    let child_binary = child_binary_path()?;

    println!("host: starting child at {}", child_binary.display());

    let control_codec = PorkControlCodec::Json;
    let child = orchestrator
        .start_process(
            ProcessSpec::builder(child_binary)
                .managed_name(CHILD_MANAGED_NAME)
                .control_codec(control_codec)
                .enable_heartbeat(Duration::from_secs(1))
                .build(),
        )
        .await?;

    let identity = child.identity();
    println!(
        "host: started child with id={} name={}",
        identity.process_id(),
        identity
            .managed_name()
            .map(ManagedChildName::as_str)
            .unwrap_or("<unnamed>")
    );

    let child_name = child_managed_name();
    let lookup_id = orchestrator
        .process_id_by_name(&child_name)
        .await?
        .ok_or_else(|| OrchestratorError::ProcessNameNotFound(child_name.clone()))?;
    println!("host: lookup by name resolved to process id={lookup_id}");

    let registered_names = orchestrator.process_names().await?;
    println!("host: registered managed names: {registered_names:?}");

    let ready = recv_child_message(&child, control_codec).await?;
    println!("child -> host: {ready:?}");

    send_host_message(&child, control_codec, HostMessage::Status)?;
    let status = recv_child_message(&child, control_codec).await?;
    println!("child -> host: {status:?}");

    let last_heartbeat_timestamp_ms =
        if let Some(child_status) = orchestrator.child_status_by_name(&child_name).await? {
            println!(
                "host: latest child-reported status = {:?} at {} ms",
                child_status.status, child_status.timestamp_ms
            );
            Some(child_status.timestamp_ms)
        } else {
            None
        };

    println!(
        "host: keeping child alive for {} seconds to observe heartbeats",
        HEARTBEAT_OBSERVATION_WINDOW.as_secs()
    );
    print_heartbeats(
        &orchestrator,
        &child_name,
        HEARTBEAT_OBSERVATION_WINDOW,
        last_heartbeat_timestamp_ms,
    )
    .await?;

    for message in [
        "hello from host",
        "pork can exchange typed payloads",
        "this child echoes messages back",
    ] {
        let host_message = HostMessage::Echo(message.to_owned());
        println!("host -> child: {host_message:?}");
        send_host_message(&child, control_codec, host_message)?;

        let response = recv_child_message(&child, control_codec).await?;
        println!("child -> host: {response:?}");
    }

    println!("host: requesting graceful shutdown");
    let exit_status = orchestrator
        .graceful_shutdown_process(child.process_id())
        .await?;
    println!("host: child exited with status {exit_status}");

    Ok(())
}

async fn recv_child_message(child: &ManagedChild, codec: PorkControlCodec) -> Result<ChildMessage> {
    let response = child.recv().await.ok_or_else(|| {
        OrchestratorError::Io(std::io::Error::new(
            std::io::ErrorKind::UnexpectedEof,
            "child closed its IPC channel before replying",
        ))
    })?;

    decode_message::<ChildMessage>(codec, response.as_ref())
        .map_err(|error| {
            OrchestratorError::Io(std::io::Error::new(std::io::ErrorKind::InvalidData, error))
        })?
        .ok_or_else(|| {
            OrchestratorError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "received a control envelope where a custom message was expected",
            ))
        })
}

fn send_host_message(
    child: &ManagedChild,
    codec: PorkControlCodec,
    message: HostMessage,
) -> Result<()> {
    let payload = encode_message(codec, message)
        .map_err(|error| OrchestratorError::Io(std::io::Error::other(error)))?;
    child.send(payload)
}

async fn print_heartbeats(
    orchestrator: &ProcessOrchestrator,
    child_name: &ManagedChildName,
    keep_alive_for: Duration,
    mut last_timestamp_ms: Option<u64>,
) -> Result<()> {
    let started_at = tokio::time::Instant::now();
    let mut observed_heartbeats = 0_u32;

    while started_at.elapsed() < keep_alive_for {
        if let Some(update) = orchestrator.child_status_by_name(child_name).await?
            && Some(update.timestamp_ms) != last_timestamp_ms
        {
            observed_heartbeats += 1;
            last_timestamp_ms = Some(update.timestamp_ms);
            println!(
                "host: heartbeat #{observed_heartbeats}: status={:?} timestamp_ms={}",
                update.status, update.timestamp_ms
            );
        }

        tokio::time::sleep(HEARTBEAT_POLL_INTERVAL).await;
    }

    println!(
        "host: observed {observed_heartbeats} heartbeats in {} seconds",
        keep_alive_for.as_secs()
    );
    Ok(())
}

fn child_binary_path() -> Result<PathBuf> {
    let current_exe = std::env::current_exe()?;
    let exe_name = std::env::consts::EXE_SUFFIX;
    let child_name = format!("child{exe_name}");

    current_exe
        .parent()
        .map(|parent| parent.join(child_name))
        .ok_or_else(|| {
            OrchestratorError::Io(std::io::Error::other(
                "failed to resolve example child binary path",
            ))
        })
}
