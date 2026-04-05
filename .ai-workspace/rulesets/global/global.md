
# Global AI Rules for This Workspace

These rules are model-agnostic and apply equally across different AI providers in any development environment. They define how to structure assistance, keep changes safe, and maximize developer productivity across any programming language or framework.

Conformance and terminology: The key words "MUST", "MUST NOT", "REQUIRED", "SHALL", "SHALL NOT", "SHOULD", "SHOULD NOT", "RECOMMENDED", "NOT RECOMMENDED", "MAY", and "OPTIONAL" in this document are to be interpreted as described in RFC 2119 and RFC 8174 when, and only when, they appear in all capitals. Imperative statements without an explicit strength are normative and treated as "MUST" unless explicitly marked otherwise. Legacy phrasing maps as follows for interpretation: "prefer" → "SHOULD", "avoid" → "SHOULD NOT", "never" → "MUST NOT".

## Purpose
- Provide consistent, high-quality assistance across languages and stacks in this workspace.
- Keep changes safe, auditable, and reversible.
- Remain interchangeable between models without provider-specific tokens or features.

## Response Style
- Active rules from this workspace MUST be treated as binding instructions and MUST be followed before any lower-priority preference or convenience goal.
- Assistants MUST assume active rules remain in force unless the user explicitly overrides them in the prompt.
- If a prompt explicitly overrides an active rule, assistants MUST follow the prompt for that request only and SHOULD continue following all non-overridden rules.
- Assistants MUST be concise by default; they MAY expand when clarification or safety warrants it.
- Backward compatibility is NOT REQUIRED unless explicitly requested by the user; default to the current target environment and simplify accordingly.
- Assistants SHOULD use stepwise, incremental changes. For large tasks, assistants MAY propose a plan first, then implement in small, reviewable steps.
- After the user has approved a plan, direction, or implementation path, assistants SHOULD continue along that path without repeatedly asking for confirmation on each obvious next step.
- Assistants SHOULD ask for renewed confirmation only when the proposed next step would materially change scope, introduce a meaningful new risk or trade-off, require a user decision between real alternatives, or conflict with prior user guidance.
- When a request is ambiguous, assistants MUST ask focused clarifying questions, then MAY proceed with a safe assumption if needed.

## Safety & Privacy
- Assistants MUST follow the dedicated global security rules for secrets, personal data, redaction, and external network access.
- Assistants SHOULD minimize shared context and include only the minimum required excerpts.
- Assistants SHOULD treat user content as potentially sensitive unless a narrower rule says otherwise.

## Output Constraints and Chunking
- If an answer risks exceeding model/editor limits:
  - Assistants MUST propose a plan and split into numbered steps.
  - Assistants SHOULD deliver the first step, await confirmation, then continue.
- For generated code, each step SHOULD compile/test independently where feasible.
- Responses MUST NOT include code snippets, code blocks or diffs, except for one optional conventional commit message block allowed by the explicit exception in `## Change Size & Diffs`.

## Tests-First Policy
- For any non-trivial change, contributors MUST add or update tests alongside code.
- If adding tests is impractical in the current step, contributors MAY explain why and SHOULD propose a follow-up test task.
- Test strategy, coverage expectations, and CI-specific validation rules belong in narrower testing or CI rules.

## Ambiguity Handling
- Assistants MUST ask targeted questions when requirements are unclear.
- Assistants MUST NOT ask repetitive permission-seeking questions when the user has already agreed on the current direction and the next step is the natural continuation of that work.
- If time-critical, assistants SHOULD state assumptions explicitly and SHOULD choose conservative defaults.
- Assistants SHOULD document trade-offs and implications of chosen defaults.

## Conflict Resolution
- If two rules conflict, the narrower scope wins (language/framework-specific over global).
- When scope is equal, the higher-order rule wins.
- If still tied, prefer security/privacy/safety over convenience.

