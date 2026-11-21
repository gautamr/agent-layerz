---
id: TASK-28.8
title: Package Agent Lens as an installable plugin
status: To Do
assignee:
  - '@release-engineer'
created_date: '2026-11-19T18:00:00Z'
updated_date: '2026-11-19T18:06:00Z'
labels:
  - area:lens
  - area:release
  - uc:L7
milestone: m-5
dependencies:
  - TASK-28.7
  - TASK-4
documentation:
  - backlog/docs/doc-2 - Agent-Lens-plan.md
parent_task_id: TASK-28
priority: medium
ordinal: 140
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
One-line install for anyone (UC-L7) and checks on every pull request.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 plugins/agent-lens with a .claude-plugin/marketplace.json in this repository
- [ ] #2 claude plugin validate and claude plugin test run in CI
- [ ] #3 Installs with one /plugin install line on a clean machine
<!-- AC:END -->
