
# Rust Best Practices for AI-Assisted Development

## Purpose
Provide the base Rust rules for this workspace: modern language idioms, workspace configuration, API design, error handling, testing, observability, and security defaults that other Rust rules build on.

## Scope
- Applies to all Rust source files, `Cargo.toml`, `Cargo.lock`, workspace manifests, and shared toolchain/lint configuration.
- Applies to production code, tests, examples, benchmarks, and supporting manifests unless a narrower Rust rule states otherwise.
- Acts as the foundational Rust ruleset; narrower Rust/framework/domain rules MUST declare `depends_on: ["rust-rust"]`.
- Establishes baseline expectations only. Narrower rules MAY tighten requirements, but they MUST document intentional overrides.
- Defines the base expectations for Rust API design, type modeling, module structure, manifests, feature flags, error handling, and documentation. Higher-level application layering and test strategy belong in narrower Rust rules.

## Rules

### Language & Edition Baseline
- Crates MUST target Rust 2024 unless a documented compatibility constraint requires an older edition.
- Crates and workspaces SHOULD declare `rust-version` explicitly and keep the MSRV policy documented and enforced in CI.
- Public APIs SHOULD prefer stable language and library features over nightly-only or macro-heavy abstractions unless a clear benefit is documented.
- Naming MUST follow Rust API Guidelines: modules and functions `snake_case`, types and traits `UpperCamelCase`, constants and statics `SCREAMING_SNAKE_CASE`.

### API Design & Types
- Public APIs SHOULD borrow data (`&str`, slices, references) when ownership transfer is not required.
- Public APIs SHOULD prefer clarity over lifetime-heavy designs; use owned types when borrowing would make the API materially harder to understand or use.
- Types SHOULD encode invariants at compile time using enums, newtypes, smart constructors, and validated or non-zero types where appropriate.
- Behavior that depends on shared state, maintained invariants, or a cohesive workflow SHOULD usually be attached to the owning type through `impl` blocks rather than spread across free functions in a module.
- When multiple functions operate on the same data, state, or workflow, code SHOULD prefer introducing a small composition type and implementing focused methods on it when doing so makes responsibilities clearer.
- Small stateless free functions remain acceptable when they are the clearest representation of a local transformation, parsing step, formatting step, or boundary-level mapping and do not introduce a parallel abstraction pattern.
- Public collections and iterables SHOULD expose idiomatic iterator entry points (`iter`, `iter_mut`, `into_iter`) instead of bespoke traversal APIs.
- Conversions SHOULD use `From` and `TryFrom`; parsing SHOULD use `FromStr` when that improves ergonomics.
- `Default` SHOULD be implemented only when there is a clear, unsurprising default state.

### Trait Design
- Traits SHOULD be introduced only at real behavioral boundaries, multi-implementation seams, or reusable integration points.
- Concrete types SHOULD be preferred when a single implementation is sufficient and no stable abstraction boundary exists.
- Public traits SHOULD be small, behavior-oriented, and easy to implement.
- Traits MUST NOT be introduced as speculative abstractions for future flexibility.

### Boundary Types & Serialization
- Types SHOULD derive or implement `Serialize` and `Deserialize` only when they actually cross a serialization boundary.
- Domain types, transport DTOs, and persistence models SHOULD remain separate when they have different fields, validation rules, or lifecycle constraints.
- Conversions between boundary-specific types SHOULD be explicit and tested.
- Public serialized formats with compatibility requirements SHOULD document those requirements near the boundary that owns them.

### Control Flow & Idioms
- Code SHOULD prefer early returns, `let-else`, and exhaustive `match` expressions over deep nesting.
- Function definitions inside other functions MUST NOT be introduced.
- Shared local logic inside a function SHOULD be expressed through private sibling functions, closures for truly local behavior, or extracted methods on an appropriate type, whichever keeps responsibilities clearest.
- Pattern matching SHOULD be used to make state transitions and enum handling explicit.
- Iterator adapters SHOULD be preferred over manual loops when they improve clarity without obscuring control flow.
- `clone`, allocation, and owned-string creation SHOULD be deliberate; avoid hidden copies on hot or repeated paths.

