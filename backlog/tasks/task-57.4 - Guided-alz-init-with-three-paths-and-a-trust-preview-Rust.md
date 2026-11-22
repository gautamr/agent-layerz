---
id: TASK-57.4
title: Guided alz init with three paths and a trust preview (Rust)
status: To Do
assignee: []
created_date: '2026-11-22T14:00:00Z'
updated_date: '2026-11-22T14:00:00Z'
labels:
  - area:cli
  - area:security
milestone: m-3
dependencies:
  - TASK-57.1
  - TASK-55.9
  - TASK-57.6
  - TASK-13
parent_task_id: TASK-57
priority: medium
ordinal: 54390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Build alz init as the fast path from the spec: the starter (pinned to its newest tag), a new personal layer from a source, or an existing layer by path or remote source; non-interactive flags for each; a trust preview listing the layer's hooks (with their commands), MCP servers, permissions and settings before anything is written, with confirmation on a terminal; refusal or a clear flag requirement without a terminal. Uses the default stack from the machine-local config when set.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Each path works interactively and with flags; tests cover a declined preview writing nothing
- [ ] #2 Without a terminal, init never fetches a remote default and tells the user which flag to pass
- [ ] #3 The preview names every hook command, MCP server, permission rule and setting the layer adds
<!-- AC:END -->
