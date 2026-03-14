# pork

Small process orchestration for host/child IPC workflows in Rust.

`pork` helps you start child processes, establish a bootstrap handshake, exchange raw IPC messages, and shut children down gracefully using a shared control protocol. The workspace also includes `pork-proto`, a companion crate that provides the shared control-plane message types and codec helpers.

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

- `pork` — high-level orchestration API for starting, tracking, messaging, restarting, and stopping managed child processes.
- `pork-proto` — shared protocol primitives, control messages, and codec helpers for JSON and Postcard.

## Quick mental model

A typical setup has two sides:

1. A **host** process creates a `ProcessOrchestrator` and starts a child from a `ProcessSpec`.
2. A **child** process reads bootstrap information from the environment and connects back to the host.
3. Both sides exchange raw `Vec<u8>` payloads.
4. Shared control messages are used for graceful shutdown.

## Where to look next

For the actual API, examples, and behavior details, use the crate documentation:

- `pork` crate docs: see `pork/src/lib.rs`
- `pork-proto` crate docs: see `pork-proto/src/lib.rs`

If you are browsing locally, the crate-level docs are the best starting point because they include the intended usage flow and focused examples.

## Validate locally

From the repository root, run:

- `cargo fmt --all --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace --all-targets`

If you use Nix for local development, enter the shell first with `nix develop` and then run the same commands.

## Status

This workspace is intentionally small and focused: the main orchestration API lives in the `pork` crate, while shared protocol details live in `pork-proto`.