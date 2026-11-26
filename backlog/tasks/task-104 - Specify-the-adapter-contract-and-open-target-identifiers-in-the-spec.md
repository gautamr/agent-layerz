---
id: TASK-104
title: Specify the adapter contract and open target identifiers in the spec
status: In Progress
assignee:
  - '@architect'
created_date: '2026-11-25T19:05:00Z'
updated_date: '2026-11-26T04:00:00Z'
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

Owner answers (2026-11-26, relayed by the coordinator): the spec is approved. Only the layer that set an item may opt a target out of it (as written). TASK-127 is needed before the Codex adapter only, not 0.2.0: TASK-126 now depends on TASK-127 (deps TASK-112, TASK-115, TASK-119, TASK-127), and TASK-55.8.4 is unchanged. Locked hooks and the default_mode floor are security items too.
Amendment (second commit, @architect): §8.0 Model. lists four kinds of security item (deny rules, set floor fields, the default_mode floor, locked hooks that run on the target). §7.6 unenforced gains default_mode: true (only in the layer whose floor is in force) and hooks: [ids] (the layer's own locked hooks), with the same own-items rule and a 'permissions' error for anything else. §8.0 Security items a target cannot enforce gives a message for each of the four kinds. §7.5 says a locked hook is a security item rather than TASK-101's warning. The status line says unenforced ships after 0.2.0 with the next minor version. No new anchor, so no new matrix row; TASK-127's ACs were rewritten for the four kinds, and TASK-105 and TASK-109 got comments. cargo test -p alz-test-support --test spec_coverage: 9 passed.

Owner (2026-11-26, relayed): a locked hook with on_fail: deny on an event where the target cannot block (Claude: stop, subagent_stop, post_tool) is a security item the target cannot enforce: the error 'target' naming the hook and the target, unless its layer opts it out with unenforced.<target>.hooks. This replaces the hook-on-fail warning for locked hooks only, from TASK-127; the warning stays until then. Lead defaults: no note for opted-out items; agent native sections stay closed to unknown ids; the limits 10000 files, 16 MiB per file, 256 MiB in all stay.
Amendment (third commit, @architect): §8.0 Model. (on_fail is part of a locked hook item; that case is not expressed from TASK-127), §8.0 Coverage report. (Claude reports it as not expressed from TASK-127), §8.1 hooks bullet and §7.5 (warning for unlocked hooks only from TASK-127; every hook keeps the warning until then), §10.1 hook-on-fail row reason, status line (TASK-127 turns that warning into an error for locked hooks, next minor version). No anchor changed, so no matrix row changed. TASK-109 AC #4 removed and re-added for all four kinds (unchecked). TASK-127 gains AC #5 (the locked on_fail case, with and without the opt-out, unlocked keeps the warning), and its Claude AC (now #6) was replaced to match. cargo test -p alz-test-support --test spec_coverage: 9 passed.
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
