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
- `examples/pork-comms/` — end-to-end host/child example showing typed messages, codec selection, and child status reporting

This keeps the workspace root focused on coordination while each crate owns its own manifest, source tree, and tests.

## Workspace crates

- `pork` — high-level orchestration API for starting, tracking, messaging, restarting, and stopping managed child processes through explicit modules such as `orchestrator`, `spec`, `child`, and `error`.
- `pork-proto` — shared protocol definitions in `protocol` plus feature-gated codec implementations in `codecs`.

## Quick mental model

A typical setup has two sides:

1. A **host** process creates a `pork::orchestrator::ProcessOrchestrator` and starts a child from a `pork::spec::ProcessSpec`.
2. A **child** process reads bootstrap information from the environment and connects back to the host with `pork::child::bootstrap`.
3. Both sides exchange typed `pork::types::DataPayload` values through `pork`.
4. Shared control messages, encoded `pork::types::ControlPayload` values, typed IPC envelopes, and codec selection live in `pork_proto::protocol`.

If you only need process orchestration and raw byte transport, depend on `pork`.

If you want typed IPC payloads and the shared codec helpers, depend on both `pork` and `pork-proto`.

## Feature flags and dual-channel architecture

`pork` uses feature flags to enable host-side and child-side APIs:
- `host` (default) — host APIs for process management
- `client` (default: off) — child APIs for bootstrap and connection

With both features enabled, `pork` establishes two IPC channels:
1. **Data channel** — application payloads
2. **Control channel** — codec-encoded framework messages (`GracefulShutdown`, `Restart`, status updates)

On the child side, `ChildBootstrap::connect` provides one API surface with two independent
receive workers (`recv_data` and `recv_control`). Heavy data traffic therefore cannot block
control-plane reception.

`ProcessSpecBuilder` configures the bootstrap environment-variable names and optional
managed child name before producing an immutable `ProcessSpec`:
- builder setters: `data_bootstrap_env(...)`, `control_bootstrap_env(...)`, and `managed_name(...)`
- `ProcessSpec` accessors: `data_bootstrap_env_ref()`, `control_bootstrap_env_ref()`, and `managed_name()`

The orchestrator reads those values when spawning the child, and `ChildBootstrap::from_env`
expects both bootstrap variable names. In the common case, use the default `ProcessSpecBuilder`
settings on the host and `ChildBootstrap::from_default_env()` on the child.

See `pork/src/host.rs` for `HostBootstrap` and `pork/src/child/bootstrap.rs` for `ChildBootstrap` documentation.

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

- Rust toolchain (this workspace targets Rust `1.85`)
- Cargo
- Unix-like local IPC support for the current process model
- Optional: Nix with flakes enabled if you want the provided development shell

### Build the workspace

Using Nix (recommended for reproducible developer shells):

```sh
nix develop -c cargo build --workspace
nix develop -c cargo test --workspace --all-targets
nix develop -c cargo test --workspace --all-features --all-targets
```

Without Nix (plain Cargo):

```sh
cargo build --workspace
cargo test --workspace --all-targets
cargo test --workspace --all-features --all-targets
```

### Optional Nix development shell

If you prefer an interactive shell, enter it first and then run the same Cargo commands inside that shell:

```sh
nix develop
cargo test --workspace --all-features --all-targets
```

## Basic workflow

A typical workflow has three parts:

1. define how the child process should be started with `pork::spec::ProcessSpec`
2. start and manage the child from `pork::orchestrator::ProcessOrchestrator`
3. connect from the child side with `pork::child::bootstrap::ChildBootstrap`

For a complete typed example, see `examples/pork-comms/`.

For heartbeat-based child status reporting, combine `ChildBootstrap` with
`pork::child::status_reporter::StatusReporter` in the child process and query the latest
child-reported status from the host with `ProcessOrchestrator::child_status` or
`ProcessOrchestrator::child_status_by_name`.

## Quick example workflow

Host side sketch:

