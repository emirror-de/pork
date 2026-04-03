
# Rust Performance Best Practices for AI-Assisted Development

## Purpose
Provide focused, modern guidance for measuring and improving Rust performance without sacrificing correctness, maintainability, portability, or safety. This ruleset enforces a profiling-first workflow, reproducible benchmarks, and clear documentation for performance-sensitive changes.

## Scope
- Applies to Rust source files, benchmark harnesses, and Cargo manifests involved in performance-sensitive work.
- Covers profiling, benchmarking, hot-path allocation behavior, algorithm and data-structure choice, async/runtime overhead, memory layout, feature gating, and observability for production regressions.
- Complements `rust-rust` for general Rust guidance and `rust-safety` for any optimization that touches `unsafe`, FFI, SIMD intrinsics, or layout-sensitive code.

## Rules
- MUST profile before optimizing; every non-trivial performance change MUST identify the measured bottleneck first.
- MUST describe the workload being optimized, including representative input sizes, concurrency level, and success metric (for example throughput, p95/p99 latency, allocation count, peak RSS, or startup time).
- MUST add or update reproducible benchmarks for hot paths affected by the change; prefer `criterion` for steady-state measurements and stable comparison baselines.
- MUST validate performance claims with before/after measurements captured under the same feature flags, compiler profile, hardware class, and operating-system conditions.
- MUST keep correctness first: tests, invariants, and error handling MUST remain intact while optimizing.
- MUST prefer algorithmic and data-layout improvements over micro-optimizations when profiling shows asymptotic or cache-behavior limits.
- MUST minimize unnecessary allocations on hot paths by reusing buffers, reserving capacity, avoiding needless clones, and preferring borrowing where ownership is not required.
- MUST choose data structures based on measured access patterns, mutation frequency, ordering requirements, cache locality, and concurrency needs.
- MUST document the expected complexity and important runtime characteristics of public performance-sensitive APIs; keep docs user-facing and omit process or provenance commentary.
- SHOULD use slices, iterators, `&str`, `Cow`, and stack-friendly representations where they materially reduce ownership churn or allocation pressure.
- SHOULD benchmark serialization, parsing, and I/O boundaries separately from business logic so regressions can be localized.
- SHOULD treat async performance separately from sync throughput: measure scheduler overhead, task fan-out, backpressure, and blocking isolation explicitly.
- SHOULD use zero-cost abstractions where they stay readable; remove abstraction layers only when profiling demonstrates meaningful overhead.
- SHOULD evaluate cache locality, branch predictability, false sharing, and contention for hot concurrent paths.
- SHOULD use SIMD, parallelism, custom allocators, or `unsafe` only after profiling and only with explicit portability, maintenance, and safety rationale.
- SHOULD keep benchmark fixtures deterministic and synthetic; avoid production payloads, secrets, or user data.
- MUST NOT merge performance claims without reproducible evidence or a documented measurement methodology.
- MUST NOT trade away safety, input validation, or observability solely for a microbenchmark win unless the trade-off is explicitly reviewed and accepted.

## Examples

Good: reserve capacity and keep the hot path allocation-aware
```
pub fn normalize_ascii_letters(input: &str) -> String {
    let mut out = String::with_capacity(input.len());

    for byte in input.bytes() {
        if byte.is_ascii_alphabetic() {
            out.push((byte as char).to_ascii_lowercase());
        }
    }

    out
}

pub fn collect_even_ids(ids: &[u64]) -> Vec<u64> {
    let mut out = Vec::with_capacity(ids.len() / 2 + 1);

    for &id in ids {
        if id % 2 == 0 {
            out.push(id);
        }
    }

    out
}
```

Bad: repeated allocation and unnecessary cloning inside the hot path
```
pub fn normalize_ascii_letters(input: &str) -> String {
    let mut out = String::new();

    for ch in input.chars() {
        if ch.is_ascii_alphabetic() {
            out.push_str(&ch.to_ascii_lowercase().to_string());
        }
    }

    out
}

pub fn collect_even_ids(ids: &[u64]) -> Vec<u64> {
    ids.iter()
        .cloned()
        .filter(|id| id % 2 == 0)
        .collect()
}
```