## Request Pattern (Repeatable Loop)
0) Re-load: before Analyze, assistants MUST read the active checklist pointer at `.ai-workspace/checklists/active`, load the referenced checklist, inspect `.ai-workspace/rulesets/index.json`, and re-read the ruleset markdown files applicable to the current task scope and affected files; assistants MUST NOT rely on earlier conversational memory of rules or checklist state alone.
   - On the first prompt of a session, assistants MUST compare the current prompt context against the active checklist context before treating that checklist as canonical for the session.
   - If the active checklist matches the current prompt context, assistants SHOULD continue using it.
   - If the active checklist does not match the current prompt context, assistants MUST check other available non-archived checklists for a matching context and SHOULD mark a matching checklist as active when one exists.
   - If no non-archived checklist matches the current prompt context, assistants MUST create a new checklist for the new context, set it as the active checklist, and then use that new checklist as the canonical execution state.
1) Analyze: assistants MAY restate the objective, constraints, and risks.
2) Propose: assistants SHOULD outline a short plan (steps, files, tests).
3) Implement: assistants SHOULD apply changes via the editor/tools with complete code/context; responses MUST NOT include code snippets, code blocks or diffs except for the optional conventional commit message block allowed by the explicit exception in `## Change Size & Diffs`.
4) Validate: assistants SHOULD note build/test commands to run and expected outcomes.
5) Iterate: assistants SHOULD await feedback before proceeding to the next non-obvious step, but SHOULD continue through obvious in-scope follow-up steps that are part of the already agreed path.

In agent final summaries, the "Why this change was necessary" section is OPTIONAL and SHOULD be included only when it adds clear value for the user.

## Change Size & Diffs
- Contributors MUST default to the smallest viable change that solves the problem without taking shortcuts that do not provide long term benefits.
- Contributors MUST perform refactors in isolation from feature changes and SHOULD sequence them if both are required.
- Contributors MUST NOT rewrite entire files when making localized changes; editors MUST update only the corresponding lines or minimal ranges necessary to implement the change. Full-file overwrites are allowed only when creating a new file or when explicitly justified and documented.
- Exception: when a task is complete and the response has no open questions, unresolved problems, blocked work, or pending decisions, assistants SHOULD include exactly one suggested conventional commit message in a fenced code block at the very end of the response when it adds practical value.
- If included, that conventional commit message block MUST contain only the commit text, MUST appear on its own line at the very end of the response, and MUST be the only allowed code block exception to the general no-code-block rule.
- Assistants MUST NOT include the conventional commit message block when the response is primarily asking clarifying questions, reporting unresolved problems, or requesting decisions needed to proceed.

## Code Quality Defaults
- Contributors MUST include all necessary imports and module declarations.
- Contributors SHOULD use explicit types for public APIs and complex expressions.
- Contributors MUST handle errors explicitly and MUST NOT use unsafe operations or suppress errors in production code.
- Contributors SHOULD add documentation comments for public APIs, constraints, and examples.
- Contributors SHOULD follow language-idiomatic style and recommended linters/formatters.
- Simplicity, canonical patterns, composition, and ambiguous-character restrictions belong in narrower coding rules.

## Security Expectations
- Systems MUST validate inputs at system boundaries and MUST sanitize before use or display.
- Systems MUST map internal errors to safe external responses and MUST NOT leak stack traces or internals.
- Systems SHOULD apply timeouts, rate limits, and backpressure where appropriate.
- Detailed credential, secret-handling, compliance, and external-call rules belong in the dedicated global security rules.

## Observability & Logging
- Systems SHOULD use structured logging and correlation/request IDs.
- Systems MUST log at appropriate levels and MUST NOT log sensitive data.
- Contributors SHOULD suggest health checks and basic metrics for new services or endpoints.

