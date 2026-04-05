
# Provider-Agnostic Model Guidance

## Purpose
- Ensure AI assistance remains interchangeable across providers and models.
- Define provider-neutral constraints that remain after the global baseline is applied.
- Keep outputs deterministic, reviewable, and free of provider-specific assumptions.

## Scope
- Applies to all AI-assisted development work in this workspace.
- Focuses on provider-neutral interaction constraints, determinism, and reviewability.
- Does not restate the global baseline for privacy, testing, general workflow, or output-format requirements except where provider-neutral clarification is required.

Conformance and terminology: The key words "MUST", "MUST NOT", "REQUIRED", "SHALL", "SHALL NOT", "SHOULD", "SHOULD NOT", "RECOMMENDED", "NOT RECOMMENDED", "MAY", and "OPTIONAL" in this document are to be interpreted as described in RFC 2119 and RFC 8174 when, and only when, they appear in all capitals.

## Rules

### Provider Neutrality
- Guidance MUST remain provider-agnostic and MUST NOT require provider-specific prompt syntax, tokens, branded modes, UI features, or proprietary tooling.
- Outputs MUST be understandable and usable without relying on model-specific behavior.
- Requirements SHOULD be expressed in terms of observable outcomes and constraints, not vendor-specific capabilities.

### Determinism and Reviewability
- Outputs SHOULD be deterministic and stable: consistent headings, ordering, terminology, and response structure.
- Assistants MUST prefer small, reviewable increments over large monolithic rewrites.
- When provider-related ambiguity remains after clarification, assistants SHOULD state conservative assumptions explicitly so reviewers can validate them.
- Guidance MUST remain auditable without requiring hidden reasoning, internal provider features, or undocumented system behavior.
### Workflow Boundaries
- Provider-neutral guidance SHOULD preserve the shared workspace workflow defined by the global baseline instead of introducing an alternative workflow here.
- This file MUST NOT duplicate generic privacy, testing, validation, workflow, or output-format rules that already belong to narrower or broader global rules unless a provider-neutral clarification is necessary.
- When a requirement is already canonical in another global rule, this file SHOULD defer to that rule instead of restating it.

## Examples

Correct: conservative, deterministic plan with validation
```
Objective: Fix pagination bug without changing API shape.
Plan:
1) Add a regression test covering "page=0" and "page too large" (error path).
2) Implement input validation and clamp max page size.
3) Update docs/comments if behavior changes.
Validate:
- format: <format command>
- lint: <lint command>
- test: <test command>
Assumptions:
- Max page size is 100 unless configured otherwise.
```

Incorrect: provider-dependent instruction and missing validation
```
Use the model's special feature to generate a full repo patch.
No tests needed; just change it quickly.
We'll verify later.
```

## Anti-Patterns to Avoid
- ❌ Relying on provider-specific tokens, branded features, UI affordances, or hidden reasoning modes to make the guidance work.
- ❌ Defining a second generic workflow here when the canonical workflow already belongs to the global baseline.
- ❌ Vague “should work” statements without specifying assumptions, constraints, or reviewable expectations.
- ❌ Large, monolithic outputs that are hard to audit or split into incremental review steps.
- ❌ Provider-neutral guidance that implicitly assumes unrestricted external access or proprietary platform behavior.

## Quality Assurance
- Guidance MUST remain model-agnostic and consistent with `global-global`.
- This file SHOULD only contain provider-neutral requirements that are not already more canonically defined elsewhere in `src/global`.
- Examples SHOULD be self-contained and avoid requiring external services unless explicitly scoped and consented.
- Reviewers SHOULD reject additions that duplicate generic safety, testing, or workflow rules already owned by another global file.

## Security Considerations
- Provider neutrality MUST NOT weaken the canonical security and privacy requirements defined in `global-security` and `global-global`.
- When provider choice affects data sharing or external access, guidance MUST remain conservative and require explicit consent rather than assuming equivalent data handling across providers.
- Examples SHOULD use deterministic placeholders when sensitive-looking values are necessary.

## AI Assistant Interaction Guidelines
- When requesting help, specify:
  - Objective and constraints (compatibility, performance, risk tolerance)
  - Scope (files/modules) and review expectations
  - Whether external access is allowed (default: no)
- When reviewing outputs, require:
  - Provider-neutral language and assumptions
  - A deterministic, reviewable structure
  - Clear assumptions where provider-related ambiguity remains
  - No provider-specific tokens, branded modes, or proprietary workflow assumptions
- If a request depends on provider-specific behavior, assistants SHOULD restate it as the underlying capability or constraint wherever possible.

## Precedence and Overrides
- Depends on: `global-global`.
- Overrides: none.

## Checklist
- [ ] Provider-neutral language only; no model/vendor-specific features or branded behavior
- [ ] Deterministic, reviewable structure and terminology
- [ ] No duplicate generic workflow, output-format, safety, or testing rules already owned by another global file
- [ ] Assumptions are explicit when ambiguity remains
- [ ] Security-sensitive implications of provider choice stay conservative
- [ ] Guidance remains usable without hidden provider-specific behavior

## Changelog
- 2.2.0: Removed remaining overlap with global workflow and output-format guidance so this file stays focused on provider-neutral constraints and reviewability.
- 2.1.0: Canonicalized the provider guidance by removing overlap with the global baseline, tightening provider-neutral constraints, and clarifying that this file defers generic workflow, safety, and testing rules to their canonical global sources.
- 2.0.0: Standardized structure; strengthened determinism, safety, and validation requirements.
- 1.0.0: Initial provider-agnostic workflow guidance.
