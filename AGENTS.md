# AGENTS.md

This file defines the rules and workflow that all AI models and agentic platforms **must** follow when contributing to this repository.

---

## Branch Policy

- AI models **may only** make changes and commits into branches that were created and initiated by the AI itself.
- AI models **must not** make changes or commits directly into any branch created or initiated by the owner (human).

## Merge / Pull Request Policy

- AI models **must** merge their changes into owner (human) branches exclusively through a **Pull Request (PR)** originating from the AI's own branch.
- Direct pushes or merges into owner branches are **strictly prohibited**.
- After making changes, the AI **must** open a PR targeting the **branch from which the instructions were given**.
- If commands come from a PR:
  - If source branch of the PR created/initiated by owner/human, AI model **must** make changes in its own branch and make PR into said source branch.
  - If source branch of the PR created/initiated by AI models, AI model can commits directly.
- If commands come from an issue(assignments):
  - By default AI models **must** make changes in its own branch and make PR into master branch. Unless owner/human specifies otherwise.
- If commands come from new agent session or from a chat session:
  - By default AI models **must** make changes in its own branch and make PR into master branch. Unless owner/human specifies otherwise.

## Pre-PR Checklist

Before finalizing any changes and making PR, the AI model **must** run the following command:

```bash
make all
```

This handles all necessary checks including — but not limited to — error checking, code formatting, and linting. The PR **must not** be submitted if this command fails.
All suggestions, warnings and errors from this command **must** be addressed before making PR.

## Test Coverage

- At every change or PR, the AI **must** ensure that test coverage adequately covers all new changes.
- Test coverage **must not** decrease as a result of the changes.
- Coverage must be verified by generating an lcov report using:

```bash
make lcov
```

- If `llvm-cov` not installed, install it using `cargo install cargo-llvm-cov`.
- If coverage has decreased, the AI **must** add the necessary tests to restore or exceed the previous coverage level before proceeding.
- If some of code sections are unreachable by tests, skip it and make report in PR's description.

## New Feature Requirements

- If the changes introduce a new **feature** in any form, the AI **must** write new tests covering all coverage that can reasonably be covered for that feature.
- This requirement is **waived only** if the owner explicitly instructs otherwise.

---

> These rules apply universally across all agentic platforms and AI models interacting with this repository.
