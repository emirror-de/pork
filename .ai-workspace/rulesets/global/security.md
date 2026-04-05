
# AI Security & Privacy Rules (Global)

Model-agnostic rules to keep code, data, and credentials safe while collaborating with AI in any development environment. These rules apply to all files matched by this ruleset.

Conformance and terminology: The key words "MUST", "MUST NOT", "SHOULD", "SHOULD NOT", and "MAY" in this document are to be interpreted as described in RFC 2119 and RFC 8174 when, and only when, they appear in all capitals.

## Purpose

- Protect secrets, personal data, and sensitive repository content during AI-assisted work.
- Define one canonical security baseline for redaction, logging, external access, and incident handling.
- Keep security guidance reviewable, provider-agnostic, and consistent across the workspace.

## Scope

- Applies to all AI-assisted development work, including prompts, responses, logs, examples, tests, documentation, and generated code.
- Covers secrets handling, PII minimization, safe logging, external calls, compliance-sensitive data handling, and incident response guidance.
- Complements `global-global` by specializing security and privacy requirements. Where guidance overlaps, this file is the canonical source for security/privacy behavior.

## Rules

### Core Security Defaults
- MUST NOT include secrets or raw PII in prompts, responses, diffs, logs, examples, or generated artifacts.
- MUST minimize shared content and include only what is strictly necessary to solve the task.
- MUST redact sensitive content before sharing and SHOULD use deterministic placeholders such as "<REDACTED:API_KEY>".
- MUST obtain explicit consent before any external network/API call, upload, or third-party processing step.
- SHOULD use synthetic data in examples and MUST follow least-privilege design patterns.

## Definitions

- Secrets: API keys, tokens, passwords, private keys, client secrets, credentials, connection strings, session cookies, JWTs, cloud access keys, SSH keys, encryption keys.
- PII: Any data that can identify a person (name, email, phone, address, identifiers, IPs when tied to identity, device IDs). Treat "quasi-identifiers" as PII when combined (e.g., ZIP + birth date).
- Sensitive business data: Proprietary code, unreleased features, financial, legal, or security configurations.
- Safety-critical code: Authentication, authorization, cryptography, payment processing, personal/medical data handlers.

### Default Assistant Behavior
- Large logs, configs, or datasets MUST be summarized to the minimum necessary lines and structure.
- Assistants SHOULD use references over raw values (for example "uses environment variable DATABASE_PASSWORD" instead of the actual value).
- Redactions MUST use explicit typed placeholders such as:
  - "<REDACTED:API_KEY>"
  - "<REDACTED:JWT>"
  - "<REDACTED:ACCESS_TOKEN>"
  - "<REDACTED:EMAIL>"
  - "<REDACTED:PHONE>"
- Long sensitive-looking values MAY be truncated when context is necessary, for example "sk_live_…<TRUNCATED>".
- Environment variables, configuration contents, tokens, and credentials MUST NOT be echoed verbatim. If mention is necessary, include only names and safe placeholders.
- If uncertain whether content is sensitive, assistants MUST treat it as sensitive.

### Secrets & Credential Hygiene
- Real credentials MUST NOT be generated, embedded, or repeated; use placeholders and secure-storage guidance instead.
- Sample configuration files SHOULD contain placeholders only, and real configuration files SHOULD be ignored by version control where appropriate.
- Secrets MUST NOT appear in logs, errors, tests, screenshots, examples, or copied command output.
- If a secret appears in code, history, logs, or discussion, assistants SHOULD recommend rotation or revocation.
- Credentials MUST NOT be embedded in URLs or source code; environment variables or secret managers SHOULD be used instead.
- Token-like strings MUST be redacted before quoting or copying.

### PII Handling
- Data minimization SHOULD be the default: real names, emails, phone numbers, IDs, or other identifying data SHOULD NOT be included unless strictly necessary and explicitly allowed.
- Examples SHOULD use synthetic data such as "Jane Doe", "jane@example.com", or "+1-202-555-0100".
- Troubleshooting examples SHOULD pseudonymize identities, for example "User A" or "Order #1234".
- Raw datasets, CSVs, or logs containing PII MUST NOT be posted; use schemas, fake samples, or summaries instead.
- Analytics or telemetry examples SHOULD aggregate or hash identifiers rather than expose unique raw identifiers.

### Logging & Tracing
- Logs SHOULD be structured and minimal, for example event name, correlation/request ID, and high-level error category.
- Secrets, tokens, full headers, and entire request/response bodies MUST NOT be logged.
- Log sizes, fields, and arrays SHOULD be capped, and large or sensitive fields SHOULD be omitted, summarized, or hashed.
- Client-facing errors MUST remain safe and MUST NOT expose stack traces, credentials, or configuration details.
- Correlation IDs SHOULD be used for tracing instead of duplicating user or sensitive identifiers.

### External Calls, Uploads, and Tooling
- Explicit consent MUST be obtained before:
  - calling external APIs or services
  - uploading files or snippets to third-party endpoints
  - using remote execution or automated external fetchers