## Documentation & Comments
- Documentation MUST focus on developers using the corresponding code, not implementers or maintainers.
- Documentation MUST NOT explain differences to previous versions or internals that developers using the corresponding code not need to know about.
- Documentation MUST NOT replace removed code by comments telling that code has been removed.
- Documentation MUST be short, simple, and informative — prioritize clarity over comprehensiveness.
- Authors SHOULD explain internal relationships where it adds value to understanding the big picture of corresponding code and SHOULD call out assumptions, trade-offs, and limitations that affect usage.
- Authors SHOULD include practical, short usage examples for public functions/components showing common use cases.
- Authors MUST document parameters, return values, and error conditions that callers need to handle.
- Authors MUST update or create README snippets when adding meaningful features or workflows.
- Authors MUST update stale references immediately when they are found and MUST NOT pause to ask for permission or inquiry before correcting them.
- Authors MUST NOT include implementation details unless they directly impact how the code should be used.
- Comments and inline notes MUST be meaningful and avoid noise:
  - Comments SHOULD be omitted in favor of structured logging and only be added where really adding value to the reader.
  - Comments MUST NOT be used to announce changes, new features, deletions, or other VCS-style provenance notes (for example: "// new feature", "// changed", "// removed", "/* deleted */"). These are unnecessary noise and duplicate version control history; they MUST NOT appear in source files.
  - Comments MUST be concise and focused on explaining intent, rationale, constraints, or non-obvious behavior. Avoid placeholder comments, vague markers, or ephemeral debug notes.
  - If a TODO/FIXME is required, include a short actionable description and, where possible, reference an issue or PR (e.g., "TODO: replace X with Y — see issue #123"). Bare TODOs or unsourced remediation comments SHOULD be avoided.
  - Unnecessary noise in code (including redundant or commit-like comments, commented-out code without a clear reason, or transient debug artifacts) MUST NOT be produced; prefer clear names, small functions, tests, and version control history to capture provenance.
- When refactoring or moving code, authors MUST NOT replace the original code with a placeholder comment at the original location; version control and history SHOULD be used to track moves and provenance instead of leaving comment placeholders.

## IDE Integration Notes
- Contributors SHOULD optimize for navigation: consistent naming, clear module structures, and meaningful symbols for go-to-definition/find references.
- Contributors SHOULD keep imports ordered and deduplicated.
- When suggesting commands or scripts, contributors SHOULD use portable, single-shot commands (no long-running watchers).
- Contributors MUST respect file scoping: adhere to globs from specific ruleset files and keep unrelated changes out of scope.

## External Services & Commands
- Contributors SHOULD propose commands that terminate on their own (format, lint, test, build).
- For integrations (APIs, DBs), contributors MUST list required env vars/config and provide safe placeholders.
- Contributors MAY suggest migration/rollback steps where schema or contract changes occur.
- Provider-neutral interaction rules and external-call consent handling belong in narrower provider and security rules.

## Anti-Patterns to Avoid
- Large, monolithic responses without a plan (MUST NOT).
- Provider-specific prompt tokens or tool calls (MUST NOT).
- Copying raw secrets, full logs, or large binary/config blobs (MUST NOT).
- Using ambiguous Unicode characters that can be confused with other characters in code, identifiers, documentation, or configuration (MUST NOT).
- Mixing refactors with feature changes without a clear sequence (MUST NOT).
- Omitting tests for non-trivial changes (MUST NOT).

## Review Checklist (Quick)
- [ ] Small, incremental change with a clear plan
- [ ] Tests added/updated (success + failure paths)
- [ ] Explicit error handling; no unsafe operations in production code
- [ ] No ambiguous Unicode characters introduced where they could be confused with other characters
- [ ] Security/privacy respected; no secrets leaked
- [ ] Small, targeted edits applied via tools; responses MUST NOT include code snippets (and no diffs) except for the optional conventional commit message block allowed by the explicit exception above
- [ ] Observability notes (logging/metrics) when relevant
- [ ] Commands to validate (format, lint, test, build) are provided

---
These global rules ensure consistent, safe, and efficient collaboration with AI across this workspace, while remaining fully interchangeable across AI providers and models.
