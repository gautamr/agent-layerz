---
id: TASK-28.1
title: Bring in the superpowers community skills through the repo layer
status: To Do
assignee:
  - '@release-engineer'
created_date: '2026-11-19T18:00:00Z'
updated_date: '2026-11-19T18:06:00Z'
labels:
  - area:lens
  - area:skills
  - enabler
milestone: m-5
dependencies: []
documentation:
  - backlog/docs/doc-2 - Agent-Lens-plan.md
parent_task_id: TASK-28
priority: high
ordinal: 60
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Community skills should arrive the alz way: declared in .layer/settings.yaml and rendered into .claude/settings.json (decision-5). Claude Code is reported not to always offer project-level plugins for install, and the plugin may add context at session start.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 superpowers from the official Claude plugin marketplace is declared in .layer/settings.yaml (enabledPlugins), pinned where the marketplace allows, and alz build writes it to .claude/settings.json
- [ ] #2 alz ls --live in this repo shows the superpowers skills loaded
- [ ] #3 If Claude Code does not offer the install, CONTRIBUTING.md documents the one-line /plugin install fallback
- [ ] #4 Context use before and after is measured with /context and recorded in the task notes; the owner decides to keep or drop it
- [ ] #5 The plugin settings keys are recorded in docs/target-facts.md with their verification status
<!-- AC:END -->
