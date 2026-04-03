
# Global Core Runtime Rules

## Purpose
- Define the smallest always-on rule set that must remain salient throughout the session.
- Establish `.ai-workspace` as the canonical source of truth for project rules and task state.
- Require reloading applicable project rules and active checklist state before meaningful work.

## Scope
- Applies to all work in the target project.
- Governs startup behavior, rule discovery, rule reloading, active checklist handling, and core safety defaults.
- Defers detailed policy to rules loaded from `.ai-workspace/rulesets/` after discovery through `.ai-workspace/rulesets/index.json`.

## Rules
- The repository root `.ai-workspace` directory is the canonical source of truth for project-specific rules, task state, and AI-workspace artifacts.
- The startup `.rules` file is only the compact runtime core and MUST NOT be treated as the full project ruleset.
- Before planning, editing, validating, or making task-state decisions, assistants MUST reload the active checklist state and use `.ai-workspace/rulesets/index.json` to discover the applicable rulesets instead of relying on earlier conversational memory alone.
- On the first prompt of a session, assistants MUST compare the current prompt context against the active checklist context and available non-archived checklist contexts before continuing meaningful work.
- Applicable rulesets MUST be selected by the affected files, task scope, and current checklist context using the metadata in `.ai-workspace/rulesets/index.json`; assistants MUST NOT reload unrelated rulesets unless they become relevant.
- After identifying applicable rulesets in `.ai-workspace/rulesets/index.json`, assistants MUST read the corresponding ruleset markdown files from `.ai-workspace/rulesets/` before meaningful work.
- If multiple applicable rulesets conflict, narrower scope wins; if scope ties, the higher-order rule wins; if still tied, prefer safety, privacy, and explicit user instructions over convenience.
- For non-trivial work, assistants MUST use and maintain the active checklist as the canonical execution state.
- The active checklist pointer file in `.ai-workspace/checklists/active` MUST be treated as a plain text file whose content is the relative path to the active checklist folder.
- If an active checklist pointer exists, assistants MUST read the pointed checklist before meaningful work and MUST update it when progress, status, assumptions, validation results, or artifacts change.
- If the active checklist context does not match the current prompt context on the first prompt, assistants MUST check other available non-archived checklists for a matching context.
- If a matching non-archived checklist already exists, assistants MUST mark that checklist as active and use it as the canonical execution state.
- If no matching non-archived checklist exists, assistants MUST create a new checklist, set it as active, and use that new checklist as the canonical execution state.
- Archived checklists under `.ai-workspace/checklists/archive` MUST be treated as historical state and MUST NOT be modified unless the user explicitly asks for it.
- Assistants MUST keep changes minimal, targeted, and in scope.
- For non-trivial changes, assistants MUST add or update tests unless that is impractical in the current step; if impractical, assistants SHOULD state why and propose the follow-up.
- Assistants MUST provide terminating validation commands for meaningful changes.
- Assistants MUST NOT include secrets, credentials, tokens, or raw personal data in outputs, examples, logs, or artifacts.
- Assistants MUST obtain explicit user consent before any external network call or third-party data sharing.
- Before finalizing a meaningful response, assistants MUST perform a compliance pass against the active applicable rules and revise the response if a mandatory rule is violated.

## Examples
Correct:
- Read `.ai-workspace/checklists/active`, load the referenced checklist, inspect `.ai-workspace/rulesets/index.json`, then load only the ruleset files relevant to the files being changed.

Correct:
- Use the compact `.rules` file as startup guidance, but rely on `.ai-workspace/rulesets/index.json` for ruleset discovery and `.ai-workspace/rulesets/` markdown files for detailed project policy.

Incorrect:
- Treat the startup `.rules` file as the only active rules source for the whole session.

Incorrect:
- Edit files after a new user request without reloading the active checklist and applicable rulesets.

## Anti-Patterns to Avoid
- ❌ Re-reading all project rules for every prompt regardless of scope.
- ❌ Skipping `.ai-workspace/rulesets/index.json` and guessing applicable rules from memory alone.
- ❌ Relying on stale chat memory when `.ai-workspace` contains newer canonical state.
- ❌ Updating implementation state without updating the active checklist for non-trivial work.
- ❌ Treating archived checklist folders as active workspaces.
- ❌ Letting detailed policy drift into the compact startup core.

## Quality Assurance
- The core rules SHOULD remain short, durable, and operational.
- Ruleset metadata and discovery state SHOULD live in `.ai-workspace/rulesets/index.json`.
- Detailed policy, examples, and scoped behavior SHOULD live in `.ai-workspace/rulesets/` markdown files, not in this file.
- Checklist updates SHOULD reflect the real canonical state of current work.

## Security Considerations
- Treat `.ai-workspace` content as confidential project state by default.
- Do not expose secrets or raw personal data from checklist artifacts, rulesets, or project files.
- Keep external access consent-gated and minimal.

## AI Assistant Interaction Guidelines
- Start from the compact core, then reload the active checklist, inspect `.ai-workspace/rulesets/index.json`, and read the applicable ruleset markdown files before meaningful work.
- Prefer filesystem-backed canonical state over conversational assumptions.
- Use the checklist to track progress for non-trivial tasks and keep it current as work advances.

## Precedence and Overrides
- Depends on: none.
- This file is the startup runtime core only.
- Detailed project behavior is delegated to `.ai-workspace/rulesets/` and active checklist state.

## Checklist
- [ ] `.ai-workspace` is treated as the canonical source of truth
- [ ] Active checklist pointer file is read before meaningful work
- [ ] First-prompt context is checked against the active checklist context
- [ ] `.ai-workspace/rulesets/index.json` is used to discover applicable rulesets
- [ ] Applicable ruleset markdown files are reloaded by scope
- [ ] Non-trivial work updates active checklist state
- [ ] Validation commands are provided for meaningful changes
- [ ] No secrets or raw personal data are exposed

## Changelog
- 1.0.0: Initial compact core runtime rules for `.ai-workspace`-based rule loading and checklist-driven task state.
