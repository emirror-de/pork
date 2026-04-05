
# Global Basic Coding Best Practices for AI-Assisted Development

Note (RFC 2119/8174): The key words MUST, MUST NOT, REQUIRED, SHALL, SHALL NOT, SHOULD, SHOULD NOT, RECOMMENDED, NOT RECOMMENDED, MAY, and OPTIONAL in this document are to be interpreted as described in RFC 2119 and RFC 8174 when, and only when, they appear in all capitals.

## Purpose
- Establish a universal baseline for simple, maintainable code across all languages and frameworks.
- Promote DRY, KISS, composition over inheritance, single responsibility, and YAGNI.
- Provide clear, testable rules and examples that scale by composing small, correct units.

## Scope
- Applies to all source files in this workspace: application code, scripts, and tests.
- Focuses on foundational coding practices, not language- or framework-specific details.
- Defines code-structure and implementation defaults only; testing policy, security/privacy policy, and collaboration workflow remain canonical in their narrower global rules.
- Complements language/framework rules, which MAY tighten these defaults for their own scopes.

## Rules
- Code SHOULD be simple, composable, and responsibility-focused; avoid unnecessary abstraction or over-engineering.
- Contributors SHOULD first inspect the existing repository for reusable models, types, functions, helpers, components, modules, and established patterns before introducing new ones.
- When an existing implementation already solves the problem or can be extended clearly and safely, contributors SHOULD reuse or adapt it instead of creating a parallel solution.
- A codebase SHOULD have one canonical way to solve a given problem; contributors MUST NOT introduce parallel implementations, helpers, abstractions, or re-export-based alternate access paths for the same goal unless explicitly requested or clearly justified.
- Core logic SHOULD stay small and composable, while I/O and orchestration SHOULD remain isolated at system boundaries.
- Inputs MUST be validated at boundaries and failures MUST be explicit, actionable, and safe to expose.
- Contributors MUST NOT introduce ambiguous Unicode characters in source text, identifiers, comments, documentation, configuration, or tests unless explicitly required and clearly justified by the task.
- Names SHOULD be clear and intentful, and public interfaces SHOULD remain small and explicit.
- Re-exports MUST NOT be introduced for items defined inside the codebase, including parent-module convenience exports, single-item child-module exports, barrel-style re-export modules, or any other alternate public path.
- Re-exports MAY be used only for external crates or packages that are intentionally and tightly coupled to the public API when doing so materially reduces downstream dependency burden.
- Any allowed re-export MUST be narrowly scoped, MUST NOT create alternate public paths for code defined inside the codebase, and MUST NOT be used as a convenience helper or organizational shortcut.
- Convenience helpers and convenience public APIs MUST NOT be introduced unless explicitly requested.
- Code SHOULD favor immutability and use guards or early returns to avoid unnecessary nesting.
- Composition SHOULD be preferred over inheritance; inheritance SHOULD be used only with a proven, stable hierarchy.
- Performance work MUST remain measurement-driven rather than speculative.
- When a task is complete and the response has no open questions, unresolved problems, blocked work, or pending decisions, assistants SHOULD suggest a conventional commit message when doing so adds practical value.
- Any suggested conventional commit message MUST follow the explicit response-format exception defined in `global-global` rather than redefining formatting rules here.

## Anti-Patterns to Avoid
- Duplicated logic across functions or modules.
- Creating new models, types, helpers, components, modules, or abstractions without first checking whether the repository already contains a reusable equivalent or an established canonical pattern.
- Parallel patterns, helpers, abstractions, or re-export-based access paths that achieve the same goal without an explicit need.
- Over-abstraction or inheritance without a concrete boundary or benefit.
- Deep nesting where guards or early returns would be clearer.
- Convenience helpers, public APIs, or internal re-exports added without explicit request or without a tightly coupled external-public-API justification.
- Mixing I/O with core computation in ways that make behavior harder to test and reason about.
- Omitting a practically useful conventional commit suggestion for a completed task even though the global exception allows it.
- Redefining conventional commit response formatting locally instead of deferring to the canonical exception in `global-global`.

