
# Rust Safety Rules for AI-Assisted Development

## Purpose
Provide a concise, actionable, and testable ruleset for designing, documenting, reviewing, and validating unsafe Rust. These rules help reviewers and automated tooling determine whether an unsafe boundary is necessary, locally justified, encapsulated behind safe APIs, and continuously verified with appropriate tooling.

## Scope
- Applies to all Rust source files matching the globs above.
- Covers: `unsafe` blocks/traits/functions, raw pointers, FFI boundaries, manual memory management, `static mut`, interior mutability, atomics, pinning, aliasing, initialization, layout assumptions, and concurrency invariants involving `Send`/`Sync`.
- Excludes: high-level API design guidance (see `rust-rust`) and performance-specific rules (see `rust-performance`) except where they intersect safety-critical code.

## Rules
- MUST minimize the use of `unsafe`; prefer safe abstractions or well-reviewed, small unsafe helpers.
- MUST enable `#![forbid(unsafe_op_in_unsafe_fn)]` or an equivalent crate-level lint policy for crates that contain unsafe code so every unsafe operation remains explicit.
- MUST isolate unsafe code into small, well-named modules/functions with narrow surface area and clear ownership boundaries.
- MUST document every `unsafe` block, `unsafe fn`, and `unsafe trait` with a clear `# Safety` section or adjacent `// SAFETY:` comment that lists preconditions, invariants, and why the unsafe is justified.
- MUST document what each unsafe item or public API does (inputs/outputs/behavior) alongside its safety invariants; keep docs user-facing and omit agent/process provenance.
- MUST wrap raw pointer, FFI, and manually managed memory interactions in safe abstractions that enforce invariants at their boundaries.
- MUST validate all pointer arithmetic and raw memory access with explicit bounds, alignment, initialization, provenance, and aliasing guarantees, either at the API boundary or through documented invariants.
- MUST avoid exposing `static mut` as a public mutable variable; prefer `OnceLock`, `LazyLock`, or synchronization primitives (`Mutex`, `RwLock`, `Atomic*`) with documented justification.
- MUST justify every use of `UnsafeCell`, `MaybeUninit`, `ManuallyDrop`, manual drop ordering, pin-projection, or raw pointer aliasing by documenting the lifetimes, aliasing guarantees, pinning guarantees, and initialization strategy in the adjacent safety docs.
- MUST use `#[repr(C)]` and layout tests when relying on struct layout for FFI or binary compatibility.
- MUST model FFI ownership explicitly: document who allocates, who frees, valid nullability, threading expectations, UTF-8/UTF-16 assumptions, and panic behavior across the boundary.
- MUST NOT allow Rust panics to unwind across FFI boundaries; catch and translate them into safe error codes/results when the foreign ABI cannot tolerate unwinding.
- MUST assert and/or test that public types documented to be `Send`/`Sync` actually implement those traits when relevant to concurrency guarantees, using compile-time assertions where possible.
- MUST treat `unsafe impl Send` and `unsafe impl Sync` as high-risk changes and document the exact invariants that make them sound.
- SHOULD use crates and primitives with well-audited unsafe code (e.g., `libc`, `nix`, `bytemuck`, `zerocopy`, `ffi-support`) rather than reimplementing unsafe bindings or layout tricks.
- SHOULD prefer safe wrappers around unsafe operations and provide unit/integration tests exercising both normal and boundary/error conditions.
- SHOULD run undefined-behavior detection, sanitizers, and fuzzing tools against unsafe code where practical (for example `miri`, `cargo-fuzz`, and sanitizer builds).
- MUST include an explicit safety explanation directly above each unsafe item explaining:
  - Assumptions made by the unsafe code
  - Who or what guarantees those assumptions
  - What callers must ensure, if applicable
  - Why a safe alternative is insufficient here
  - How the unsafe block is kept minimal and locally auditable

## Examples