### Modules & Visibility
- Modules SHOULD have one primary responsibility and a small public surface.
- Internal items SHOULD default to private or `pub(crate)`; wider visibility requires a real consuming call site or public API need.
- Top-level modules SHOULD represent stable domains or capabilities rather than vague buckets such as `utils`, `misc`, or `helpers`.
- Modules SHOULD NOT become collections of loosely related free functions when the code would be modeled more clearly as a composed type with focused responsibilities and `impl` methods.
- Small groups of stateless free functions MAY remain at module scope when they form a clear, minimal API and introducing a dedicated type would not improve clarity or ownership modeling.
- Convenience helpers, utility modules, or helper abstractions MUST NOT be introduced unless explicitly requested.
- Cross-module boundaries SHOULD depend on domain types, small contracts, or well-scoped traits rather than broad concrete implementation details.
- Crates SHOULD expose a small, documented root module with clear public modules while keeping detailed implementation split into submodules by domain capability.
- Re-exports MUST NOT be used for any items defined inside the crate, including internal items, parent-module convenience exports, single-item child-module exports, crate-root convenience exports, or barrel-style re-export modules.
- Re-exports MAY be used only to expose external crates that are intentionally and tightly coupled to the crate's public API when doing so materially reduces downstream dependency burden.
- Any allowed re-export MUST be narrowly scoped, MUST NOT create alternate public paths for items defined inside the crate, and MUST NOT be used as a convenience helper or organizational shortcut.
- Prelude modules MUST contain only items that are required for essentially every normal use of the crate’s public API.
- Prelude modules MUST NOT become convenience grab-bags for optional, situational, or internal items.
- Non-test nested modules SHOULD live in separate files for navigation and maintainability.
- Inline nested modules MAY be used only for very small private groupings where a separate file would reduce clarity.
- When a domain grows, it SHOULD use a navigable structure such as `foo.rs` plus `foo/` submodules so the entry module remains discoverable.

### Error Handling
- Recoverable failures MUST use `Result`; panics MUST be reserved for invariant violations or unreachable states.
- Public error types MUST implement `std::error::Error`, `Display`, and `Debug`, and SHOULD be `Send + Sync + 'static`.
- Libraries SHOULD prefer typed errors (`thiserror` or equivalent); binaries and tools MAY use application-level wrappers (`anyhow`, `eyre`) at the top level.
- Error types SHOULD group failures by caller-relevant behavior rather than mirroring every dependency mechanically.
- Errors SHOULD carry actionable context, but MUST NOT leak secrets, credentials, or internal-only details in user-facing output.
- Boundary layers (HTTP, CLI, IPC, FFI) MUST translate internal failures into safe external responses.

### Async, Concurrency & Shared State
- Async Rust MUST remain non-blocking end-to-end; blocking work MUST be isolated onto dedicated blocking threads or worker pools.
- Shared mutable state SHOULD be minimized; prefer message passing, ownership transfer, or narrowly scoped synchronization.
- Global initialization MUST use safe primitives such as `OnceLock` or `LazyLock`; mutable globals are prohibited outside carefully reviewed abstractions.
- Public async APIs SHOULD document cancellation, timeout, and `Send` expectations when they matter to callers.

### Manifests, Features & Dependencies
- `Cargo.toml` SHOULD declare `edition = "2024"` and an explicit `rust-version` consistent with the documented MSRV policy.
- Dependencies SHOULD be minimal, maintained, and justified; unused or duplicate crates SHOULD be removed.
- Crates SHOULD keep dependencies scoped to what that crate truly needs, especially avoiding framework or database dependencies in reusable core crates or modules.
- Optional integrations SHOULD be represented explicitly through optional dependencies and additive feature flags.
- Feature flags SHOULD be capability-oriented and additive; they SHOULD NOT silently change core semantics.
- Expected feature combinations SHOULD be validated in CI when features materially change behavior.
- Workspaces SHOULD centralize shared dependency versions, resolver configuration, and lint policy when a workspace split is explicitly requested.
- Virtual workspace manifests MUST NOT be used to share or inherit package metadata across member crates; each crate MUST declare its own package metadata explicitly.
- Repositories SHOULD keep root-level quality and release tooling near the workspace manifest, for example CI workflows, dependency policy files, changelog tooling, and formatter configuration.

