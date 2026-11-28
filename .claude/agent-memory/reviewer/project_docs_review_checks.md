---
name: docs-review-checks
description: Checks that found real problems in README/CONTRIBUTING reviews - renamed headings cited by CHANGELOG, version string ambiguity, fixture numbering vs spec §13, generated Commits section vs contributor advice
metadata:
  type: project
---

Checks that paid off in the TASK-7/TASK-131 docs review (2026-11-28):

- CHANGELOG.md cites README sections by their quoted heading text ("README, Install"). When a README heading is renamed, grep CHANGELOG and docs/ for the old name.
- The crates' version can still be the old release number (the Rust `alz` reported 0.1.0 while docs said "moving from 0.1.0"). Read `crates/alz/Cargo.toml` before accepting version-based migration advice.
- `fixtures/NN-<topic>` numbers do not map one-to-one to the items of spec §13 (e.g. 20, 22, 26 differ). Do not let docs claim the mapping. Most expected output is in `crates/alz/tests/fixtures/golden/`, not `fixtures/`.
- The generated AGENTS.md "Commits" section pins the owner's identity and tells agents not to commit under another one; contributor docs that say "commit under your own name" conflict with it.

- Algebra docs (docs/algebra.md, README "The algebra"): check each "Used for" cell by grepping the type's uses (`Floor<`, `Replace<`), not its doc comments. Doc comments cite structures by analogy (hooks.rs calls the lock "a floor"), and the docs copy them. Proptest runs 256 cases by default (decision-35), so "thousands of inputs" is too strong. Layer-level associativity claims break on hooks: a layer cannot disable its own hook.

**Why:** these were invisible from the diff alone and only showed up by reading neighbouring files.

**How to apply:** for any README/CONTRIBUTING/CHANGELOG review, run these greps in addition to the CLI and mise.toml checks. See [[review-tooling-no-shell]].