- Any proposed external call MUST specify:
  - endpoint or service
  - purpose and expected benefit
  - data categories involved
  - data minimization strategy
  - alternatives if external access is disallowed
- Organizational constraints such as proxies, allowlists, or approval workflows MUST be respected. If unknown, assistants MUST assume access is restricted.

### Compliance-Sensitive Guidance
- GDPR/CCPA-sensitive work SHOULD minimize personal data and avoid unnecessary retention or disclosure.
- HIPAA/PHI-sensitive data MUST be treated as highly restricted and MUST NOT be shared in prompts, outputs, or examples.
- PCI-DSS-sensitive data such as raw payment card information MUST NOT appear in logs, prompts, or code examples.
- SOC 2-sensitive workflows SHOULD use least privilege, auditing, change control, and controlled data egress.
- If a change could affect compliance-sensitive behavior, assistants SHOULD flag it and recommend review by appropriate stakeholders.

### Source Code & Licensing
- Large third-party code blocks SHOULD NOT be pasted when linking, summarizing, or citing is sufficient.
- Licenses MUST be respected, and incompatible-licensed code MUST NOT be incorporated into repositories where it would create legal or policy conflicts.
- License and vulnerability findings SHOULD be handled through controlled review processes rather than public disclosure.

### Data Classification Defaults
- Repository content MUST be treated as confidential by default.
- External sharing MUST be assumed prohibited unless explicitly approved.
- If data classification is unclear, assistants MUST proceed with the most restrictive reasonable assumption.

### Incident Handling
- If sensitive data may have been exposed, assistants MUST stop propagating it immediately.
- Exposure summaries SHOULD include only minimal details such as type, scope, and timing, without repeating the exposed content.
- Recommended next steps SHOULD include rotation, revocation, invalidation, or cache purging as appropriate.
- Assistants SHOULD suggest notifying the responsible security or privacy contact with only the necessary details.

## Quality Assurance
- Security/privacy guidance MUST remain consistent with `global-global` while serving as the canonical source for redaction, minimization, and external-access behavior.
- Examples SHOULD use synthetic or redacted data only.
- Proposed external access SHOULD always be reviewable in terms of purpose, scope, and minimization.
- Sensitive outputs SHOULD be checked for accidental credential, token, or PII leakage before being returned.

## Security Considerations
- Least privilege, minimization, and fail-closed behavior SHOULD be the default posture.
- Security-sensitive areas such as authentication, authorization, payments, health data, cryptography, and secret management require extra caution and narrower review.
- Safe error handling and safe logging are mandatory parts of the security posture, not optional documentation concerns.

## AI Assistant Interaction Guidelines
- Require redaction and minimization whenever logs, configs, datasets, credentials, or external calls are involved.
- Ask for explicit consent before any external sharing or remote lookup that could transmit repository or user data.
- Prefer summaries, schemas, and placeholders over raw sensitive content.
- If exposure is suspected, stop, minimize repetition, and recommend containment steps first.

## Precedence and Overrides
- Depends on: `global-global`.
- This file is the canonical source for workspace-wide security and privacy behavior.
- Narrower file- or domain-specific security rules MAY tighten these requirements for their own scopes.
- Overrides: none.

## Checklist
- [ ] No secrets, tokens, or credentials included
- [ ] No raw PII; synthetic or redacted data only
- [ ] Logs/configs minimized; only essential lines included
- [ ] Any proposed external call includes purpose, minimization, and consent request
- [ ] Error messages safe; no internal details leaked
- [ ] Test data uses fakes; no production samples
- [ ] Licensing respected; no large third-party code pasted
- [ ] Recommendations include secret rotation if exposure is suspected
- [ ] Compliance-sensitive areas flagged when relevant
- [ ] Least-privilege and data-minimization principles are followed

## Examples

Correct: safe redaction and minimized context
```
Log excerpt:
- request_id=req-123
- error_category=database_timeout
- auth_header=<REDACTED:ACCESS_TOKEN>
```

Correct: safe external-call consent request
```
Proposed external call:
- endpoint: https://example.invalid/metadata
- purpose: retrieve package metadata
- data shared: package name only
- minimization: no repository code, no secrets, no PII
- proceed only with explicit approval
```

Incorrect: raw secret and full request logging
```
Authorization: Bearer sk_live_1234567890
Request body: { full customer record ... }
```

## Anti-Patterns to Avoid
- ❌ Repeating secrets, tokens, credentials, or raw PII in responses, examples, or logs.
- ❌ Sharing more context than necessary, especially full logs, full configs, or full datasets.
- ❌ Assuming external calls are acceptable without explicit consent.
- ❌ Logging full headers, bodies, or sensitive identifiers when high-level summaries would suffice.
- ❌ Copying large third-party code or sensitive internal material into public or unnecessary contexts.

## Changelog
- 1.1.0: Canonicalized the global security rules, removed overlap with the global baseline, and clarified security/privacy ownership for minimization, redaction, external access, and incident handling.

---
By following these rules, assistants and contributors maintain strong security posture across models and workflows while enabling effective, privacy-preserving collaboration in any development environment regardless of the programming language or technology stack being used.
