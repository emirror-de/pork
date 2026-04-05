
# Global Planning & Checklist Rules for AI-Assisted Development

## Purpose
- Define one canonical, machine-readable standard for checklist and plan state.
- Ensure agents and humans can work top-to-bottom from the same structured source of truth.
- Keep planning artifacts auditable, mergeable, and validation-oriented without embedding secrets.

## Scope
- Applies to checklist/plan Markdown files referenced by the repository globs (examples: `checklists/**/*.md`, files ending in `.checklist.md`).
- Covers checklist schema, canonical state handling, session history, item lifecycle, validation hints, and the lightweight plan template.
- Excludes repository-level operational workflows (for example: git branching/PR rules) and does not restate the global collaboration workflow already owned by `global-global`.

## Rules
- File structure:
  - Each checklist file MUST include a YAML front matter block at the top. The front matter is the canonical state and agents MUST read/write it directly.
  - Front matter SHOULD include at minimum: `id`, `title`, `goal`, `owner`, `created_at`, `updated_at`, `version`, `status`, `items`, `sessions`, `validate`, and `lock_hint`.
  - RFC3339 timestamps MUST be used for `created_at`, `updated_at`, `started_at`, `completed_at`, and session timestamps.
- Canonical state:
  - `items[]` is authoritative. Each item MUST have at least: `id`, `title`, `description`, `order`, `owner`, and `status`.
  - Per-item `status` enum: `todo`, `in_progress`, `done`, `blocked`.
  - Document-level `status` enum: `todo`, `in_progress`, `done`, `blocked`, `archived`.
- Session handling:
  - `sessions` is append-only. Agents MUST append entries recording `session_id`, `agent`, `started_at`, `ended_at` (null while active), `changes`, and an optional `note`.
  - Agents MUST NOT overwrite or remove other `sessions` entries.
- Source-of-truth edits:
  - Agents SHOULD update only YAML front matter for machine-visible state changes such as status, timestamps, artifacts, and sessions.
  - The Markdown body is human-facing only. It is not canonical state and MUST NOT be treated as authoritative by agents.
- Item lifecycle:
  - When starting work on an item, an agent MUST:
    - append or extend a `sessions` entry with `started_at`
    - update the item `status` to `in_progress`
    - set the item `started_at` if it is still null
  - When completing an item, an agent MUST:
    - update the item `status` to `done`
    - set `completed_at`
    - append artifact references such as a commit SHA, PR number, or test report
    - record the change in the active `sessions` entry and set `ended_at` if the session finishes
- Validation:
  - Each item SHOULD include `validate_cmds` with short, single-shot commands that verify the work.
  - The file-level `validate` array contains document-wide checks such as format/lint/test commands.
  - Agents SHOULD run relevant `validate_cmds` before marking an item `done` and record the outcome in `artifacts` or the session `note`.
- Concurrency and locking:
  - `lock_hint` MAY be used as an advisory coordination field, for example a session id.
  - Agents MUST remain resilient to concurrent edits even when `lock_hint` is present.
- Change history and versioning:
  - `version` SHOULD increase for non-trivial state changes such as major item transitions.
  - `updated_at` MUST be updated whenever front matter changes.
- Security & privacy:
  - Secrets, credentials, and raw PII MUST NOT be stored in checklist files.
  - Artifacts MAY reference PRs/commits by id or URL, but MUST NOT include tokens or credentials.

## Examples

Correct checklist template (agents MUST render/update the YAML front matter; human body is optional):
```
---
id: checklist-example-001
title: "Short descriptive title"
goal: "One-line goal statement describing the success condition"
owner: "team/name-or-person"
created_at: "2026-01-01T00:00:00Z"
updated_at: "2026-01-01T00:00:00Z"
version: 1
status: todo

items:
  - id: i01
    title: "Small, testable task"
    description: "Clear description and success criteria"
    order: 1
    estimate_hours: 2
    owner: "alice"
    status: todo            # todo | in_progress | done | blocked
    started_at: null
    completed_at: null
    artifacts: []
    tests: []
    validate_cmds: ["pytest -q tests/test_small_task.py"]

  - id: i02
    title: "Next small task"
    description: "Another clear step"
    order: 2
    estimate_hours: 3
    owner: "bob"
    status: todo
    started_at: null
    completed_at: null
    artifacts: []
    tests: []
    validate_cmds: ["pytest -q tests/test_next_task.py"]

sessions: []
validate:
  - "make format"
  - "make lint"
lock_hint: null
---

# Title: Short descriptive title

Goal: One-line goal statement describing the success condition.

Instructions:
- Work items top-to-bottom by `order`.
- The YAML `items[]` array is the canonical state.
- Append to `sessions` when starting/ending a session.

Human checklist (rendered):
1. [ ] Small, testable task — owner: alice — estimate: 2h
2. [ ] Next small task — owner: bob — estimate: 3h
```

