---
id: TASK-89
title: Small R5 review follow-ups
status: To Do
assignee: []
created_date: '2026-11-24T06:35:00Z'
labels:
  - area:rust
milestone: m-7
dependencies: []
priority: low
ordinal: 160390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
R5 review (2026-11-24): (1) golden_trees.rs compares stderr only by level and code and inherits the developer's environment (env_clear plus an allowlist; edit_is_caught must not pass when AGENTS.md cannot be read); (2) adopt/natives.rs and spec §4.3 should name DiffersOnlyInCase and NotText among the unadoptable collisions; (3) decide whether alz check reports a stale .gitignore block with commit_generated: false and state it in §10.3; (4) init's create_new inside a lazy map closure; (5) pipeline clones Merged and ResolvedStack per run; (6) doctor exits 1 silently when the working directory is unknown; (7) build ignores RepoRoot::apply's Applied.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Each item is fixed or answered in the spec, with tests where behaviour changes
<!-- AC:END -->
