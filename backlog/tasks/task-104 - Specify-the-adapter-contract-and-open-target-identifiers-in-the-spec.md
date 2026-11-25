---
id: TASK-104
title: Specify the adapter contract and open target identifiers in the spec
status: In Progress
assignee:
  - '@architect'
created_date: '2026-11-25T19:05:00Z'
updated_date: '2026-11-25T19:53:00Z'
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

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
Spec-only change (no production code, no tests). docs/LAYERING-SPEC.md: status line names the minor bump (0.1.0 to 0.2.0, TASK-55.8.4); §5.2 Target ids (grammar ^[a-z][a-z0-9-]{0,31}$, where ids are read from TASK-106, account families stay closed); §4.1, §7.8, §7.9, §7.11 point at it; §7.6 Targets that cannot enforce an item (unenforced field, TASK-127); §8 Targets without an adapter (no adapter for, codex keeps its message); §8.0 Adapter contract (Model, Descriptor, Rendering with the Rust trait, Output kinds, Output checks, Coverage report, Security items a target cannot enforce); §8.1 Claude territory; §10.1 Adapter codes and the target row. No new diagnostic code: every new failure is the existing code 'target' (or 'permissions' for the unenforced schema), so the diagnostic_codes guard is unchanged. docs/spec-coverage.md: one row per new anchor, planned with TASK-105..112 or TASK-127. doc-6 §2 and §4 through backlog doc update. Untrusted inputs named by the spec: layer.yaml targets, settings/models/targets sections for any id, permissions.yaml unenforced, adapter output (paths, codes, messages). Target facts: none relied on (alz-internal contract). Checks: cargo test -p alz-test-support --test spec_coverage, mise run rust:check, AC greps.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Owner (2026-11-26): decision-40 accepted. When an adapter cannot express a security item (for example a decision class's rules on a target without allow/ask/deny lists), alz fails the build with an error naming the item and the target, never a warning; a layer may opt that target out of the item explicitly.

Evidence (2026-11-26, @architect, branch docs/adapter-contract-spec-task-104):
- AC1: grep -n '^### 8.0 Adapter contract' docs/LAYERING-SPEC.md -> 1 hit; bold leads Model. (model 1.0, major.minor, security items), Descriptor., Rendering. (pure function, Context, Rust trait with descriptor() and render(&Model, &Context) -> Output), Output. (Json, Markdown, Toml, Bytes), Output checks. (territory, core paths, limits), Coverage report., Security items a target cannot enforce. (error naming item and target, opt-out via §7.6 unenforced).
- AC2: grep '^\*\*Target ids\*\*' and '^[a-z][a-z0-9-]{0,31}$' -> §5.2; grep 'no adapter for' -> §8 'Targets without an adapter', which keeps codex's 'not implemented yet (implemented: claude)' message.
- AC3: grep '^\*\*Adapter codes\.\*\*' and '<id>-<word>' -> §10.1; shared adapter codes models, tool-scope, hook-on-fail; the output-check failures are listed there and in the 'target' row. No new code was added: every new failure is 'target' (or 'permissions' for the unenforced schema), so crates/alz/tests/diagnostic_codes.rs needs no new case.
- AC4: backlog doc view doc-6 --plain | grep -c 'trait (sealed)' -> 0; it lists alz-adapter/ and alz-target-claude/ in §2, and §2's crate rationale and §4's anti-corruption row follow decision-40 (backlog doc update doc-6).
- AC5: the status line names the minor version bump, 0.1.0 to 0.2.0 (TASK-55.8.4), with a CHANGELOG entry.
- Owner decision applied: unexpressed security item = error 'target' at <layer>:permissions.yaml, never a warning; opt-out = permissions.yaml unenforced.<target>.deny / .floor_fields, own items only. Built by new task TASK-127 (depends on TASK-109).
- docs/spec-coverage.md: 12 new planned rows (§5.2 Target ids; §7.6 Targets that cannot enforce an item; §8 Targets without an adapter; §8.0 and its 7 leads; §10.1 Adapter codes.); summary all 158/24/22/0/204. cargo test -p alz-test-support --test spec_coverage: 9 passed.
- mise run rust:check: exit 0, 75 'test result: ok' lines, 0 FAILED (diagnostic_codes and spec_coverage guards included).
- Target facts: none relied on; the contract is alz-internal.
<!-- SECTION:NOTES:END -->

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
