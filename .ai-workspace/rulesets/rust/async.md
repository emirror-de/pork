
# Rust Async Best Practices for AI-Assisted Development

## Purpose
Provide concise, actionable guidance for writing safe, efficient, and maintainable asynchronous Rust code. These rules focus on non-blocking design, structured concurrency, cancellation, deadlines/timeouts, streaming, backpressure, and how to reason about `Send`/`Sync` requirements in async contexts.

## Scope
- Applies to Rust source files matched by the globs above.
- Covers: async functions, futures, task spawning, structured concurrency, streaming APIs, deadlines/timeouts, cancellation, async traits, backpressure, and FFI crossing async boundaries.
- Does NOT replace general Rust API or performance rules — consult `rust-rust` and `rust-performance` for higher-level or performance-specific guidance. For unsafe concerns in async code consult `rust-safety`.

## Rules

- MUST use `async`/`await` for asynchronous functions and prefer end-to-end async designs where appropriate; do not mix blocking and async code without explicit isolation.
- MUST NOT perform blocking operations (for example `std::thread::sleep`, synchronous file I/O, blocking database drivers, or CPU-heavy loops) directly inside async tasks. Offload blocking work with `tokio::task::spawn_blocking`, dedicated worker threads, or runtime-appropriate non-blocking APIs.
- MUST choose the runtime and document it (for example `tokio` multi-threaded, `tokio` current-thread, or another supported runtime) in crate-level docs, operational docs, and test setup.
- MUST prefer structured concurrency over detached task spawning. Child tasks SHOULD be owned by a parent scope using `tokio::task::JoinSet`, `tokio_util::task::TaskTracker`, scoped supervisors, or equivalent lifecycle management.
- MUST treat `tokio::spawn` and equivalent APIs as ownership boundaries. If a task is spawned, its cancellation, error propagation, and shutdown path MUST be explicit.
- MUST prefer `Send` futures for tasks spawned onto a multi-threaded runtime. If a future is not `Send`, it MUST be confined to a `LocalSet`, `spawn_local`, or a documented single-threaded runtime boundary.
- MUST apply deadlines or timeouts to external calls and untrusted I/O. Document timeout rationale, retry interaction, and whether timeout means cancellation, retryability, or partial failure.
- MUST design cancellation-aware code: coordinate child tasks with `tokio::select!`, `JoinSet`, `TaskTracker`, or cancellation tokens; ensure cancellation frees resources, stops outbound work, and returns structured errors where callers need to react.
- MUST prefer streaming abstractions (`futures::Stream`, `tokio_stream`, framed I/O) for incremental processing instead of buffering unbounded data into memory.
- MUST use bounded channels, semaphores, or explicit queue limits on producer/consumer boundaries so async systems exert backpressure rather than growing without bound.
- MUST NOT hold mutex guards, rwlock guards, or other synchronization guards across `.await` unless the primitive and critical section are intentionally designed for that pattern and the behavior is documented.
- MUST bound fan-out concurrency for work over collections or request batches; use patterns such as bounded task sets, semaphores, or `buffer_unordered` with an explicit limit instead of unbounded spawning.
- SHOULD gate access to shared resources or outbound dependencies with async-aware semaphores (for example `tokio::sync::Semaphore`) so concurrency limits are centralized and observable.
- SHOULD propagate cancellation tokens (for example `tokio_util::sync::CancellationToken`) or equivalent shutdown handles through API boundaries so callers can compose graceful shutdown behavior.
- SHOULD use `tracing` for structured telemetry in async code, instrument spawned tasks via `.instrument(span.clone())`, and attach stable request or correlation IDs so logs, metrics, and traces remain linked.
- SHOULD prefer native `async fn` in traits on toolchains that support it; use `async-trait` only when MSRV or object-safety constraints require it, and document the allocation and dispatch trade-offs when that choice matters.
- SHOULD document task ownership and lifetime expectations in public APIs, including who spawns, who awaits, who cancels, and what cleanup is guaranteed on shutdown.
- MUST document public async APIs with behavior, inputs/outputs, error semantics, cancellation behavior, and whether work continues after caller drop or timeout.
- MUST test async code under realistic concurrency using the project runtime test harness, including cancellation, timeouts, shutdown, backpressure, lock contention, and failure injection cases.
- MUST handle panics in spawned tasks explicitly: propagate through join handles where possible, log with context, and isolate or restart only with a documented supervisory policy.
- MUST NOT assume deterministic scheduling in production; correctness MUST hold under concurrency, races, reordering, and partial cancellation.

