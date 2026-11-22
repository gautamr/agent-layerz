---
id: TASK-44
title: Verify autoMemoryDirectory and decide which tool sets it
status: To Do
assignee: []
created_date: '2026-11-21T17:40:00Z'
updated_date: '2026-11-21T18:40:00Z'
labels:
  - area:memory
milestone: m-3
dependencies: []
priority: medium
type: spike
ordinal: 10390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Every Claude account now shares one projects/ folder through a symlink, so all memory sits in one place for every client. Claude Code has an autoMemoryDirectory setting (present in 2.1.323; its schema says a checked-in .claude/settings.json ignores it) that could place memory per folder instead. Not tested yet. Under decision-15 alz owns account folders (TASK-50), but alz writes only committed project files, where this setting is ignored, so how alz sets it is open. Source: a design review (2026-11-21) of a cross-tool memory store for AI coding tools; this task keeps one of its good ideas.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 docs/target-facts.md records, with the Claude Code version, where autoMemoryDirectory is honoured (user settings, .claude/settings.local.json, --settings) and that a committed .claude/settings.json ignores it, each tested in a scratch repository
- [ ] #2 It records whether a value in user settings applies per project or to every project, and whether ~/ expands
- [ ] #3 The spec says how alz sets it (user config, an uncommitted local file, or the account hook from TASK-50.6) and why, citing decision-15
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
2026-11-22 (@architect): criterion 3 now names TASK-50.6, which builds the shell hook (TASK-50.3 is alz run).
<!-- SECTION:NOTES:END -->