Correct plan template — a lightweight, agent-friendly planning structure to attach to a session or front matter:
```
plan:
  summary: "One-line summary of the plan"
  assumptions:
    - "Assumption 1"
    - "Assumption 2"
  steps:
    - step_id: p01
      item_id: i01         # link to canonical item.id when applicable
      description: "Concrete action to take"
      expected_outcome: "What must be true after this step"
      validate_cmds: ["pytest -q tests/test_small_task.py"]
      estimate_hours: 2
  risks:
    - "Short description of risk and mitigation"
  exit_criteria:
    - "All item statuses are done"
```

Incorrect (anti-pattern) — storing secrets or making the body the canonical state:
```
---
# ❌ BAD: DO NOT do this
# - Avoid storing API keys or credentials here
# - Avoid using the rendered checkboxes as authoritative state for agents
---
```

## Anti-Patterns to Avoid
- Using rendered Markdown checkboxes as the authoritative state instead of YAML front matter.
- Storing secrets, tokens, credentials, or raw PII inside the checklist file.
- Large, monolithic items that take days when they could be broken into smaller validated steps.
- Overwriting other agents' `sessions` entries or removing session history.
- Embedding long logs or binary blobs in `artifacts` instead of referencing them by id/URL.
- Duplicating general workflow, safety, or testing rules here when they already belong to more canonical global files.

## Quality Assurance
- Canonical state MUST remain parseable and machine-readable after every edit.
- For non-trivial items, at least one automated test or validation command SHOULD appear in `validate_cmds`.
- Validation commands SHOULD be deterministic, fast, and suitable for automation.
- YAML front matter SHOULD be validated on every change, including required keys and timestamp formats.
- Human reviewers SHOULD verify that referenced `artifacts` exist and that no secrets or raw PII were introduced.

## Security Considerations
- Checklist files are treated as confidential by default; assume restricted sharing.
- Use pseudonymous or synthetic data in examples and tests.
- When referencing external/private artifacts, include only minimal references (PR number, commit SHA), not credentials or tokens.
- If a secret or sensitive identifier is discovered in a checklist, follow incident guidance: redact, rotate, and notify owners.

## AI Assistant Interaction Guidelines
- Agents MUST parse YAML front matter first and treat it as the only canonical state.
- If a checklist requirement is unclear, agents SHOULD ask one focused clarifying question before making a state transition.
- To start work, an agent SHOULD:
  - create a `session_id` such as `sess-YYYYMMDD-<short-rand>`
  - append a `sessions` entry with `session_id`, `agent`, `started_at`, planned `changes`, and optional `note`
  - update targeted items to `in_progress` and set `started_at` if unset
- To finish work, an agent SHOULD:
  - run relevant `validate_cmds`
  - update item status to `done`
  - set `completed_at`
  - add `artifacts`
  - append the change to the active session and set `ended_at` when appropriate
- Agents MUST NOT edit or remove other session records.
- If a merge-conflict-like situation is detected, agents SHOULD record it in `sessions` and set the item to `blocked` until resolved.
- Progress summaries SHOULD report canonical front matter state first, with any rendered checklist treated as a convenience view only.

## Precedence and Overrides
- Depends on: `global-global`.
- This file is the canonical source for checklist-file state, session history, and machine-readable planning conventions.
- Use specific globs to reduce accidental application outside intended checklist files.
- If a narrower file-level rule conflicts with this one, the narrower-scope rule wins per the global precedence policy.

## Checklist (Quick Reference)
- [ ] File has YAML front matter with required keys
- [ ] YAML front matter, not rendered Markdown, is treated as canonical state
- [ ] `items[]` array is present and ordered
- [ ] `sessions` exists and is treated as append-only
- [ ] Each item has `validate_cmds` where applicable
- [ ] No secrets, credentials, or raw PII appear in the file
- [ ] `updated_at` and `version` are updated on non-trivial state changes
- [ ] `lock_hint` is used as advisory only
- [ ] Document-level `validate` checks are present
- [ ] The file does not duplicate broader workflow/security/testing rules owned elsewhere

## Changelog
- 1.1.0: Canonicalized the planning rules by tightening ownership around checklist state, session history, and validation metadata while removing residual overlap with the global baseline.
- 1.0.0: Initial ruleset and templates for machine-readable, session-aware checklists and agent planning.