## Examples

Good: async function with timeout and spawn_blocking for work that must block
```
use tokio::time::{timeout, Duration};
use tokio::task;

async fn fetch_from_network() -> Result<Vec<u8>, anyhow::Error> {
    // represent non-blocking I/O (e.g., hyper, reqwest async)
    Ok(vec![1, 2, 3])
}

async fn process_heavy_sync_work(data: Vec<u8>) -> usize {
    // Offload CPU- or blocking-bound work to a blocking pool
    task::spawn_blocking(move || {
        // heavy CPU or blocking I/O
        data.len()
    })
    .await
    .expect("blocking task panicked")
}

pub async fn fetch_and_process() -> Result<usize, anyhow::Error> {
    // Apply a timeout to the network fetch to avoid indefinite waits
    let data = timeout(Duration::from_secs(5), fetch_from_network()).await??;
    let result = process_heavy_sync_work(data).await;
    Ok(result)
}
```

Good: async trait using `async-trait` with documented constraints
```
use async_trait::async_trait;

#[async_trait]
pub trait KeyValueStore: Send + Sync {
    async fn get(&self, key: &str) -> anyhow::Result<Option<String>>;
    async fn put(&self, key: String, value: String) -> anyhow::Result<()>;
}

// Implementation backed by an async client (non-blocking)
pub struct RemoteStore {
    // client: SomeAsyncClient
}

#[async_trait]
impl KeyValueStore for RemoteStore {
    async fn get(&self, key: &str) -> anyhow::Result<Option<String>> {
        // non-blocking client usage here
        Ok(Some(key.to_string()))
    }
    async fn put(&self, _key: String, _value: String) -> anyhow::Result<()> {
        Ok(())
    }
}
```

Bad: Blocking call inside an async fn (never do this)
```
use std::time::Duration;

async fn bad_sleep() {
    // ❌ Blocking the async runtime thread — bad!
    std::thread::sleep(Duration::from_secs(1));
}

async fn usage() {
    bad_sleep().await; // this will block the executor
}
```

Bad: Unnecessary repeated `.await` or awaiting non-Send future on multi-threaded runtime
```
async fn nested_awaits() {
    let fut = async { async { 42 } };
    // ❌ unnecessary double-await pattern
    let v = fut.await.await;
    println!("{}", v);
}
```

## Anti-Patterns to Avoid

- ❌ Blocking inside async code (`std::thread::sleep`, synchronous file/database drivers, CPU-heavy loops on runtime threads).
- ❌ Spawning tasks and then dropping the handle without an intentional ownership, shutdown, or observability strategy.
- ❌ Spawning non-`Send` futures onto a multi-threaded runtime without confinement.
- ❌ Holding a lock guard across `.await` and then wondering why contention, deadlocks, or latency spikes appear.
- ❌ Fan-out over large inputs with unbounded `tokio::spawn` or unbounded buffering.
- ❌ Ignoring cancellation semantics so dropped requests continue consuming resources.
- ❌ Buffering unbounded streams or channel queues in memory instead of streaming and applying backpressure.
- ❌ Relying on task scheduling order for correctness.
- ❌ Using `async-trait` for hot inner-loop code where boxing or dynamic dispatch materially affects performance, without measuring the trade-off.

## Quality Assurance

- Tests:
  - Use `#[tokio::test(flavor = "multi_thread")]` or an equivalent harness that matches the deployment runtime.
  - Add tests for cancellation and shutdown: verify tasks stop, resources are released, and background work does not outlive its owner unexpectedly.
  - Add timeout tests for external I/O and dependency failures; assert the returned error shape and cleanup behavior.
  - Add tests for bounded concurrency and backpressure when fan-out, queues, or semaphores are part of the design.
  - Add tests proving locks are released before awaited downstream work when shared state and synchronization are involved.
  - Include integration tests using the chosen async clients (or well-mocked async drivers) to validate behavior under network failures, slow dependencies, partial cancellation, and contention.
  - Use property-based or stress tests for concurrency invariants where applicable.
