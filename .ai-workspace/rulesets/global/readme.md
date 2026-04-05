
# README Best Practices for AI-Assisted Development

Note on normative language and conformance: The key words "MUST", "MUST NOT", "REQUIRED", "SHALL", "SHALL NOT", "SHOULD", "SHOULD NOT", "RECOMMENDED", "NOT RECOMMENDED", "MAY", and "OPTIONAL" in this document are to be interpreted as described in RFC 2119 and RFC 8174 when, and only when, they appear in all capitals.

## Purpose
Define canonical standards for creating accurate, user-friendly README.md files that communicate project purpose, setup, usage, and contribution guidance clearly to developers and users.

## Scope
- Applies only to `README.md` files matched by `globs`.
- Governs README structure, accuracy, runnable examples, and maintenance expectations.
- Specializes documentation guidance for README files; broader documentation/comment rules remain in `global-global`.
- Does not override language/framework-specific guidance about code style; it focuses on documentation usability and correctness.

## Rules

### Repository Conventions for Code Blocks
- Code blocks in repository rule files MUST follow the repository’s rule-file code block convention.
- In `README.md` files, code blocks SHOULD use normal language-tagged Markdown fences unless repository rendering rules explicitly require something else.
- README code blocks MUST optimize for end-user readability and copy/paste usability.

## Essential README Structure

### Project Header
- MUST start with a clear, descriptive H1 project title.
- MUST include a concise one-line description immediately after the title.
- SHOULD add a short paragraph explaining the project purpose and main goals.
- MAY place badges (build status, version, license) after the description.

### Installation
- MUST provide prerequisites with version requirements where relevant.
- MUST include step-by-step installation instructions.
- MUST show the actual commands users should run.
- SHOULD include platform-specific guidance only where it changes the steps materially.
- Installation commands MUST be tested before inclusion.

### Usage
- MUST include a basic usage example that a new user can run immediately.
- MUST provide complete, working examples with correct syntax highlighting.
- SHOULD show expected output where it materially improves understanding.
- SHOULD include the most common workflows or use cases before less common ones.
- Configuration options SHOULD be documented only when they matter to normal usage.

### Documentation Structure
- MUST use a consistent heading hierarchy (H1 for title, H2 for major sections, H3 for subsections).
- SHOULD include a table of contents for READMEs longer than 200 lines.
- SHOULD link to external documentation when available.
- Libraries SHOULD provide an API reference or quick reference section when that helps users navigate the project.

## Code Example Standards

### Code Block Requirements
- MUST use proper language tags for syntax highlighting.
- MUST provide complete, runnable examples that work out of the box.
- MUST include necessary imports, setup code, and context.
- SHOULD use realistic data and scenarios rather than vague placeholder text.
- All code examples MUST be tested before inclusion.

### Example Code Patterns

Correct (command example):
```
npm install package-name --save
npm run build
```

Correct (code example with imports and realistic usage):
```
const express = require('express');
const app = express();

app.get('/api/users', (req, res) => {
  res.json({ users: ['alice', 'bob'] });
});

app.listen(3000, () => {
  console.log('Server running on port:', 3000);
});
```

### Anti-Patterns to Avoid
- ❌ Incomplete code snippets without imports or execution context.
- ❌ Placeholder text like "your-api-key-here" without explanation.
- ❌ Commands that do not actually work as written.
- ❌ Examples using deprecated or outdated syntax.

## Project Type Specific Guidance

### Library/Package Documentation
- Emphasize installation and basic usage.
- Include API documentation or links to it.
- Show integration examples with common frameworks where relevant.
- Include troubleshooting for common usage problems when needed.

### Application Documentation
- Include screenshots or demo links when they materially help evaluation.
- Provide deployment and configuration instructions.
- Document required environment variables and runtime settings.
- Show realistic usage scenarios or user workflows.
- Include system requirements and dependencies where relevant.

### CLI Tool Documentation
- Show command examples with actual output when helpful.
- Include common workflows and usage scenarios.
- Provide installation verification commands.
- Include troubleshooting for common CLI issues when relevant.

## Contributing and Community Sections

### Contributing Guidance
- SHOULD explain how to report bugs.
- SHOULD provide development environment setup instructions when contributions are expected.
- SHOULD document coding standards, pull request expectations, and how to run tests.
- MUST keep contributing instructions aligned with the actual repository workflow.

