---
id: TASK-55.7.11
title: 'R6.11: The terminal gate and the confirmation prompt'
status: In Progress
assignee:
  - '@implementer'
created_date: '2026-11-24T08:16:00Z'
updated_date: '2026-11-24T11:16:00Z'
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

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Implementer (2026-11-24), branch feat/rust-r6.11-prompt. alz-core account/person.rs (AgentSessionMarker, AgentSession non-empty, NotAPerson with every reason, decide) and account/confirmation.rs (Answer Line|Ended with a redacting Debug, Confirmation::of: trims only spaces and tabs, exact match). alz-io terminal.rs: PersonAtTerminal::check(&EnvSnapshot) (isatty of stdin, six markers by presence, no file system call) and ask(&mut self, prompt) -> Result<Answer, TerminalError>: saves the settings, clears ISIG and sets VEOL to VINTR (so the interrupt key ends the line; echo and line editing stay on), writes the prompt to stderr after the change, reads one line (one fill_buf; bytes without a newline, from Ctrl-C or Ctrl-D, are Ended), restores explicitly and in a Drop guard; no unsafe, rustix termios only (workspace rustix gains the termios feature, per decision-27's 2026-11-24 amendment). alz-test-support gains pty_run_executable() (finds pty-run above the test executable). AC1: crates/alz-io/tests/terminal.rs re-runs the test executable under pty-run: prompt on stderr and a typed answer per prompt (also two answers typed ahead), wrong answers (y, empty, other name, other case, name plus text) are no, a pipe on stdin is refused before anything is asked (terminal=false), each of the six markers alone and an empty or valued marker refuse and are named by name only, a pipe plus a marker names both. alz-core unit and proptests cover the parser and the person decision. AC2: Ctrl-C (alone and after the name) is Ended with exit by the child itself, not by SIGINT; the end of input likewise; the settings (input, output, control, local modes and special codes) read back through /dev/tty equal the ones before, after every answer. Negative control: with the ISIG change removed both Ctrl-C tests fail. AC3: std::io::stdin is a disallowed method in alz-core (existing), alz-io and alz clippy.toml, with one expect in terminal.rs; crates/alz-io/tests/standard_input_guard.rs scans the three crates' sources for stdin() and io::Stdin outside terminal.rs. Checks: mise run rust:ci exit 0 (fmt, clippy -D warnings, tests, doc, msrv, deny, audit); mise run lint ok. Spec needs: §7.13 should say that text typed and then ended with Ctrl-D (no newline) is the end of input, not an answer, so Ctrl-D after the name does not confirm. Not wired: alz account allow itself (R6.13), the refusal text (R6.13).
<!-- SECTION:NOTES:END -->
