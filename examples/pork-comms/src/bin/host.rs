use std::path::PathBuf;

use pork::error::{OrchestratorError, Result};
use pork::orchestrator::ProcessOrchestrator;
use pork::spec::ProcessSpec;
use pork_comms::{ChildMessage, HostMessage};

const CHILD_MANAGED_NAME: &str = "example-child";

#[tokio::main]
async fn main() -> Result<()> {
    let orchestrator = ProcessOrchestrator::new();
    let child_binary = child_binary_path()?;

    println!("host: starting child at {}", child_binary.display());

    let child = orchestrator.start_process(
        ProcessSpec::new(child_binary)
            .managed_name(CHILD_MANAGED_NAME)
            .capture_output(),
    )?;

    let (process_id, managed_name) = child.identity();
    println!(
        "host: started child with id={} name={}",
        process_id,
        managed_name.unwrap_or("<unnamed>")
    );

    let lookup_id = orchestrator
        .process_id_by_name(CHILD_MANAGED_NAME)?
        .ok_or_else(|| OrchestratorError::ProcessNameNotFound(CHILD_MANAGED_NAME.to_owned()))?;
    println!("host: lookup by name resolved to process id={lookup_id}");

    let registered_names = orchestrator.process_names()?;
    println!("host: registered managed names: {registered_names:?}");

    let ready = recv_child_message(&child).await?;
    println!("child -> host: {ready:?}");

    send_host_message(&child, HostMessage::Status)?;
    let status = recv_child_message(&child).await?;
    println!("child -> host: {status:?}");

    for message in [
        "hello from host",
        "pork can exchange raw bytes",
        "this child echoes messages back",
    ] {
        let host_message = HostMessage::Echo(message.to_owned());
        println!("host -> child: {host_message:?}");
        send_host_message(&child, host_message)?;

        let response = recv_child_message(&child).await?;
        println!("child -> host: {response:?}");
    }

    println!("host: requesting graceful shutdown");
    let exit_status = orchestrator.graceful_shutdown_process(child.id())?;
    println!("host: child exited with status {exit_status}");

    Ok(())
}

async fn recv_child_message(child: &pork::orchestrator::ManagedChild) -> Result<ChildMessage> {
    let response = child.recv().await.ok_or_else(|| {
        OrchestratorError::Io(std::io::Error::new(
            std::io::ErrorKind::UnexpectedEof,
            "child closed its IPC channel before replying",
        ))
    })?;

    ChildMessage::decode(&response).map_err(|error| {
        OrchestratorError::Io(std::io::Error::new(std::io::ErrorKind::InvalidData, error))
    })
}

fn send_host_message(child: &pork::orchestrator::ManagedChild, message: HostMessage) -> Result<()> {
    child.send(message.encode())
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
