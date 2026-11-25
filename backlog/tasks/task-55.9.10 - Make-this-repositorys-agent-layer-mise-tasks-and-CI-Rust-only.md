---
id: TASK-55.9.10
title: Make this repository's agent layer, mise tasks and CI Rust only
status: Done
assignee:
  - '@implementer'
created_date: '2026-11-24T17:44:00Z'
updated_date: '2026-11-25T13:28:00Z'
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
- [x] #1 grep -rnE 'npm|npx|node |vitest|src/|test/|referenceDrift|reference drift|TypeScript' .layer prints nothing, or only lines the notes list (grep)
- [x] #2 The Stop hook runs only the Rust checks, and the tests rule and the spec-test skill describe Rust tests written from the spec (file review)
- [x] #3 mise.toml has no node tool and no task that runs npm, npx or node, mise run test runs the Rust tests, and mise run install:local installs the Rust alz as one regular file at ~/.local/bin/alz (grep, one install run in notes)
- [x] #4 grep -rnE 'npm|node|vitest' .github/workflows prints nothing, and mise run workflows:lint passes
- [x] #5 mise run alz:build regenerates AGENTS.md, CLAUDE.md and .claude/ with the change, and mise run alz:check passes
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Implementation (implementer, 2026-11-25). Evidence per AC:
AC1: grep -rnE 'npm|npx|node |vitest|src/|test/|referenceDrift|reference drift|TypeScript' .layer prints nothing. src.md and add-acceptance-test are removed; rules/tests.md now covers crates/**/tests/**, fixtures/** and fuzz/** and describes Rust tests written from the spec; new skill .layer/skills/spec-test; implementer, reviewer, docs-writer, release-engineer, rust.md, threat-model and 50-repo.md reworded for Rust only.
AC2: the Stop hook is now 'cargo fmt --all --check && cargo clippy --workspace --all-targets --all-features --locked -- -D warnings && cargo test --workspace --locked'. dist/** is gone from protect-generated. Permissions: the npm/npx/node allows are removed and the npm deny rules are replaced by denies for cargo publish, yank, owner and login (owner to review).
AC3: mise.toml has no node tool and no npm/npx/node task (grep prints nothing). Tasks: test = cargo test --workspace --locked; lint = cargo fmt --check + clippy -D warnings; rust:check = lint + test; install, test:acceptance, build and test:node24 are removed; install:local runs scripts/install-local.sh, which is now the Rust installer (the old Node script was overwritten, install-local-rust.sh removed, install:local:rust removed). mise.lock: the node entries are removed. Install run: HOME could not be redirected in the worktree sandbox, so PREFIX=<scratch> mise run install:local was run twice: installed <scratch>/bin/alz as a regular, non-symlink file (-rwxr-xr-x), 'alz 0.1.0', the second run updated it in place and printed 'previous alz 0.1.0'. The default target is $HOME/.local/bin/alz by the script's prefix rule; the real ~/.local/bin was not touched.
AC4: grep -rnE 'npm|node|vitest' .github/workflows prints nothing. tests.yml (typescript and acceptance-rust jobs) is deleted; rust.yml keeps its pinned actions, permissions and persist-credentials: false and only two comments changed. mise run workflows:lint: the actionlint step failed because the shellcheck shim has no version; run with actionlint -shellcheck=<mise where aqua:koalaman/shellcheck@0.11.0>/shellcheck: exit 0; zizmor --pedantic --offline .github/workflows: no findings.
AC5: the build needs ../agent-layerz-personal, so the worktree was copied (without target/ and backlog/) next to a copy of the personal layer in the scratchpad, the Rust alz build ran there, and only .claude/{agents,rules,skills}, .claude/settings.json, AGENTS.md, CLAUDE.md, .mcp.json and layer.lock were copied back. alz check in the copy of the final state: 'check passed: 67 generated files match the layers and the lock'. mise run alz:check itself cannot run in the worktree (no sibling personal layer).
rust:ci: mise run rust:ci exit 0 (fmt, clippy -D warnings, 2043 tests passed 0 failed, doc, msrv, deny, audit).
For the owner: hooks, permissions and CI changed; the required status check named 'typescript and acceptance (ubuntu)' and 'acceptance against the Rust binary (ubuntu)' of the deleted tests workflow must be removed from branch protection. The personal layer's retrospective skill still mentions npx ccusage (outside this repository). Left for 55.9.11: package.json, npm files, biome/vitest/tsconfig, src/, test/, evals scripts and README references to mise run build/lint.

Correction (lead's order): the npm deny rules stay in .layer/permissions.yaml next to the cargo denies, because the npm package exists until the owner unpublishes it and deny rules only get stricter. AC1's grep therefore prints exactly these lines, the intended exception: the comment line 7 and the six deny rules 'bash:npm publish*', 'unpublish*', 'deprecate*', 'dist-tag*', 'token*', 'login*' (permissions.yaml lines 9-14). Remove them when the owner has unpublished the package. The npm allow rules (npm test, npm run, npx vitest/tsc/biome, node dist/cli.js) are still removed; the earlier note that the npm denies were replaced is superseded. Generated files rebuilt in the scratch copy: .claude/settings.json and layer.lock changed, alz check passed there (67 generated files match).
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
.layer/, mise and CI are Rust only: the Stop hook runs the cargo checks, Node is gone from mise and CI, install:local installs the Rust alz; the npm publish denies stay until the package is unpublished. Owner approved; merged in #23 (1166ebb).
<!-- SECTION:FINAL_SUMMARY:END -->
