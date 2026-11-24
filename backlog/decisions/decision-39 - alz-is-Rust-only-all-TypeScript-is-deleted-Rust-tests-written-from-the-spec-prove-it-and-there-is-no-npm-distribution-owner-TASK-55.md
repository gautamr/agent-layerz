---
id: decision-39
title: >-
  alz is Rust only: all TypeScript is deleted, Rust tests written from the spec
  prove it, and there is no npm distribution (owner, TASK-55)
date: '2026-11-24T17:40:00Z'
status: accepted
---
## Context

decision-21 froze the TypeScript implementation as the behavioural reference for the Rust port,
made cutover wait for byte-identical output on every fixture, and kept `npm install -g
agent-layerz` working through an npm wrapper. decision-30 stopped all TypeScript fixes and made
the vitest acceptance suite under `test/` the oracle, with the reference's known failures marked
as reference drift. By R6 (TASK-55.7) the Rust `alz` implements the spec, the Rust workspace
already holds unit, integration, `assert_cmd` and proptest tests, and much of the remaining R8
work was parity bookkeeping: drift marks, the §13 drift list, a parity run against a reference
that is known to be wrong. The owner decided on 2026-11-24 (stated to the lead session, which
recorded it through the architect) to end the reference role and to make the Rust port the
project.

## Decision

The owner's three decisions, final:

1. Everything in TypeScript is deleted: the implementation `src/`, the Node build and package
   files, `test/reference/`, and the vitest acceptance suite `test/acceptance/` with its helpers.
   The TypeScript code and tests are not ported and are not followed. The requirements come from
   `docs/LAYERING-SPEC.md`, and they are tested with Rust tests only (unit, integration,
   `assert_cmd` against the binary, proptest), written from the spec.
2. There is no npm distribution and no npm wrapper. The owner removes the npm package himself.
3. TASK-55's criterion "byte-identical to the TypeScript reference for every fixture" is settled
   by this decision: it no longer applies. The spec, proven by the Rust tests, is the only
   measure of correct output.

This supersedes decision-30's reference role (the oracle under `test/`, reference drift, the
byte-identical exception) and the TypeScript parts of decision-21 (the frozen reference, parity
before cutover, the npm wrapper). It also supersedes decision-4, which was about npm publishing.
What stays: bugs are fixed only in Rust (decision-30), `src/` is not changed before it is
deleted, the spec states the intended behaviour, releases are published only by CI, and the rest
of decision-21 (Rust for all tooling, the phases, doc-6).

## Consequences

- R8 (TASK-55.9) becomes the Rust-only cutover, split into subtasks. The vitest suite keeps running
  in CI as a safety net until a spec-to-Rust-test coverage matrix (`docs/spec-coverage.md`, with a
  Rust guard test) shows no gap; only then are `src/`, `test/` and Node deleted.
- The fixtures the Rust tests read move from `test/fixtures/` to a workspace-level `fixtures/`
  folder. Golden trees and emitter samples made once from the reference stay as expected output
  owned by the Rust tests; the Node scripts that made them are deleted.
- The spec states only what alz does: §13's reference drift list and every sentence about the
  TypeScript `alz`, the reference or parity are removed. The user-visible differences from the
  published 0.1.0 move to `CHANGELOG.md`.
- This repository builds its own agent config with the Rust binary (`mise run alz:build`), and
  `.layer/` (the Stop hook, rules, skills, agents, permissions, instructions) describes Rust-only
  work.
- R7 (TASK-55.8) distributes prebuilt binaries (decision-25 targets) as GitHub releases from CI,
  through Homebrew, and as a local install of one regular file. It no longer blocks R8; the first
  Rust release waits for R8.
- Open tasks that only chase TypeScript parity or the npm pipeline are archived by the lead.
- doc-5's R7 npm wrapper and R8 parity run are superseded by this record; doc-5 is not edited.
- The installed Node `alz` 0.1.0 keeps its known bugs; the first Rust release says what it fixes.