- CI:
  - Run `cargo fmt --all --check`, `cargo clippy --workspace --all-targets --deny warnings`, and `cargo test --all-targets --all-features` (or runtime-specific feature sets) before merging.
  - Document any additional runtime flags, feature flags, or environment assumptions required for local and CI runs.
  - Optionally run stress, loom, or concurrency-fuzzing jobs in separate workflows for critical synchronization code.
- Benchmarks:
  - If performance-sensitive, create representative benchmarks that simulate realistic concurrency, queue pressure, cancellation behavior, and lock contention; document environment and run commands.

## Security Considerations

- Timeouts, queue bounds, and rate limiting are important defense-in-depth controls: unbounded async tasks can be exploited for DoS.
- Validate and sanitize untrusted inputs before entering async code paths that allocate, enqueue, or spawn tasks.
- Avoid leaking sensitive data into logs, spans, or metrics; use structured logging with redaction when needed.
- Be cautious with `spawn_blocking` or worker-thread tasks that process untrusted data; isolate risky work and cap concurrency.
- When exposing streaming endpoints, enforce authentication/authorization, output bounds, and backpressure to prevent resource exhaustion.

## AI Assistant Interaction Guidelines

When requesting async-related changes from an AI assistant, provide:
- The exact file(s) and function(s) to change and the target runtime/environment.
- Representative workloads and concurrency scenarios (number of concurrent requests, expected payload sizes, dependency latency).
- Any profiling artifacts or observed issues (flamegraphs, traces, allocation counts, stalled tasks).
- Desired guarantees (for example latency targets, shutdown expectations, cancellation deadlines, or queue bounds).
- Constraints (no additional major dependencies, must be `Send`, must support graceful shutdown, and so on).

Expect assistant output to include:
- A short plan describing changes, trade-offs, and runtime assumptions.
- Small, focused edits with tests that run under the chosen async runtime.
- Recommendations for monitoring and benchmarks to validate behavior.

If the assistant is uncertain about runtime or deployment (single-threaded vs multi-threaded), it MUST ask one focused clarifying question before making changes that affect `Send`/`Sync` or task spawning semantics.

## Precedence and Overrides

- Depends on: `rust-rust` (id: `rust-rust`) for general Rust idioms and API shape.
- Order: this file runs after the base Rust rules and before specialized performance rules (where performance changes also require benchmarking guidance from `rust-performance`).

## Checklist

- [ ] Async public APIs document behavior, inputs/outputs, errors, timeout/deadline semantics, and cancellation behavior; no agent/process notes
- [ ] No direct blocking calls inside async functions; blocking work is isolated explicitly
- [ ] External or untrusted I/O has timeouts or deadlines where appropriate
- [ ] Locks and other synchronization guards are not held across `.await` unless intentionally designed and documented
- [ ] Fan-out concurrency, queues, and downstream access are explicitly bounded where pressure could grow
- [ ] Tests cover cancellation, timeout, shutdown, contention, and failure behavior
- [ ] Spawned futures on multi-threaded runtimes are `Send`, or are intentionally confined to documented local/single-threaded scopes
- [ ] Streaming APIs and work queues are bounded or otherwise backpressured
- [ ] Task ownership, cleanup, and shutdown paths are explicit
- [ ] `tracing` or equivalent instrumentation is present for observability
- [ ] CI enforces formatting and lints (`rustfmt`, `clippy`)

## Changelog
- 1.2.0: Tightened async guidance around lock handling across `.await`, bounded fan-out concurrency, and validation of contention and backpressure behavior.
- 1.1.0: Updated for current async best practice with stronger structured concurrency, shutdown ownership, backpressure, and cancellation guidance.
- 1.0.0: Initial version — establishes non-blocking principles, task/cancellation patterns, timeout guidance, streaming/backpressure recommendations, and test/CI expectations.
