---
id: TASK-55.9.11
title: >-
  Delete the TypeScript implementation, its tests and the Node files, and update
  the docs
status: To Do
assignee:
  - '@implementer'
created_date: '2026-11-24T17:44:00Z'
labels:
  - area:rust
  - area:docs
milestone: m-7
dependencies:
  - TASK-55.9.10
parent_task_id: TASK-55.9
priority: medium
ordinal: 193390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The last step of decision-39: with no gap in the coverage matrix and nothing running Node, delete src/, test/ (its fixtures already moved), package.json, package-lock.json, tsconfig.json, biome.json, vitest.config.ts, scripts/install-local.sh and docs/standards/typescript.md, drop the dist/ and node_modules/ lines from .gitignore, and delete each fixtures/ folder that no Rust test reads. README.md, CHANGELOG.md, FRAMEWORK-GAPS.md and evals/ then describe only the Rust alz: building from source and the local install (R7 adds Homebrew and releases), how to leave an npm install of 0.1.0 behind, and no npm command. Dropping the npm install path is user-facing and ships in the first Rust release (R7, TASK-55.8). Out of scope: distribution, and removing the npm package (owner).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 git ls-files src test package.json package-lock.json tsconfig.json biome.json vitest.config.ts scripts/install-local.sh docs/standards/typescript.md and git ls-files '*.ts' '*.mts' '*.cts' '*.js' '*.mjs' '*.cjs' print nothing (grep)
- [ ] #2 Every folder under fixtures/ is read by at least one Rust test (the check's command and output in notes)
- [ ] #3 git grep -niE 'npm|npx|node[.]?js|typescript|vitest' -- README.md CHANGELOG.md FRAMEWORK-GAPS.md evals docs .gitignore prints only CHANGELOG lines about 0.1.0 and lines the notes list (grep)
- [ ] #4 Every command in README's install and development sections works in this checkout (each run once, output in notes)
- [ ] #5 mise run rust:ci and mise run alz:check pass
<!-- AC:END -->