### Documentation, Linting & Observability
- Public APIs SHOULD document behavior, inputs, outputs, errors, panics,
  and safety invariants where applicable.
- Non-public functions, types, constants, statics, trait implementations,
  and modules SHOULD also be documented when that documentation adds
  meaningful value for understanding intent, invariants, constraints,
  non-obvious behavior, or usage within the crate.
- Inline comments SHOULD be added where they materially improve a reader's
  understanding of intent, invariants, constraints, non-obvious control
  flow, or why a particular pattern is used.
- Inline comments MUST NOT merely restate obvious code or become line-by-line
  narration.
- Inline comments SHOULD explain rationale, assumptions, and behavioral
  implications, SHOULD complement rather than replace clear naming, type
  design, tests, and appropriate logging or tracing, and MUST keep each
  comment line at 80 characters or fewer.
- Public crates SHOULD include crate-level documentation that explains their
  purpose, primary entry points, and important feature flags.
- Public modules SHOULD be documented when they define meaningful domain or
  architectural boundaries.
- Module documentation MUST live in the module file itself using inner doc
  comments (`//!` or `/*! ... */`).
- Outer doc comments on module declarations (`///` or `/** ... */` attached
  to `mod`) MUST NOT be used for module documentation.
- Crates SHOULD declare a consistent lint baseline early; lint suppressions
  MUST remain narrow and justified.
- Inner or outer `allow` or `deny` attributes on modules MUST NOT be added
  unless the user explicitly requests them.
- Production-facing paths SHOULD use structured logging or tracing and stable
  metric names; logs MUST avoid secrets and raw sensitive payloads.
- When logging adds real diagnostic or explanatory value, Rust code SHOULD
  use `tracing` macros intentionally: `trace!` for deep internal execution
  details, `debug!` for developer-oriented insight into state changes and
  control flow, and `info!` for significant domain or operational events
  that operators and developers are expected to notice during normal
  investigation.
- Logging SHOULD clarify behavior at meaningful boundaries, state
  transitions, and key outcomes without becoming noisy, and MUST remain
  structured, redact sensitive data, and avoid replacing proper types,
  tests, or error handling.
- Example applications SHOULD validate the intended public API surface rather
  than relying on internal-only shortcuts.

## Examples

Correct: type-safe API with borrowing, typed errors, and explicit validation
```
use std::num::NonZeroUsize;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SlugError {
    #[error("slug must not be empty")]
    Empty,
}

pub fn normalize_slug(input: &str, max_len: NonZeroUsize) -> Result<String, SlugError> {
    let slug = input.trim().to_ascii_lowercase();

    if slug.is_empty() {
        return Err(SlugError::Empty);
    }

    Ok(slug.chars().take(max_len.get()).collect())
}
```

Correct: boundary-specific DTO stays separate from the domain type
```
use serde::Deserialize;

pub struct EmailAddress(String);

pub struct User {
    pub email: EmailAddress,
}

#[derive(Deserialize)]
pub struct CreateUserRequest {
    pub email: String,
}
```

Incorrect: panicking API with unnecessary ownership and vague naming
```
pub fn do_it(value: String) -> String {
    let output = value.trim().to_lowercase();
    assert!(!output.is_empty());
    output
}
```

Incorrect: one type reused for API, persistence, and domain concerns
```
#[derive(serde::Serialize, serde::Deserialize)]
pub struct User {
    pub id: i64,
    pub email: String,
    pub password_hash: String,
    pub created_at: String,
}
```