### Community Information
- MUST include license information with a link to the full text.
- SHOULD add a code-of-conduct reference for open source projects.
- MAY credit contributors and acknowledgments where appropriate.
- SHOULD provide contact, community, or support links when relevant.
- SHOULD include a security policy reference if applicable.

## Quality Assurance

### Content Verification
- Installation instructions MUST be tested in a clean or representative environment.
- Code examples MUST compile or run correctly as documented.
- Links SHOULD be checked for accuracy and currentness.
- Formatting and terminology SHOULD remain consistent throughout the README.
- Examples MUST be updated when dependencies, commands, or APIs change.

### Maintenance
- READMEs MUST stay aligned with the actual project behavior.
- Version numbers, compatibility notes, and command names MUST remain current.
- Screenshots and demo links SHOULD be refreshed when they become stale.
- Contributing guidance SHOULD be reviewed periodically if the repository accepts contributions.

## AI Assistant Interaction Guidelines

### Effective README Requests
When requesting README assistance:
- Specify project type (library, application, CLI tool, etc.).
- Mention the target audience.
- Include the existing project structure and key files.
- State any special documentation requirements or constraints.

### Iterative README Development
- Start with essential sections first.
- Expand examples and reference material iteratively.
- Refine based on likely user questions and review feedback.
- Prefer tested, accurate content over exhaustive but speculative detail.

### README Review Process
When reviewing generated READMEs:
- Verify that examples work as written.
- Check that installation instructions are complete and accurate.
- Ensure the depth matches the target audience.
- Confirm consistent formatting and professional presentation.

## Content Organization Patterns

### Information Hierarchy
README content SHOULD generally follow the user journey:
1. **Discovery**: title, description, key features
2. **Evaluation**: screenshots, demos, alternatives
3. **Getting Started**: installation and first usage
4. **Learning**: common workflows and examples
5. **Reference**: configuration, API, troubleshooting
6. **Contributing/Community**: development and contribution info

### Section Ordering Standards
- Place the most critical information first.
- Group related information together.
- End with contribution, license, and community information where relevant.
- Use consistent section naming across projects when possible.

## Formatting and Style Guidelines

### Markdown Best Practices
- Use consistent heading hierarchy and spacing.
- Include blank lines around code blocks and major sections.
- Use proper list formatting with consistent indentation.
- Include alt text for images and meaningful link text.
- Test Markdown rendering in the target platform where practical.

### Visual Design Principles
- Use whitespace to improve scanability.
- Separate major sections clearly.
- Use code blocks, emphasis, and lists strategically.
- Keep text readable and reasonably concise.
- Maintain accessible presentation and clear contrast where visuals are used.

## Checklist

- [ ] Clear project title and concise description
- [ ] Installation instructions tested and complete
- [ ] Basic usage example works as written
- [ ] Code examples are complete, runnable, and correctly highlighted
- [ ] README structure matches the user journey and target audience
- [ ] Links, version information, and commands are current
- [ ] License and relevant community/contribution information are present
- [ ] README content matches actual project behavior

## Anti-Patterns to Avoid

**❌ Incomplete Installation Instructions:**
```
# Too vague
## Installation
Just install the dependencies and run it.
```

**❌ Non-Working Code Examples:**
```
# Missing imports, won't actually run
result = awesome_function("test")
print(result)
```

**❌ Overwhelming Wall of Text:**
```
# No structure or breathing room
This project does many things and here's everything about it in one giant paragraph that goes on and on without any breaks or organization making it very difficult to scan and find specific information that users need when they're trying to understand what this project does and how to use it...
```

**❌ Outdated or Incorrect Information:**
```
# Information that doesn't match current project state
This project requires Node.js 8+ (when it actually requires Node.js 16+)
Run `npm start` to begin (when the actual command is `npm run dev`)
```

**❌ Missing Critical Information:**
```
# Assumes too much knowledge
## Usage
Configure the settings and run the application.
```

## Precedence and Overrides
- Depends on: `global-global`.
- This file is the canonical source for README-specific structure, accuracy, and example requirements.
- Narrower language- or framework-specific documentation rules MAY tighten these expectations for their own scopes.
- Overrides: none.

## Changelog
- 2.1.0: Canonicalized the README rules, reduced overlap with the global documentation baseline, and tightened README-specific ownership around structure, runnable examples, and maintenance expectations.
- 2.0.0: Refined structure and standards for comprehensive, user-friendly README creation and maintenance.
- 1.0.0: Initial README guidance.

---

Following these rules ensures README files serve as effective project documentation that welcomes new users, provides clear guidance, and remains aligned with actual project behavior.
