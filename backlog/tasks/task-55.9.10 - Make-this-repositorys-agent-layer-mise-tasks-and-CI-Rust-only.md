---
id: TASK-55.9.10
title: Make this repository's agent layer, mise tasks and CI Rust only
status: To Do
assignee:
  - '@implementer'
created_date: '2026-11-24T17:44:00Z'
updated_date: '2026-11-24T17:49:00Z'
labels:
  - area:agents
  - area:ci
  - area:rust
milestone: m-7
dependencies:
  - TASK-55.9.1
  - TASK-55.9.2
parent_task_id: TASK-55.9
priority: medium
ordinal: 192390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Once the Rust tests cover the spec, the development loop drops Node. .layer/ still runs and describes the TypeScript era: the Stop hook runs npm lint and vitest, rules src.md and tests.md apply to src/** and test/**, the add-acceptance-test skill and the implementer, reviewer, docs-writer and release-engineer agents describe vitest cases and reference drift, permissions allow npm, npx and node, the protect-generated hook lists dist/**, and 50-repo.md's Map and Commands name src/ and the vitest suite. mise's tools and its test, test:acceptance, lint, build, test:node24 and install:local tasks use Node, and tests.yml runs the typescript and acceptance jobs. These become Rust only: a tests rule for the Rust tests and fixtures/, a skill for writing a test from the spec, a Stop hook that runs the Rust checks, install:local installing the Rust alz as one regular file at ~/.local/bin/alz, and no Node job. Generated files change only through mise run alz:build. Changing agent layers, hooks, permissions and CI is part of decision-39; the PR still needs the owner's review. Out of scope: deleting files (next subtask).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 grep -rnE 'npm|npx|node |vitest|src/|test/|referenceDrift|reference drift|TypeScript' .layer prints nothing, or only lines the notes list (grep)
- [ ] #2 The Stop hook runs only the Rust checks, and the tests rule and the spec-test skill describe Rust tests written from the spec (file review)
- [ ] #3 mise.toml has no node tool and no task that runs npm, npx or node, mise run test runs the Rust tests, and mise run install:local installs the Rust alz as one regular file at ~/.local/bin/alz (grep, one install run in notes)
- [ ] #4 grep -rnE 'npm|node|vitest' .github/workflows prints nothing, and mise run workflows:lint passes
- [ ] #5 mise run alz:build regenerates AGENTS.md, CLAUDE.md and .claude/ with the change, and mise run alz:check passes
<!-- AC:END -->
