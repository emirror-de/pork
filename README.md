# pork

Small process orchestration for host/child IPC workflows in Rust.

`pork` helps you start child processes, establish a bootstrap handshake, exchange raw IPC messages, and shut children down gracefully using a shared control protocol. The workspace also includes `pork-proto`, a companion crate that provides the shared control-plane protocol types and codec implementations.

## What it is for

Use `pork` when you want to:

- supervise one or more child processes,
- connect parent and child over IPC without building the handshake yourself,
- send application-defined payloads between host and child,
- and keep framework-level shutdown behavior consistent.

## Workspace layout

This repository uses a workspace-first layout:

- repository root — shared workspace files, Nix development setup, and top-level documentation
- `pork/` — main `pork` library crate
- `pork-proto/` — shared protocol crate
- `examples/pork-comms/` — small host/child example showing typed message exchange

This keeps the workspace root focused on coordination while each crate owns its own manifest, source tree, and tests.

## Workspace crates

- `pork` — high-level orchestration API for starting, tracking, messaging, restarting, and stopping managed child processes through explicit modules such as `orchestrator`, `spec`, `child`, and `error`.
- `pork-proto` — shared protocol definitions in `protocol` plus feature-gated codec implementations in `codecs`.

## Quick mental model

A typical setup has two sides:

1. A **host** process creates a `pork::orchestrator::ProcessOrchestrator` and starts a child from a `pork::spec::ProcessSpec`.
2. A **child** process reads bootstrap information from the environment and connects back to the host with `pork::child::bootstrap`.
3. Both sides exchange raw `Vec<u8>` payloads through `pork`.
4. Shared control messages, typed IPC envelopes, and codec selection live in `pork_proto::protocol`.

If you only need process orchestration and raw byte transport, depend on `pork`.

If you want typed IPC payloads and the shared codec helpers, depend on both `pork` and `pork-proto`.

## Where to look next

For the actual API, examples, and behavior details, use the crate documentation:

- `pork` crate docs: see `pork/src/lib.rs`
  - `pork::orchestrator` for host-side process management
  - `pork::spec` for child process configuration
  - `pork::child::bootstrap` for child-side bootstrap helpers
- `pork-proto` crate docs: see `pork-proto/src/lib.rs`
  - `pork_proto::protocol` for protocol models, shared control messages, and typed IPC envelopes
  - `pork_proto::codecs` for `JsonCodec` and `PostcardCodec`

If you are browsing locally, the crate-level docs are the best starting point because they include the intended usage flow and focused examples for the namespaced API.

## Installation

### Prerequisites

You need:

- Rust `1.85` or newer
- a working Cargo toolchain
- Unix-like local IPC support for the current process model
- optional: Nix with flakes enabled if you want the provided development shell

### Build the workspace

From the repository root, run:

```/dev/null/install.sh#L1-2
cargo build --workspace
cargo test --workspace --all-targets
```

### Optional Nix development shell

If you use Nix for local development, enter the shell first and then run the same Cargo commands:

```/dev/null/install.sh#L1-2
nix develop
cargo test --workspace --all-targets
```

## Basic workflow

A typical workflow has three parts:

1. define how the child process should be started with `pork::spec::ProcessSpec`
2. start and manage the child from `pork::orchestrator::ProcessOrchestrator`
3. connect from the child side with `pork::child::bootstrap::child_connect_from_env`

For a complete typed example, see `examples/pork-comms/`.

## Quick example workflow

Host side sketch:

```/dev/null/host.rs#L1-16
use pork::orchestrator::ProcessOrchestrator;
use pork::spec::ProcessSpec;

async fn run_host() -> Result<(), pork::error::OrchestratorError> {
    let orchestrator = ProcessOrchestrator::new();
    let child = orchestrator
        .start_process(
            ProcessSpec::new("./child-binary")
                .managed_name("worker")
                .capture_output(),
        )
        .await?;

    child.send(b"ping".to_vec())?;
    let _status = orchestrator.graceful_shutdown_process(child.id()).await?;
    Ok(())
}
```

Child side sketch:

```/dev/null/child.rs#L1-11
use pork::child::bootstrap::child_connect_from_env;
use pork::DEFAULT_BOOTSTRAP_ENV;

async fn run_child() -> Result<(), pork::error::OrchestratorError> {
    let (from_host, to_host) = child_connect_from_env(DEFAULT_BOOTSTRAP_ENV).await?;
    let _ = (from_host, to_host);
    Ok(())
}
```

## Validate locally

From the repository root, run:

- `cargo fmt --all --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace --all-targets`

## Example project

The `examples/pork-comms/` crate demonstrates a small end-to-end setup with:

- a host binary
- a child binary
- typed messages encoded with `pork-proto`
- coverage for both JSON and Postcard codec flows

Use that example when you want a concrete reference before integrating `pork` into your own application.

## Status

This workspace is intentionally small and focused: the main orchestration API lives in the `pork` crate, while shared protocol details live in `pork-proto`. Users who want the full typed host/child workflow should add both crates as dependencies, keeping orchestration and protocol concerns explicit instead of relying on cross-crate re-exports.