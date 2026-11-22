---
id: decision-21
title: >-
  Port alz to Rust and write all future tooling in Rust; the TypeScript
  implementation is frozen as the reference until the Rust binary reaches parity
  (owner)
date: '2026-11-22T12:52:00Z'
status: accepted
---
## Context

alz is about 10,600 lines of TypeScript on Node 22, with an 840-case acceptance suite and a
1,900-line spec, and has not been released. Measured on 2026-11-22: `alz account which` takes
about 70 ms, of which about 25 ms is Node starting; the planned shell hook runs on every prompt.
Several accepted security limits exist only because Node has no `openat`-style file access (a
folder swapped between checks, the component-by-component symlink walk of TASK-52). Users need
Node 22.12 and npm dependencies. The owner prefers Rust for desktop and developer tooling.

## Decision

alz is rewritten in Rust, and all future tooling the owner builds is written in Rust. The
TypeScript implementation on `main` at the time of this decision is frozen as the reference: no
new features land in it, only fixes needed to keep it a correct oracle. `docs/LAYERING-SPEC.md`
stays the contract, and the acceptance suite becomes language-neutral so the same tests run
against either binary. Cutover happens only when the Rust binary passes the whole suite and
produces byte-identical generated output for every fixture. The plan is doc-5; the work is
milestone m-7.

## Consequences

Feature work (TASK-50.3 to TASK-50.6 and the rest of the roadmap) pauses until cutover and then
continues in Rust against the same spec. The npm release pipeline (TASK-4, TASK-15, TASK-16) is
replaced by a Rust distribution (prebuilt binaries, with an npm wrapper so the install command
keeps working). The personal layer's default "TypeScript on Node.js for new projects" changes
for tooling; that is an edit in the personal layer repository.