```rust
use pork::orchestrator::ProcessOrchestrator;
use pork::spec::ProcessSpec;

async fn run_host() -> Result<(), pork::error::OrchestratorError> {
    let orchestrator = ProcessOrchestrator::new();
    let child = orchestrator
        .start_process(
            ProcessSpec::builder("./child-binary")
                .managed_name("worker")
                .log_output("./worker.log")
                .build(),
        )
        .await?;

    child.send("ping")?;
    let _status = orchestrator.graceful_shutdown_process(child.process_id()).await?;
    Ok(())
}
```

Child side sketch:

```rust
use pork::child::bootstrap::ChildBootstrap;

async fn run_child() -> Result<(), pork::error::OrchestratorError> {
    let channels = ChildBootstrap::from_default_env()?.connect().await?;

    channels.send_data("ready")?;
    while let Some(payload) = channels.recv_data().await {
        let _ = payload;
    }

    Ok(())
}
```

## Validate locally

Run these checks before making a release or merging large changes. These match what CI enforces.

Formatting and lints

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

Tests and docs

```sh
cargo test --workspace
cargo test --workspace --doc
cargo test --workspace --all-features --all-targets
```

Security & license checks

```sh
# install tools if you don't have them already
cargo install --locked cargo-audit cargo-deny

# run the checks
cargo audit
cargo deny check
```

MSRV (verify compilation on minimum supported Rust)

```sh
rustup toolchain install 1.85.0
rustup run 1.85.0 cargo check --workspace
```

Nix-based validation

```sh
nix flake check
```

## Security and license checks

This repository uses `cargo-audit` and `cargo-deny` to enforce advisories and license policies in CI. The cargo-deny configuration lives at `deny.toml` (root of the `pork/` workspace). CI runs `cargo audit` and `cargo deny check` as part of the `security` job; run the same commands locally before releasing.

The `deny.toml` file also contains any temporary advisory suppressions that have been reviewed and accepted with a plan to remediate (for example, a transitive unmaintained crate that currently has no safe upgrade path). Treat suppressions as temporary and track follow-up work to remove them.

## Example project

The `examples/pork-comms/` crate demonstrates a small end-to-end setup with:

- a host binary
- a child binary
- typed messages encoded with `pork-proto`
- coverage for both JSON and Postcard codec flows
- child-to-host status reporting over the control channel

Use that example when you want a concrete reference before integrating `pork` into your own application.

**Note:** example crates are `publish = false` in their Cargo.toml to avoid accidental publishing.

## Release status & publishing

This workspace is prepared for the `2.0.0` release line: the main orchestration API lives in the `pork` crate, while shared protocol details live in `pork-proto`.

Before publishing, ensure CI is green and perform these validation steps locally (see the `Validate locally` section above).

The publish order is:
1. `pork-proto`
2. `pork`

The workspace currently uses a local path dependency from `pork` to `pork-proto` together with the matching published version requirement. Validate both crates with dry runs first, then publish in that order.

Recommended pre-publish commands

```sh
cargo package --manifest-path pork-proto/Cargo.toml
cargo publish --dry-run -p pork-proto
cargo package --manifest-path pork/Cargo.toml
cargo publish --dry-run -p pork
```

## Where we enforce CI

CI is defined in `.github/workflows/ci.yml` and runs the following gates on PRs and pushes to release branches:

- formatting (`cargo fmt --all -- --check`)
- clippy (`cargo clippy --workspace --all-targets -- -D warnings`)
- workspace tests (`cargo test --workspace`)
- all-features and all-targets tests (`cargo test --workspace --all-features --all-targets`)
- feature-matrix checks for `pork` and `pork-proto`
- documentation tests (`cargo test --workspace --all-features --doc`)
- MSRV compile check (Rust 1.85)
- security and license checks (`cargo audit`, `cargo deny check`)
- `nix flake check`

## Where to look next

- See `pork/src/lib.rs` and `pork-proto/src/lib.rs` for crate-level documentation and examples.
- Look at `.github/workflows/ci.yml` for the exact CI jobs and expected checks.