## Anti-Patterns to Avoid
- ❌ `unwrap` or `expect` in production paths where failures are recoverable.
- ❌ Broad `pub` exposure for internals that have no real external consumer.
- ❌ Adding serialization derives, trait impls, or abstractions before there is an actual use site.
- ❌ Introducing speculative traits for “future flexibility” without a present boundary or second implementation.
- ❌ Re-exporting any items defined inside the crate, including internal items, parent-module convenience exports, child-module items, crate-root convenience exports, or barrel-style re-export modules.
- ❌ Creating multiple public paths to the same crate-defined item through `pub use`, including at crate root.
- ❌ Re-exporting external crates when they are not intentionally and tightly coupled to the crate’s public API or when the re-export does not materially reduce downstream dependency burden.
- ❌ Using a `prelude` module as a convenience grab-bag instead of limiting it to universally required public API items.
- ❌ Sharing or inheriting package metadata from a virtual workspace manifest instead of declaring it explicitly in each crate manifest.
- ❌ Deeply nested control flow where `let-else`, guards, or early returns would be clearer.
- ❌ Defining functions inside other functions instead of using a private sibling function, a closure for truly local behavior, or a method on an appropriate type.
- ❌ Introducing a dedicated type purely to satisfy style when a small stateless free function is clearer and does not own shared state, invariants, or workflow coordination.
- ❌ Accumulating many related free functions in a module when a small composition type with focused `impl` methods would model the workflow more clearly.
- ❌ Introducing dependencies, macros, or unsafe code for convenience without a documented need.
- ❌ Collapsing domain types, transport DTOs, and persistence models into one catch-all type by default.
- ❌ Creating vague catch-all modules such as `utils`, `misc`, or `helpers` instead of naming real responsibilities.
- ❌ Letting example applications depend on internal implementation details instead of the intended public API surface.
- ❌ Adding inner or outer `allow` or `deny` attributes to modules without an explicit user request.
- ❌ Enabling broad default features that force heavy optional dependencies onto users who only need the core library.

## Quality Assurance
- Run `cargo fmt --all --check`, `cargo clippy --workspace --all-targets --deny warnings`, and `cargo test --all-targets --all-features` before merging.
- Add targeted tests alongside each non-trivial change and include regression tests for bug fixes.
- Validate expected feature combinations in CI when features materially change behavior.
- Use benchmark or profiling tooling when making performance claims rather than relying on intuition.
- Keep lint suppressions narrow and justified; prefer fixing the underlying issue.
- When a project uses examples, validate that they still compile and continue to demonstrate the intended public API surface and feature combinations.

## Security Considerations
- Validate and sanitize all external inputs at trust boundaries.
- Do not log secrets, tokens, raw credentials, or sensitive personal data.
- Prefer vetted ecosystem crates for cryptography, parsing, and protocol handling rather than custom implementations.
- Keep unsafe code isolated, documented, and reviewed under the dedicated Rust safety rules.

## AI Assistant Interaction Guidelines
- Provide file or module context, API constraints, relevant manifests, and representative inputs when requesting Rust changes.
- Ask for minimal, testable changes with any required imports, type definitions, conversions, and error handling included.
- Require generated code to preserve canonical module boundaries, avoid speculative abstractions, keep one clear public path per exported item, and prefer composition types with focused `impl` methods when behavior belongs to shared state, invariants, or a cohesive workflow.
- Allow small stateless free functions when they are clearer than introducing a dedicated type and do not create a parallel abstraction pattern.
- Reject generated Rust that defines functions inside other functions; require extraction to a private sibling function, a closure for truly local behavior, or a method on an appropriate type.
- When performance, async behavior, unsafe code, or serialized boundaries are involved, require tests and explicit validation steps.

## Precedence and Overrides
- This is the base Rust ruleset for the category.
- Narrower Rust, framework, or domain rules MUST depend on this file and MAY tighten requirements for their scopes.
- Intentional contradictions MUST be documented in front matter via `overrides`.

