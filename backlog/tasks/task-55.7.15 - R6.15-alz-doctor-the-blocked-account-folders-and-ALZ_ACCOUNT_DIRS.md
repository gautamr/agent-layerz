---
id: TASK-55.7.15
title: 'R6.15: alz doctor: the blocked account folders and ALZ_ACCOUNT_DIRS'
status: In Progress
assignee:
  - '@implementer'
created_date: '2026-11-24T08:16:00Z'
updated_date: '2026-11-24T11:03:00Z'
labels:
  - area:rust
  - area:accounts
  - area:cli
milestone: m-7
dependencies:
  - TASK-55.7.2
  - TASK-55.7.7
parent_task_id: TASK-55.7
priority: high
type: task
ordinal: 174390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Plan for this step (§10 doctor row; TASK-50.4 AC #5 only; owner questions 5 and 8).
- doctor's account-folder list (alz-io account_names) leaves out ~/.claude-alz-blocked and ~/.codex-alz-blocked and warns once when one exists, since Claude creates a missing folder at a blocked start.
- ALZ_ACCOUNT_DIRS as the owner decides (recommended: not read; the spec says doctor lists CLAUDE_CONFIG_DIR, ~/.claude and ~/.claude-* only; reference drift).
- The rest of TASK-50.4 stays in m-3.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 alz doctor with ~/.claude-alz-blocked present does not list it as an account and prints one warning naming it (acceptance case, referenceDrift)
- [ ] #2 ALZ_ACCOUNT_DIRS is handled as the owner decided and the spec says so (acceptance case and spec diff)
<!-- AC:END -->
