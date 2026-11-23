---
id: TASK-55.6.5
title: 'R5.5: settings.json with permissions, floor fields and hooks, and .mcp.json'
status: To Do
assignee:
  - '@implementer'
created_date: '2026-11-23T18:08:00Z'
labels:
  - area:rust
  - area:render
  - area:security
milestone: m-7
dependencies:
  - TASK-55.6.2
  - TASK-76
parent_task_id: TASK-55.6
priority: high
type: task
ordinal: 137390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Plan for this step (§7.5 to §7.9, §8.1, target-facts §4, §7b, §7c).
- permissions: allow, ask and deny as Claude rules, defaultMode, additionalDirectories from workspace (hub and add_dirs as written), and the three floor fields as permissions.disableBypassPermissionsMode, blockReadsOutsideWorkingDirectories and blockWritesOutsideWorkingDirectories, only once docs/target-facts.md records them (TASK-76).
- hooks: grouped by native event in stack and file order; one handler per tool and path condition; `run:` copies the script once to `.claude/hooks/<layer>/<path>` with its executable bit and the command `"${CLAUDE_PROJECT_DIR}/<path>"`; `command:` with the project-dir variable rewritten; matcher, timeout and onFailure as the reference writes them, with the hook-on-fail warnings.
- model: the default alias's Claude model.
- The generated keys deep-merged over Merged::settings for claude, written with the stable JSON emitter (keys in code-point order, decision-30); no settings.json when the result is empty.
- .mcp.json: the enabled servers (stdio: command, args, env as `${NAME}` references; http and sse: type and url), only when one is enabled.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 settings.json, the copied hook scripts and .mcp.json match the reference's bytes for fixtures 07, 08, 09, 16, 19 and 33, keys in code-point order (Rust tests)
- [ ] #2 The floor fields are written under the three permission keys exactly when a layer set them true, and no other path writes those keys (unit tests)
- [ ] #3 on_fail: deny on stop or subagent_stop and on post_tool gives the hook-on-fail warnings, and a run: script used by two hooks is copied once (unit tests)
<!-- AC:END -->
