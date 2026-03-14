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

- repository root — virtual workspace and shared workspace files
- `pork/` — main `pork` library crate
- `pork-proto/` — shared protocol crate

This keeps the workspace root focused on coordination while each crate owns its own manifest, source tree, and tests.

## Workspace crates

- `pork` — high-level orchestration API for starting, tracking, messaging, restarting, and stopping managed child processes through explicit modules such as `orchestrator`, `spec`, `child`, `error`, and `proto`.
- `pork-proto` — shared protocol definitions in `protocol` plus feature-gated codec implementations in `codecs`.

## Quick mental model

A typical setup has two sides:

1. A **host** process creates a `pork::orchestrator::ProcessOrchestrator` and starts a child from a `pork::spec::ProcessSpec`.
2. A **child** process reads bootstrap information from the environment and connects back to the host with `pork::child::bootstrap`.
3. Both sides exchange raw `Vec<u8>` payloads.
4. Shared control messages and codec selection live under `pork::proto::protocol` or directly in `pork_proto::protocol`.

## Where to look next

For the actual API, examples, and behavior details, use the crate documentation:

- `pork` crate docs: see `pork/src/lib.rs`
  - `pork::orchestrator` for host-side process management
  - `pork::spec` for child process configuration
  - `pork::child::bootstrap` for child-side bootstrap helpers
  - `pork::proto::protocol` for shared protocol items
  - `pork::proto::codecs` for codec marker types
- `pork-proto` crate docs: see `pork-proto/src/lib.rs`
  - `pork_proto::protocol` for protocol models and helpers
  - `pork_proto::codecs` for `JsonCodec` and `PostcardCodec`

If you are browsing locally, the crate-level docs are the best starting point because they include the intended usage flow and focused examples for the namespaced API.

## Validate locally

From the repository root, run:

- `cargo fmt --all --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace --all-targets`

If you use Nix for local development, enter the shell first with `nix develop` and then run the same commands.

## Status

This workspace is intentionally small and focused: the main orchestration API lives in the `pork` crate, while shared protocol details live in `pork-proto`, with both crates exposing explicit, domain-specific modules instead of a flat crate-root API.