Correct (safe abstraction + documented unsafe)
```ai-ruleset/src/rust/safety.md#L120-199
use std::ptr::NonNull;

/// Safe wrapper around an unsafe buffer pointer.
///
/// Safety:
/// - The pointer must be non-null and point to `capacity` properly-initialized elements of `T`.
/// - Callers must ensure exclusive access when using `as_mut` concurrently.
///
/// This wrapper exposes only safe methods; the unsafe code is contained and documented.
pub struct SafeBuffer<T> {
    ptr: NonNull<T>,
    capacity: usize,
}

impl<T> SafeBuffer<T> {
    /// # Safety
    /// The caller must ensure `ptr` is valid for reads/writes for `capacity` elements
    /// and that the memory is properly aligned for `T`.
    pub unsafe fn from_raw_parts(ptr: *mut T, capacity: usize) -> Self {
        // Safety invariants documented above
        Self {
            ptr: NonNull::new_unchecked(ptr),
            capacity,
        }
    }

    pub fn capacity(&self) -> usize {
        self.capacity
    }

    pub fn get(&self, idx: usize) -> Option<&T> {
        if idx < self.capacity {
            // Safety: bounds checked above; pointer is assumed valid per constructor doc
            unsafe { Some(&*self.ptr.as_ptr().add(idx)) }
        } else {
            None
        }
    }
}
```

Incorrect (undocumented/unchecked unsafe)
```ai-ruleset/src/rust/safety.md#L200-260
pub struct UnsafeVec<T> {
    ptr: *mut T,
    len: usize,
}

impl<T> UnsafeVec<T> {
    pub fn push(&mut self, value: T) {
        unsafe {
            // ❌ No bounds checks, no documented invariants, writes directly to pointer
            *self.ptr.add(self.len) = value;
            self.len += 1;
        }
    }
}
```

## Anti-Patterns to Avoid
- ❌ Large, scattered `unsafe` blocks with mixed responsibility instead of small, reviewed helpers.
- ❌ Omitting a `# Safety` docblock or `// SAFETY:` explanation describing invariants and caller responsibilities.
- ❌ Exposing raw pointers, `NonNull<T>`, or `*mut T` publicly without a safe wrapper or explicit ownership contract.
- ❌ Using `unsafe` to silence borrow-checker issues instead of redesigning APIs.
- ❌ Relying on undefined behavior as a performance hack without exhaustive tests and documentation.
- ❌ Using `static mut` for shared mutability without proper synchronization and explicit justification.
- ❌ Writing `unsafe impl Send` or `unsafe impl Sync` without documenting aliasing, thread-affinity, and interior-mutability guarantees.
- ❌ Assuming FFI callers respect Rust invariants unless the boundary validates and documents them explicitly.

## Quality Assurance
- Tests:
  - Unit tests covering safe wrappers, including boundary cases and invalid inputs where the wrapper must return `Err` or fail predictably.
  - Integration tests for FFI boundaries using representative synthetic inputs.
  - Property tests (for example `proptest`) for invariants where applicable.
  - Fuzz tests for inputs crossing unsafe boundaries (`cargo-fuzz`).
- Compile-time checks:
  - Add `static_assertions` where helpful to check `size_of`, alignment, niche/layout assumptions, and trait impls.
  - Where a type is claimed `Send`/`Sync`, add compile-time assertions in `#[cfg(test)]`:
    - `fn assert_send_sync<T: Send + Sync>() {}`
  - Add compile-time checks or snapshot tests for `#[repr(C)]` FFI structs when layout compatibility matters.
- Tooling:
  - Use `miri` (`cargo miri test`) for undefined-behavior detection on unsafe-heavy tests and document the exact command, target, and feature set.
  - Run sanitizer builds where supported (`RUSTFLAGS="-Zsanitizer=address"` / `thread` / `memory"` on the appropriate toolchain) for pointer-heavy or concurrency-heavy code.
  - Run `cargo fmt --all --check`, `cargo clippy --workspace --all-targets --deny warnings`, and `cargo test --all-targets --all-features` in CI before executing fuzzers or extended safety jobs so diagnostics reflect merged code.
  - Schedule `cargo fuzz` (or equivalent) campaigns for unsafe boundaries and capture corpus growth or coverage metrics.
  - Add periodic safety hygiene jobs (`cargo udeps`, `cargo audit`, sanitizer builds, `cargo geiger` where useful) so dead code, dependency drift, and unsafe-surface regressions are surfaced automatically.
