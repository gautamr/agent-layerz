---
id: decision-30
title: >-
  The TypeScript reference is never changed again; every bug is fixed in Rust
  only, the spec states the intended behaviour and the acceptance suite marks
  the reference's known drift (owner, supersedes the fixes-only rule of
  decision-21)
date: '2026-11-23T04:22:00Z'
status: accepted
---
## Context

decision-21 froze the TypeScript implementation as the behavioural reference for the Rust port
with "fixes only, no new features". In practice each bug found while porting (TASK-59 to TASK-64,
TASK-66) meant a second fix in
`src/`, a second review, and the risk of changing the oracle the port is measured against.
The owner decided on 2026-11-23 to stop all work on the TypeScript code: the port goes on only in
Rust, and every bug is fixed there, whenever and wherever it is found.

## Decision

- `src/` is never changed again, not even for a bug or a security flaw. It stays as it is until
  cutover (R8, TASK-55.9) removes it.
- A bug found in the reference is fixed only in the Rust `alz`, in the phase that ports that
  behaviour. Its task names that phase and is closed there.
- `docs/LAYERING-SPEC.md` states the intended behaviour. Where the spec and the reference differ,
  the spec wins and the Rust port follows the spec.
- The acceptance suite asserts the spec. A case that the reference is known to fail is marked as
  reference drift: it is expected to fail against the bundled TypeScript reference (ALZ_BIN unset)
  and must pass against any binary given as ALZ_BIN. Each drift case names its task.
- "Byte-identical to the TypeScript reference for every fixture" holds except for recorded drift.
- The test suite under `test/` is the oracle and keeps changing; only the implementation under
  `src/` is frozen.

## Consequences

- No TypeScript fix, review or release for bugs found from now on; the installed Node `alz` keeps
  those bugs until the Rust binary replaces it.
- Each port phase carries the bugs of the behaviour it ports, and the next phase depends on them.
- Uncommitted TypeScript fixes (TASK-64's `src/` change) are reverted.
- The repository layer (`.layer/`) says that `src/` is never changed, and the implementer agent
  and the tests rule describe drift cases.
- decision-21 stays in force otherwise: the port, its phases, and cutover as planned.
