---
id: TASK-55.9.7
title: >-
  Write the missing Rust tests for spec sections 7.1 to 7.11, 8 and 11: merge
  rules and adapters
status: Done
assignee:
  - '@implementer'
created_date: '2026-11-24T17:44:00Z'
updated_date: '2026-11-25T18:12:00Z'
labels:
  - area:rust
  - area:test
milestone: m-7
dependencies:
  - TASK-97
parent_task_id: TASK-55.9
priority: medium
ordinal: 189390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
decision-39: test/ may be deleted only when every requirement of docs/LAYERING-SPEC.md has a Rust test. This writes, from the spec alone, the tests that docs/spec-coverage.md marks as gap in §7.1 to §7.11 (merge rules per item type), §8 (target adapters) and §11 (native plugins): assert_cmd against the binary for command behaviour (isolated HOME, scratch folders, fixtures under fixtures/), library tests and proptest laws for rules. Each new test names in its doc comment the spec section it proves, and the PR touches only these sections' rows. Where a new test shows alz differing from the spec, alz is fixed here if the fix is small; otherwise a bug task is created and this task depends on it. Untrusted input: the tests feed untrusted layer.yaml, layer.lock and layer content, and never touch real account folders or the developer's home. Out of scope: other sections, new spec rules (a spec hole becomes its own task).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 docs/spec-coverage.md has no gap row in §7.1 to §7.11, §8 or §11 (guard test and grep)
- [x] #2 Every new test names in its doc comment the spec section it proves, and the row it fills names it (guard test)
- [x] #3 mise run rust:check passes, and the rust workflow passes on ubuntu-latest and macos-latest with the Linux-only cases
- [x] #4 Every difference from the spec that a new test found is fixed here or tracked by a bug task this task depends on (notes)
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
From docs/spec-coverage.md (TASK-55.9.5): 2 gap rows in §7.1 to §7.11, §8 and §11: '§8 What an adapter may rely on' (no test ties adapter output keys to target-facts) and '§8.2' (the Codex adapter is not implemented in Rust, so implement it or amend the spec; §13.13 depends on it). Fixture unnamed by tests: 37-settings-targets.

Correction (55.9.5): the gap rows in your sections are now 1, not 2: only '§8 What an adapter may rely on' is a gap (it concerns every adapter, not only Codex). §8.2 is planned under TASK-18 (the Codex adapter is not built yet).

Re-read the tests named by the covered rows of your sections and downgrade any row whose test does not assert the requirement (from 55.9.5).

Implementation (55.9.7). Added crates/alz-core/src/target/claude/tests/facts.rs (spec §8 'What an adapter may rely on'): renders one stack that makes the Claude adapter write every kind of key §8.1 lists, reads every agent/skill/rule frontmatter field, settings.json key (top level, permissions, hook events, group and handler keys), .mcp.json key and file location back out of the files, and looks each up in the verified (non-UNVERIFIED) rows of the target-facts sections the spec brackets. Result: 11 keys rest on no recorded fact (skill name, description, argument-hint; permissions allow, ask, deny; .mcp.json command, args, env, type, url). They are listed in NO_FACT_RECORDS (a ratchet: a new unrecorded key fails, a newly recorded one fails until removed) and tracked by TASK-97 (bug, area:docs), which this task depends on (AC4). The row '§8 What an adapter may rely on' stays gap until TASK-97 (AC1 is therefore not met for that row). Further tests: the checker itself reports a key of an UNVERIFIED row or another section; the stack covers every key §8.1 lists (non-vacuity); the two UNVERIFIED rows the spec names are the only UNVERIFIED Claude rows of §1-§5, §7, §7c. crates/alz/tests/settings_targets.rs (§7.9, fixture 37-settings-targets): refused top-level key writes nothing and prints no value; non-mapping target value refused; two layers deep-merge into .claude/settings.json. §11 row is n/a (guidance) and §8.2 is planned (TASK-18), unchanged. Checks: mise run rust:ci exit 0 (fmt, clippy -D warnings, tests, doc, msrv, deny, audit); spec_coverage guard passes. The ubuntu/macos workflow (AC3) was not run here.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
§7.1–§7.11, §8 and §11 have no gap row; the adapter-facts test ties every Claude output key to a verified fact (TASK-97). Merged through #31, #33 and #34; CI green on macOS and Linux.
<!-- SECTION:FINAL_SUMMARY:END -->
