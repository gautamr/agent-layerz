---
id: TASK-55.7.1
title: >-
  R6.1: Drive alz account allow through a pseudo-terminal in the acceptance
  suite
status: In Progress
assignee:
  - '@implementer'
created_date: '2026-11-23T03:26:00Z'
updated_date: '2026-11-24T10:42:00Z'
labels:
  - area:test
  - area:rust
milestone: m-7
dependencies:
  - TASK-55.2.7
parent_task_id: TASK-55.7
priority: medium
ordinal: 82390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Moved from TASK-55.2.8 (R1.8) on 2026-11-23 at the owner's request: the backlog CLI cannot change a task's parent, so the task was recreated here and the R1.8 copy archived.

This brings the fake-terminal cases that R1.6 moved to reference back into the language-neutral oracle. `alz account allow` runs only when standard input is a terminal (§7.13). A spawned binary therefore needs a pseudo-terminal on stdin. stdout and stderr stay pipes, so the tests can still tell the streams apart. The driver must be able to wait for a prompt on stderr, change files (the "read again after the confirmation" cases), and then send an answer or the end of input.

Owner choice, open (see the TASK-55.2 comment):
- (a) A small pty driver binary in crates/alz-test-support, built with the safe rustix pty functions (openpt, grantpt, unlockpt, ptsname), so `unsafe_code = "forbid"` holds and no setsid is needed. R6 (TASK-55.7) reuses it for the Rust CLI tests of alz account allow. This needs a decision that lets alz-test-support use rustix outside alz-io; decision-27 says "only behind alz-io capability types".
- (b) The node-pty dev dependency: a native addon, which needs an intake decision.
- (c) Python's pty module through python3: a system dependency, which on macOS is another xcrun shim.
- (d) No pty: these cases stay reference-only, and R6 writes its own pty tests.

Once chosen:
- The cases marked "pending R1.8" move back to acceptance as spawned cases, and their reference copies are removed only where the acceptance case covers the same behaviour.
- The 12-process lock case gets a CLI form: twelve `alz account allow` runs under ptys answering at once.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The owner's choice of driver is recorded as a decision and linked from this task
- [ ] #2 Every fake-terminal case R1.6 marked pending R1.8 runs in test/acceptance against ALZ_BIN through the driver and passes on the TypeScript bundle; the notes list each case with its reference copy, kept or removed
- [ ] #3 A positive control shows the driver's stdin is a terminal and a plain spawn's is not: allow asks under the driver and refuses without it
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Owner (2026-11-22): no PTY driver until R6; choose it when R6 ports accounts (Rust driver in alz-test-support, node-pty or python3). The account allow cases stay in the reference project until then. Owner (2026-11-23): moved under R6.

Architect (2026-11-24, R6 plan): the fake-terminal cases this task brings back into test/acceptance pass on the TypeScript bundle but not yet on the Rust alz, so add each of them to test/ci/r6-pending.json (reason: needs alz account allow, R6.13 TASK-55.7.13, or the hint, R6.14 TASK-55.7.14) and raise test/ci/case-floor.json in the same change; R6.13 and R6.14 delete them as they pass. R6.11 (TASK-55.7.11) reuses the driver for the Rust tests of the prompt. Owner question 7 in TASK-55.7 repeats the driver choice.

Implementer (2026-11-24), branch feat/rust-r6.1-pty. Driver: crates/alz-test-support pty module plus the pty-run binary (rustix pty and termios, no unsafe, no new crate). pty-run [--deadline-ms N] [--] program args: opens a pty pair, becomes session leader, TIOCSCTTY, runs the program with the slave as stdin in its own process group (tcsetpgrp to it, SIGCONT), stdout and stderr inherited so a test with pipes tells them apart, its stdin is typed into the terminal (0x03 is Ctrl-C), its end sends 0x04, the echo is drained, the group is SIGKILLed at the deadline (exit 124); exit code is the program's, 128+signal, 125 for a driver error. Termios is needed only for tcsetpgrp (decision-27 amendment names pty only; owner please confirm termios for alz-test-support). cargo tree -p alz -e normal shows rustix with alloc,fs,process,std only (no pty, termios, alz-test-support); with dev edges: pty and termios, via alz-test-support. deny.toml: rustix wrappers gain alz-test-support; cargo deny ok. Evidence: cargo test -p alz-test-support (9 integration tests: terminal on stdin and pipes on stdout and stderr, typed line echoed, EOF, Ctrl-C trapped and untrapped, deadline kill with its child gone, no fd above 2, exit code, usage error; 5 unit tests), 8 repeated runs stable. TS: test/pty.ts (PtySession), ptyRun provided by test/setup (ALZ_PTY_RUN or cargo build), test/acceptance/40-account-allow-terminal.test.ts, 18 cases, pass on the TypeScript bundle, fail on the Rust alz, all 18 added to test/ci/r6-pending.json; case-floor.json 1083 -> 1101; check-r6-pending.ts exits 0 against target/release/alz (1101 cases, 274 failing, 274 listed). AC3 positive control: first case of file 40. NOT done: AC1 (decision link, owner-created), AC2 only partly: the other pending R1.8 cases of test/reference/32 (state refusals, lock cases, hint cases, escaping, one-chunk reader) are not yet moved; their reference copies are all kept. mise run rust:ci exit 0; mise run lint ok. mise run test under machine load average 50-79 failed 10 and 15 different timeouts (28, 30, 31, reference 30, 32) that passed in other runs; file 40 alone passes 18/18. Known driver limit: a program that cannot start may end pty-run by SIGHUP instead of exit 125 (message still printed).
<!-- SECTION:NOTES:END -->
