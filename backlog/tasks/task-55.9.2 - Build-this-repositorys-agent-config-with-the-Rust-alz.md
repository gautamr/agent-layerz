---
id: TASK-55.9.2
title: Build this repository's agent config with the Rust alz
status: To Do
assignee:
  - '@implementer'
created_date: '2026-11-24T17:44:00Z'
updated_date: '2026-11-24T19:01:00Z'
labels:
  - area:rust
  - area:ci
  - area:agents
milestone: m-7
dependencies:
  - TASK-13
parent_task_id: TASK-55.9
priority: medium
ordinal: 184390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
This repository is managed by alz itself: AGENTS.md, CLAUDE.md, .claude/, .mcp.json and layer.lock are generated from .layer/ and the personal layer. mise's alz:build, alz:check and alz:ls run npm run -s build && node dist/cli.js, so they must move to the Rust binary before Node is deleted (decision-39); doing it early also runs the Rust alz on a real repository. The Rust alz checks layers more strictly than 0.1.0, so .layer/ may need changes (a change to the personal layer is a request to its owner, in another repository), and generated files change only through mise run alz:build. CI then runs mise run alz:check so the generated config cannot drift; this covers TASK-14. Untrusted input: the repository's own .layer/ and layer.lock, built by the binary of the same commit, in a CI job that holds no secret. The CI change is part of decision-39; the PR still needs the owner's review. Out of scope: the Stop hook, rules, skills and agent texts (the agent-layer subtask).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 mise's alz:build, alz:check and alz:ls build alz with cargo build --release --locked -p alz and run target/release/alz, and none runs npm or node (grep of mise.toml)
- [x] #2 After mise run alz:build, mise run alz:check passes on a clean checkout (output in notes)
- [x] #3 Every change to a generated file came from mise run alz:build and is listed in the notes with the spec section that explains it (diff review)
- [ ] #4 The rust workflow runs mise run alz:check on ubuntu-latest and fails on a stale generated file or layer.lock (a failing run in notes), and mise run workflows:lint passes
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
AC1: mise.toml alz:build, alz:check, alz:ls now run 'cargo build --release --locked --package alz && target/release/alz <build|check|ls>'; grep of the three task bodies for npm or node finds nothing. AC2/AC3: in this git worktree the personal layer path:../agent-layerz-personal does not resolve (error [source] layer directory not found), because the worktree sits two levels deeper than the main checkout. So I built and checked in a scratch copy where the personal layer (v0.4.0, read only copy) sits next to the repository copy. Rust alz build there: 'built 68 files for claude: 0 created, 0 updated, 0 deleted' and git status clean, so NO generated file and not layer.lock changed (no spec section needed). alz check there: 'check passed: 68 generated files match the layers and the lock' (also with --skip-identity). Warnings only: agent-replace for architect and security-reviewer overlays. The owner should rerun mise run alz:build and mise run alz:check in the main checkout to confirm. AC4: new job agent-config in .github/workflows/rust.yml runs 'mise run alz:check -- --skip-identity' on ubuntu-latest (spec section 14 CI, section 10.3 check). Local failure evidence (no CI run, nothing pushed): appending a line to AGENTS.md gives 'error [check] AGENTS.md: differs from a fresh build' and exit 1; changing a .layer instruction also gives 'error [check] layer.lock: out of date'. CI CHOICE: the runner cannot resolve path:../agent-layerz-personal (TASK-13, owner, makes it github:). The job probes layer.yaml for a path:.. source; if found it emits a ::warning annotation and skips only the check steps, otherwise it runs the check, so it activates itself when TASK-13 lands. Until then the job does not yet fail CI on a stale config. workflows:lint: actionlint (with shellcheck from mise where aqua:koalaman/shellcheck@0.11.0, because the mise shim has no version set) exit 0; zizmor --pedantic --offline no findings. mise run rust:check passed.

Lead (2026-11-25): merged in #19 (fd6bfb3). After TASK-84 (#20), mise run alz:check passes in the real checkout: 'check passed: 68 generated files match the layers and the lock', exit 0; no generated file changed. AC #4 stays open: CI cannot resolve the personal layer (path:../agent-layerz-personal), so the agent-config job warns and skips until TASK-13 stacks it through github:.
<!-- SECTION:NOTES:END -->
