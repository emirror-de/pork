use std::path::PathBuf;

use pork::error::{OrchestratorError, Result};
use pork::orchestrator::{ManagedChild, ProcessOrchestrator};
use pork::spec::ProcessSpec;
use pork_comms::{ChildMessage, HostMessage, decode_message, encode_message};
use pork_proto::protocol::PorkControlCodec;

const CHILD_MANAGED_NAME: &str = "example-child";

#[tokio::main]
async fn main() -> Result<()> {
    let orchestrator = ProcessOrchestrator::new();
    let child_binary = child_binary_path()?;

    println!("host: starting child at {}", child_binary.display());

    let control_codec = PorkControlCodec::Json;
    let child = orchestrator
        .start_process(
            ProcessSpec::new(child_binary)
                .managed_name(CHILD_MANAGED_NAME)
                .capture_output()
                .control_codec(control_codec),
        )
        .await?;

    let (process_id, managed_name) = child.identity();
    println!(
        "host: started child with id={} name={}",
        process_id,
        managed_name.unwrap_or("<unnamed>")
    );

    let lookup_id = orchestrator
        .process_id_by_name(CHILD_MANAGED_NAME)
        .await?
        .ok_or_else(|| OrchestratorError::ProcessNameNotFound(CHILD_MANAGED_NAME.to_owned()))?;
    println!("host: lookup by name resolved to process id={lookup_id}");

    let registered_names = orchestrator.process_names().await?;
    println!("host: registered managed names: {registered_names:?}");

    let ready = recv_child_message(&child, control_codec).await?;
    println!("child -> host: {ready:?}");

    send_host_message(&child, control_codec, HostMessage::Status)?;
    let status = recv_child_message(&child, control_codec).await?;
    println!("child -> host: {status:?}");

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
    let exit_status = orchestrator.graceful_shutdown_process(child.id()).await?;
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

    decode_message::<ChildMessage>(codec, &response)
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
