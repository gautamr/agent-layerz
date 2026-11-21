---
id: TASK-48
title: Give alz build --adopt a dry run, a backup and a record of its sources
status: To Do
assignee: []
created_date: '2026-11-21T17:40:00Z'
labels:
  - area:cli
milestone: m-3
dependencies: []
priority: medium
ordinal: 14390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
--adopt rewrites hand-written .claude/, CLAUDE.md, AGENTS.md and .mcp.json into layer sources with no report-only mode and no backup, so a wrong guess is hard to undo. Source: a design review (2026-11-21) of a cross-tool memory store for AI coding tools; this task keeps one of its good ideas.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 alz build --adopt --dry-run lists every file it would convert, its destination and the reason, and writes nothing (git status unchanged afterwards)
- [ ] #2 Before writing, --adopt saves every file it replaces to a timestamped backup outside the generated paths and prints the backup's path
- [ ] #3 Each layer source created by --adopt records the path it came from
<!-- AC:END -->