- Review:
  - Peer review of any unsafe change is REQUIRED; include checklist items in the PR description:
    - Safety docs/comments present
    - Tests covering invariants and edge cases
    - Reasoning for unsafe necessity
    - `miri`, sanitizer, or fuzzing results where applicable
    - Explicit review of FFI ownership, layout, and panic behavior where relevant

## Security Considerations
- Do NOT embed secrets or PII in tests, examples, fuzz corpora, or benchmarking datasets.
- Be cautious with unsafe code that parses or processes untrusted input; validate, bound-check, and sanitize before handing data to unsafe paths.
- Document any potential side channels, out-of-bounds risks, use-after-free risks, double-free risks, or memory disclosure risks when using unsafe parsing or buffer handling.
- For FFI, validate and sanitize all inputs crossing the language boundary; consider ownership, lifetime translation, thread-affinity, and encoding assumptions explicitly.
- When using platform-specific unsafe features, note platform constraints, ABI assumptions, and fallbacks.
- Prefer fail-closed behavior at unsafe boundaries: reject invalid states rather than attempting implicit recovery from malformed foreign input.

## AI Assistant Interaction Guidelines
When asking an AI assistant to modify or create unsafe code, provide:
- The exact file(s) and the functions/modules to change.
- A clear statement of the invariant(s) the unsafe code will rely on.
- Representative inputs and edge cases (synthetic or anonymized).
- Any profiling or tooling evidence motivating the unsafe change.
- The desired safety contract, including what callers must guarantee.
- Any FFI constraints, layout guarantees, panic requirements, or thread-affinity rules.

Expect outputs to include:
- A short plan and rationale for any unsafe usage.
- Small, contained edits with safety docs/comments above every unsafe item.
- New or updated unit/integration tests plus any relevant `miri`, sanitizer, or fuzzing commands.
- Explicit discussion of whether a safe abstraction or existing crate can replace the proposed unsafe block.

If uncertain, the assistant MUST ask one focused clarifying question before applying changes that introduce `unsafe`.

## Precedence and Overrides
- Depends on: `rust-rust` (id: `rust-rust`) for general Rust idioms and API style.
- Order: set to run after the basic `rust-rust` to specialize on safety concerns.
- Overrides: if this file intentionally contradicts higher-level rules, document the rationale in `overrides`.

## Checklist
- [ ] Each `unsafe` block has nearby `# Safety` docs or `// SAFETY:` comments describing preconditions and who enforces them.
- [ ] Crates with unsafe code enforce explicit unsafe operations via `unsafe_op_in_unsafe_fn`.
- [ ] Public APIs and unsafe items are documented with behavior plus safety invariants; no agent/process provenance.
- [ ] Unsafe surface area is minimized and isolated behind safe APIs.
- [ ] Unit/integration tests are added or updated for unsafe boundaries.
- [ ] `miri`, fuzzing, sanitizers, or other UB-detection tools are used where practical.
- [ ] Compile-time assertions are added for layout, trait, and concurrency guarantees when relevant.
- [ ] FFI boundaries document ownership, panic behavior, nullability, and encoding assumptions.
- [ ] Peer review is required and the safety checklist is included in the PR description.
- [ ] No secrets or PII appear in tests, examples, or fuzz corpora.

## Changelog
- 1.2.0: Normalized metadata wording to align with the Rust ruleset category and kept version history consistent with the current file state.
- 1.1.0: Tightened unsafe-boundary guidance with explicit linting, FFI ownership and panic rules, sanitizer recommendations, and stronger review/tooling expectations.
- 1.0.0: Initial version — establishes conventions for documenting, testing, and auditing `unsafe` and other safety-sensitive Rust code.
