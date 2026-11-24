---
id: TASK-55.7.11
title: 'R6.11: The terminal gate and the confirmation prompt'
status: To Do
assignee: []
created_date: '2026-11-24T08:16:00Z'
updated_date: '2026-11-24T08:19:00Z'
labels:
  - area:rust
  - area:accounts
  - area:security
milestone: m-7
dependencies:
  - TASK-55.7.1
  - TASK-55.7.2
parent_task_id: TASK-55.7
priority: high
type: task
ordinal: 170390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Plan for this step (§7.13 "Trust and alz account allow", doc-6 §3.4; owner question 4).
- alz-io terminal.rs: PersonAtTerminal::check(&EnvSnapshot) -> Result<PersonAtTerminal, NotAPerson>: standard input is a terminal (rustix termios isatty) and none of the six agent-session markers is set (presence, even empty); every reason is named, markers by name; constructible nowhere else, and checked before any file system call.
- ask(&mut self, prompt) -> Answer { Line, Ended }: the prompt to standard error, one line from standard input, the answer never printed by alz. Ctrl-C and the end of input are Ended, as owner question 4 decides; the terminal's settings are restored on every path.
- The rustix termios feature is recorded as an amendment to decision-27's feature list.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Under the R6.1 driver alz asks and reads one answer per prompt; without a terminal, or with any marker set (even empty), it refuses before any file system call and names each reason (pty and assert_cmd tests)
- [ ] #2 Ctrl-C and the end of input end the prompt as not confirmed, and the terminal settings are the same before and after (pty test)
- [ ] #3 Only alz-io's terminal module reads standard input (guard test or clippy disallowed-methods)
<!-- AC:END -->
