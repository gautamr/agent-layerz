---
id: TASK-55.7.1
title: >-
  R6.1: Drive alz account allow through a pseudo-terminal in the acceptance
  suite
status: In Progress
assignee:
  - '@implementer'
created_date: '2026-11-23T03:26:00Z'
updated_date: '2026-11-24T10:11:00Z'
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
<!-- SECTION:NOTES:END -->
