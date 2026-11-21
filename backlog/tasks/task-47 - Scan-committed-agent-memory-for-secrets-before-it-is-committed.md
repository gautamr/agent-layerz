---
id: TASK-47
title: Scan committed agent memory for secrets before it is committed
status: To Do
assignee: []
created_date: '2026-11-21T17:40:00Z'
labels:
  - area:memory
  - area:security
milestone: m-3
dependencies: []
priority: high
ordinal: 13390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Project memory in .claude/agent-memory/ is committed by design (spec 4.5), so a token an agent writes there gets pushed. Nothing scans it today. Source: a design review (2026-11-21) of a cross-tool memory store for AI coding tools; this task keeps one of its good ideas.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 alz check fails when a file under .claude/agent-memory/ matches a secret pattern, naming the file and line without printing the value (test with a fake token fixture)
- [ ] #2 The scan uses gitleaks when it is installed and built-in patterns otherwise; tests cover both paths
- [ ] #3 The spec and README document the scan and how to mark a false positive
<!-- AC:END -->
