---
id: TASK-104
title: Specify the adapter contract and open target identifiers in the spec
status: To Do
assignee:
  - '@architect'
created_date: '2026-11-25T19:05:00Z'
updated_date: '2026-11-25T19:10:00Z'
labels:
  - area:adapter
  - needs:owner-approval
milestone: m-8
dependencies: []
references:
  - decision-40
  - docs/LAYERING-SPEC.md
priority: high
ordinal: 205390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
decision-40 (proposed) turns spec §8's promise that new targets need no core changes into a contract. Once the owner accepts decision-40, the spec states it before any code moves. Out of scope: autonomy (doc-7) and the WASM host. User-facing: the targets grammar and an error text change, so a minor version bump.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 LAYERING-SPEC.md has '### 8.0 Adapter contract' stating the model version, the render signature, the output kinds, the territory checks and the coverage report (grep)
- [ ] #2 §5.2 states the target id grammar, and §8 says a well-formed id without an adapter gives 'no adapter for' while codex keeps its current message (grep)
- [ ] #3 §10.1 states the adapter code rule (<id>-<word> or a shared adapter code) and the 'target' errors of the output checks (grep)
- [ ] #4 doc-6 §2 no longer calls the target trait sealed and lists alz-adapter and alz-target-claude (backlog doc view doc-6 | grep)
- [ ] #5 The spec status line names the minor version bump these changes need (grep)
<!-- AC:END -->

## Comments

<!-- COMMENTS:BEGIN -->
author: @architect
created: 2026-11-25T19:10:00Z

---

Owner approval needed before this starts (decision-40 is proposed). Questions, with recommendations:
1. Accept decision-40 as written? It amends decision-23 (more crates) and replaces doc-6 §2's sealed trait. Recommended: accept.
2. A security item the adapter reports as not expressed (coverage report): warning, or build error? Recommended: an error for Claude, a warning `unenforced` for targets whose facts show no way to express it (Codex deny rules today).
3. Native sections for an id with no adapter and not in targets: kept silently (recommended), or a warning.
4. alz doctor for third-party adapters: show the declared verified range and never run a program a component names (recommended), or run `<tool> --version`.
5. Facts files: split docs/target-facts.md per target when the Codex adapter lands (recommended), or now.
6. Order: TASK-104 to TASK-112 before the first Rust release, because the targets grammar is user-facing; WASM after (recommended).
Blocked: TASK-105 onward on the adapter track.

---
<!-- COMMENTS:END -->
