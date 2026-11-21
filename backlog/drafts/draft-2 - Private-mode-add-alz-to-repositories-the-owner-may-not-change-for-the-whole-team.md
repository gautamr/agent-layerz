---
id: DRAFT-2
title: >-
  Private mode: add alz to repositories the owner may not change for the whole
  team
status: Draft
assignee:
  - '@architect'
created_date: '2026-11-20T16:59:00Z'
labels:
  - idea
  - area:cli
  - area:spec
dependencies: []
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Found while preparing a client rollout: client-owned repositories may not accept committed agent config. .git/info/exclude hides new files only; a file the team already tracks (CLAUDE.md) still shows as changed. Other tools call this a private mode. Open questions: where the layers live, how tracked files are handled, how a teammate's clone stays clean.
<!-- SECTION:DESCRIPTION:END -->
