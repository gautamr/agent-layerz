---
id: TASK-55.9.2
title: Build this repository's agent config with the Rust alz
status: To Do
assignee:
  - '@implementer'
created_date: '2026-11-24T17:44:00Z'
labels:
  - area:rust
  - area:ci
  - area:agents
milestone: m-7
dependencies: []
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
- [ ] #1 mise's alz:build, alz:check and alz:ls build alz with cargo build --release --locked -p alz and run target/release/alz, and none runs npm or node (grep of mise.toml)
- [ ] #2 After mise run alz:build, mise run alz:check passes on a clean checkout (output in notes)
- [ ] #3 Every change to a generated file came from mise run alz:build and is listed in the notes with the spec section that explains it (diff review)
- [ ] #4 The rust workflow runs mise run alz:check on ubuntu-latest and fails on a stale generated file or layer.lock (a failing run in notes), and mise run workflows:lint passes
<!-- AC:END -->