## Quality Assurance
- Run formatting, linting, and fast tests locally before merging.
- Add or update tests for non-trivial changes covering success and error paths.
- Provide measurements when claiming performance gains; add benchmarks only when performance is the goal.
- Confirm that conventional commit suggestions are emitted only for completed tasks and defer their formatting to the canonical exception in `global-global`.

## Security Considerations
- Validate and sanitize inputs at boundaries; avoid trusting external data.
- Keep error messages actionable but non-revealing; do not leak internals.
- Avoid visually confusable or ambiguous Unicode characters because they can hide defects, reduce reviewability, and create security risks.
- Defer broader secret-handling and privacy requirements to the canonical global security rules.

## AI Assistant Interaction Guidelines
- Ask for the smallest change that delivers value while preserving the existing canonical pattern.
- Before adding new models, types, helpers, components, modules, or abstractions, inspect the repository for existing reusable equivalents or nearby canonical patterns and prefer extending them when that keeps the design clear and correct.
- Re-exports for code defined inside the codebase MUST NOT be introduced; only tightly coupled external public dependencies MAY be re-exported when explicitly justified by API ergonomics.
- Convenience helpers and convenience public APIs MUST NOT be introduced unless explicitly requested.
- Preserve plain, unambiguous text and avoid introducing visually confusable Unicode characters unless the task explicitly requires them.
- Request correct/incorrect examples only when boundaries are ambiguous.
- Ask for a measurement plan before performance work.
- When a completed task is fully wrapped up, assistants SHOULD consider suggesting a conventional commit message when it is practically useful and the global exception in `global-global` permits it.
- Assistants MUST NOT redefine code-block or placement rules for commit suggestions here and SHOULD rely on the canonical global exception instead.

## Precedence and Overrides
- Depends on: `global-global`.
- Overrides:
  - `global-global`
    - rationale: Centralizes coding-specific simplicity, canonical-pattern, and composition guidance here so the global baseline does not duplicate implementation rules.
- Language/framework-specific rules remain higher fidelity for their scopes.

## Checklist
- [ ] Coding guidance is canonical and does not duplicate workflow, testing, or security rules owned elsewhere
- [ ] Repository discovery and reuse guidance is present for existing models, types, helpers, components, modules, and canonical patterns
- [ ] One canonical-pattern rule is present for avoiding parallel implementations and re-export-based alternate access paths
- [ ] Anti-patterns include duplication, parallel patterns, internal re-exports, and over-abstraction
- [ ] Quality expectations are noted without duplicating narrower CI/testing rules
- [ ] Conventional commit guidance is restored for completed tasks without redefining response formatting locally
- [ ] Precedence and overrides are documented


## Changelog
- 1.12.0: Restored commit-message suggestion guidance for completed tasks while continuing to defer all formatting and placement rules to the explicit exception in the canonical global baseline.
- 1.11.0: Removed coding-owned response-format instructions and deferred conventional-commit output behavior to the explicit exception in the canonical global baseline.
- 1.10.0: Added explicit guidance to inspect the repository for reusable existing models, types, helpers, components, modules, and canonical patterns before introducing new ones.
- 1.5.0: Made re-export policy explicit in the canonical coding rules by prohibiting internal re-exports in any form and allowing only tightly coupled external public dependencies as a narrow exception.
- 1.4.0: Canonicalized the coding rules, removed overlap with the global baseline and other narrower global rules, and made the one-pattern-per-problem policy the central coding rule.
- 1.3.0: Prohibited ambiguous Unicode characters unless explicitly required and justified, and added reviewability/security guidance for visually confusable text.
- 1.2.0: Added canonical-pattern guidance to prefer one established way to achieve a goal and avoid parallel equivalent patterns unless explicitly requested or justified.
- 1.1.0: Streamlined rules for productivity, editor-first workflows, lean QA/anti-pattern guidance, and removed examples for brevity.
- 1.0.0: Initial version centralizing DRY, KISS, composition, SRP, and YAGNI with examples.