## Anti-Patterns to Avoid
- ❌ Premature optimization without a profiler trace, benchmark, or production metric signal.
- ❌ Repeated allocation on hot paths when capacity or reuse is predictable.
- ❌ Optimizing a cold function because it “looks slow” while ignoring measured bottlenecks elsewhere.
- ❌ Comparing benchmark runs captured with different feature flags, input shapes, hardware, or compiler settings.
- ❌ Using `unsafe`, target-specific intrinsics, or parallelism as a first step instead of exhausting simpler approaches first.
- ❌ Replacing clear APIs with opaque micro-optimized code that has no measured justification.

## Quality Assurance
- For every non-trivial performance change:
  - Add or update a benchmark in `benches/` or the project’s approved benchmark location.
  - Capture before/after measurements with the exact commands used to run them.
  - Record environment details sufficient to reproduce results: CPU class, OS, Rust toolchain, profile, and feature flags.
  - Re-run functional tests and linting before interpreting benchmark output.
- Recommended measurement workflow:
  - Use `cargo bench` with `criterion` for steady-state comparisons.
  - Use a profiler such as `cargo flamegraph`, Instruments, `perf`, or an equivalent platform-native tool to confirm where time is spent.
  - Use allocation-aware tools when relevant (for example `dhat`, jemalloc profiling, or allocator statistics).
  - Use request-level telemetry or tracing spans to correlate synthetic results with production behavior.
- CI guidance:
  - Run `cargo fmt --all --check`, `cargo clippy --workspace --all-targets --deny warnings`, and `cargo test --all-targets --all-features` before publishing benchmark numbers.
  - Run long-running benchmarks in a dedicated workflow or on demand; keep default CI fast and deterministic.
  - Preserve benchmark baselines and artifacts for reviewer comparison when a PR makes measurable performance claims.
- Validation checklist for a performance PR:
  - [ ] Bottleneck identified from profiling or production telemetry.
  - [ ] Benchmark workload reflects realistic usage.
  - [ ] Before/after numbers captured under comparable conditions.
  - [ ] Tests, formatting, and linting pass.
  - [ ] Any new dependency or feature flag is minimal and justified.
  - [ ] Trade-offs in readability, portability, memory, or safety are documented.

## Security Considerations
- MUST use synthetic, anonymized, or generated data in benchmarks and profiler captures.
- MUST NOT include secrets, tokens, production payloads, or raw PII in benchmark fixtures, trace files, or screenshots.
- SHOULD review caching, pooling, and memoization changes for data-retention and multi-tenant isolation risks.
- SHOULD treat perf-driven `unsafe`, FFI, or SIMD changes as safety-sensitive and route them through the requirements in `rust-safety`.

## AI Assistant Interaction Guidelines
When requesting performance help, provide:
- The exact file, module, or function under investigation.
- The observed symptom and target metric.
- Representative workloads or dataset shape.
- Existing profiling output, benchmark data, or telemetry summaries when available.
- Constraints such as portability, MSRV, memory budget, feature flags, or dependency limits.

Expected assistant output should include:
- A short plan focused on the measured bottleneck.
- Minimal code changes rather than speculative rewrites.
- Benchmark and validation updates for the affected path.
- Exact commands to reproduce measurements and regression checks.

If the goal or workload is unclear, the assistant MUST ask one focused clarifying question before proposing an optimization.

## Precedence and Overrides
- Depends on: `rust-rust`.
- Works alongside `rust-async` for async-specific throughput and latency concerns.
- Defers to `rust-safety` for any change that relies on `unsafe`, layout guarantees, raw pointers, or target-specific intrinsics.
- Overrides: none.

## Checklist
- [ ] Profiling-first workflow enforced.
- [ ] Workload and target metric documented.
- [ ] Benchmarks added or updated for affected hot paths.
- [ ] Before/after measurements captured under comparable conditions.
- [ ] Allocation, algorithm, and data-structure choices justified by evidence.
- [ ] Documentation explains performance characteristics without process noise.
- [ ] Security/privacy guidance applied to datasets and trace artifacts.
- [ ] Validation commands and reviewer reproduction steps provided.

## Changelog
- 1.2.0: Aligned the metadata wording with the canonical Rust ruleset style while keeping the profiling-first, reproducible-measurement guidance intact.
- 1.1.0: Modernized guidance for profiling-first workflows, reproducible measurements, async/runtime analysis, and evidence-based hot-path optimization.
- 1.0.0: Initial version extracted from `rust-rust` and formatted as a focused, actionable performance ruleset.
