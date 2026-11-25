---
id: TASK-98
title: Say that generated allow rules take effect only once the folder is trusted
status: To Do
assignee: []
created_date: '2026-11-25T15:34:00Z'
labels:
  - area:spec
  - needs:owner-approval
milestone: m-7
dependencies: []
priority: low
ordinal: 200390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
From TASK-97 (verified 2026-11-25 against the Claude Code docs and 2.1.323): in a project .claude/settings.json, Claude Code ignores permissions.allow until the person trusts the folder, while ask and deny apply at once. So the allow rules alz generates have no effect in an untrusted folder. Decide (owner) whether the spec states this and whether alz doctor reports it.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The spec states when generated allow rules take effect (owner approved)
- [ ] #2 alz doctor reports it, or the spec says why it does not (test or spec diff)
<!-- AC:END -->