## Checklist
- [ ] Rust 2024 and MSRV expectations are documented and consistent
- [ ] Public APIs use idiomatic naming, borrowing, visibility, typed invariants, and attach behavior to types when shared state, invariants, or cohesive workflows make that the clearer design
- [ ] Traits are introduced only at real behavioral boundaries
- [ ] Boundary-specific types stay separate when domain, transport, and persistence concerns differ
- [ ] Recoverable errors use `Result`; panics are reserved for true invariants
- [ ] Functions are not defined inside other functions
- [ ] Modules have clear responsibilities, small public surfaces, use composition types with focused `impl` methods where they clarify ownership and workflow, keep small stateless free functions when they are the clearest minimal design, and do not re-export crate-defined items; any re-export is limited to tightly coupled external crates that are intentionally part of the public API
- [ ] Feature flags are explicit, additive, and validated where behavior changes materially
- [ ] Public crates and meaningful public modules are documented
- [ ] Non-public Rust definitions are documented where that documentation
      adds meaningful value
- [ ] Inline comments are added only where they materially improve
      understanding, do not narrate obvious code, and keep each comment line
      at 80 characters or fewer
- [ ] Module documentation uses inner doc comments in the module file itself,
      not outer doc comments on `mod` declarations
- [ ] Module-level `allow` and `deny` attributes are not added unless
      explicitly requested by the user
- [ ] Formatting, linting, and test commands are documented and enforced
- [ ] Observability and security expectations are considered for production
      paths
- [ ] `tracing` log levels are chosen intentionally: `trace!` for deep
      internal tracing, `debug!` for developer-facing behavior insight, and
      `info!` for significant domain or operational events

## Changelog
- 1.10.9: Added a carefully scoped Rust rule for meaningful inline comments,
  requiring comments to explain non-obvious intent or behavior without
  narrating obvious code and to keep each comment line at 80 characters or
  fewer.
- 1.10.8: Required documentation for non-public Rust definitions when it adds
  meaningful value for understanding intent, invariants, constraints,
  non-obvious behavior, or crate-internal usage.
- 1.10.7: Added generic Rust tracing guidance for intentional `trace!`,
  `debug!`, and `info!` usage so logging clarifies behavior without becoming
  noise.
- 1.10.6: Clarified that composition types and focused `impl` methods are preferred when behavior owns shared state, invariants, or cohesive workflows, while small stateless free functions remain acceptable when they are the clearest minimal design.
- 1.10.5: Prohibited nested function definitions and strengthened the preference for composition types with focused `impl` methods over clusters of related module-level functions.
- 1.10.4: Prohibited adding inner or outer `allow` or `deny` attributes to modules unless explicitly requested by the user.
- 1.10.3: Tightened the Rust baseline wording so crate-defined re-exports are prohibited in all forms and only tightly coupled external public dependencies remain allowed as a narrow exception.
- 1.10.2: Removed the internal re-export exception so re-exports are now allowed only for tightly coupled external crates that are intentionally part of the public API.
- 1.7.0: Simplified the re-export policy by prohibiting internal and parent-module convenience re-exports and allowing re-exports only for intentional public external dependencies that reduce downstream dependency burden.
- 1.6.0: Tightened the canonical public API path rule so parent-module re-exports are allowed only when the implementation module path is not part of the public API.
- 1.5.0: Required module documentation to live inside the module file via inner doc comments and prohibited outer doc comments on `mod` declarations for module docs.
- 1.4.0: Sharpened the base Rust rules to remove overlapping guidance, added canonical policies for traits and public paths, tightened boundary-type and feature-flag rules, and clarified scope across code, tests, examples, and manifests.
- 1.3.0: Added scalable Rust project structure guidance favoring a single-crate, module-first layout by default, explicit workspace splits only on request, stricter separate-file nested modules, and clear adapter and repository boundaries inside the crate.
- 1.2.0: Refreshed the base Rust rules to the standard workspace template and updated language, API, testing, observability, and tooling guidance.
- 1.1.0: Expanded modern Rust guidance, workspace linting, observability, and focused companion rules.
- 1.0.0: Initial version



---

Following these practices helps ensure generated and hand-written Rust code in this workspace remains maintainable, safe, and well-measured while being compatible with modern Rust idioms and AI-assisted development workflows.
