
# CI and Quality Best Practices for AI-Assisted Development

Conformance and terminology: The key words "MUST", "MUST NOT", "REQUIRED", "SHALL", "SHALL NOT", "SHOULD", "SHOULD NOT", "RECOMMENDED", "NOT RECOMMENDED", "MAY", and "OPTIONAL" in this document are to be interpreted as described in RFC 2119 and RFC 8174 when, and only when, they appear in all capitals.

## Purpose
- Make quality checks fast, deterministic, and automated across languages and toolchains.
- Ensure changes are testable, reviewable, and safe to merge.
- Reduce regressions by failing early on format/lint/test/security issues.

## Scope
- Applies to CI pipelines and local developer workflows for any language/framework in this workspace.
- Covers formatting, linting, testing, dependency/supply-chain hygiene, and PR workflow expectations.
- Out of scope: provider-specific AI features or vendor-specific CI recommendations.

## Rules
### Test Execution
- CI MUST run the relevant automated tests for the change.
- Test commands MUST terminate and MUST NOT start long-running dev servers or file watchers.
- Flaky tests MUST be treated as defects and SHOULD be quarantined only with a time-bounded remediation plan.

### Formatting
- CI MUST check formatting in non-fixing mode and fail on violations.
- Tool versions SHOULD be pinned to keep formatting outputs stable over time.

### Linting
- CI MUST run linters with warnings treated as errors.
- If a lint rule must be disabled, the suppression MUST be narrowly scoped and MUST document the rationale.

### CI Pipeline Shape (Toolchain-Neutral)
- CI pipelines SHOULD include, in order:
  1) Setup runtime/toolchain (pinned versions)
  2) Install dependencies (with caching)
  3) Format check
  4) Lint (warnings as errors)
  5) Test
  6) Supply-chain checks (vuln/license/policy) where applicable
  7) Build/package (if applicable)

### Dependency and Supply-Chain Hygiene
- Dependency manifests and lockfiles MUST be committed and reviewed.
- CI SHOULD run vulnerability scanning and license/policy checks where applicable.
- Security exceptions MUST be documented with scope, justification, and an expiration/review date.
- Unused dependencies SHOULD be removed to keep the dependency graph minimal.

### Review and Merge Policy
- PRs MUST be small and focused; large changes SHOULD be split into staged PRs.
- Every PR MUST describe:
  - What changed and why
  - How it was validated (exact commands)
  - Security impact (data handling, secrets, permissions) when relevant
  - Performance impact when relevant (and how it was measured)

### Quality Gate Ownership
- CI MUST be the canonical place where formatting, linting, test, and supply-chain gates are enforced for merge decisions.
- Local workflows SHOULD mirror CI commands closely so failures are reproducible before review.
- Projects SHOULD keep default CI fast and deterministic, moving slower exhaustive checks into dedicated workflows where needed.

## Examples
Correct: toolchain-neutral CI stage ordering (pseudo-yaml)
```yaml
steps:
  - setup-runtime: <pinned>
  - restore-cache: <toolchain-specific>
  - install-deps: <toolchain-specific>
  - format-check: <toolchain-specific>
  - lint: <toolchain-specific, warnings-as-errors>
  - test: <toolchain-specific>
  - supply-chain: <toolchain-specific vuln/license/policy>
  - build: <toolchain-specific>
```

Incorrect: skipping quality gates and relying on manual review
```yaml
steps:
  - install-deps: <toolchain-specific>
  - build: <toolchain-specific>
```

## Anti-Patterns to Avoid
- ❌ Allowing formatting drift by not running a formatter in CI.
- ❌ Treating lint warnings as informational in CI.
- ❌ Merging changes without tests for non-trivial logic.
- ❌ Ignoring flaky tests instead of fixing them.
- ❌ Keeping unused dependencies or unchecked transitive dependencies.

## Quality Assurance
- CI configurations SHOULD be reproducible (pinned versions, deterministic caches).
- Critical workflows SHOULD be exercised periodically (scheduled CI) to catch slow regressions and supply-chain changes.
- Teams SHOULD track build duration and flaky-test rates and treat regressions as actionable.

## Security Considerations
- CI MUST NOT expose secrets in logs; redact sensitive outputs and avoid printing environment variables.
- Pipelines SHOULD use least-privilege tokens and short-lived credentials where supported.
- Supply-chain checks SHOULD be run in CI for dependency-heavy projects.

## AI Assistant Interaction Guidelines
- When requesting CI guidance, specify:
  - Language/toolchain(s)
  - Required quality gates (format/lint/test/security)
  - Expected runtime budget (e.g., fast PR checks vs nightly)
- Require outputs to remain toolchain-neutral unless the project already standardizes on specific tools.

## Precedence and Overrides
- Depends on: `global-global`.
- Overrides: none.

## Checklist
- [ ] Format check runs in CI and fails on violations
- [ ] Lint runs in CI with warnings treated as errors
- [ ] CI runs the relevant automated tests for the change
- [ ] CI does not start long-running processes (dev servers/watchers)
- [ ] Dependency manifests + lockfiles are reviewed and supply-chain checks run where applicable
- [ ] CI remains the canonical merge gate for validation
- [ ] PR descriptions include what changed, why, and exact validation commands

## Changelog
- 2.1.0: Removed overlap with the global testing guidance, made CI the canonical enforcement layer for quality gates, and normalized review/validation wording.
- 2.0.0: Standardized structure; strengthened supply-chain, determinism, and workflow requirements.
- 1.0.0: Initial CI and quality guidance.
