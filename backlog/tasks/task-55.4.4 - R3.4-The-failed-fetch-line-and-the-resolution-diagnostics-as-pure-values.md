---
id: TASK-55.4.4
title: 'R3.4: The failed-fetch line and the resolution diagnostics as pure values'
status: Done
assignee:
  - '@implementer'
created_date: '2026-11-23T07:42:00Z'
updated_date: '2026-11-23T08:10:00Z'
labels:
  - area:rust
  - area:security
milestone: m-7
dependencies:
  - TASK-55.4.1
parent_task_id: TASK-55.4
priority: high
type: task
ordinal: 105390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Plan for this step. Fixes TASK-68 (AC #3 and the messages of AC #1 and AC #4) as pure code, which R3.8 wires.
- alz-core `git/failure.rs`: `FetchFailureLine::from_stderr(&[u8]) -> Option<Stripped>` follows spec §6 "A failed fetch" exactly:
  1. decode as UTF-8, with U+FFFD for each invalid sequence;
  2. split at LF only;
  3. strip each line (class 1);
  4. trim Unicode White_Space from both ends;
  5. take the first line that is not empty.
- `diagnostic.rs` gains the codes `source`, `cache`, `cache-index` (a warning), `layer` and `env-file`, with message constructors:
  - `cannot fetch '<source>': <line>`, with no `: ` when there is no line;
  - `cannot store '<source>' in the layer cache at <dir>`;
  - the §5.3 cache-integrity message "cached layer '<name>' (<source>) no longer matches layer.lock … delete that directory and rebuild";
  - "layer directory not found for '<source>'";
  - `cannot resolve ref <ref> for '<source>'`;
  - the §5.3 "File names" message;
  - the credentials message, which already exists.
  Each takes `Stripped` or validated values only, and none has a location.
- `Code::NotPorted` stays until R3.10 removes its last use.
Laws, as proptest:
- the line holds no control or format character and no LF;
- it is trimmed;
- it is `None` exactly when no line survives;
- a CR inside a line is removed and does not split it.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 FetchFailureLine follows spec §6 steps 1 to 5: git wrote nothing, blank or whitespace-only first lines, a CR inside a line, and bytes that are not UTF-8 each give the spec's result (unit tests; TASK-68 AC #3)
- [x] #2 No rendered source, cache or layer diagnostic holds a control or format character or a raw source, and cannot fetch has no trailing ': ' without a line (proptest laws)
- [x] #3 The cache store failure has its own message, distinct from cannot fetch (unit test; TASK-68 AC #4's message)
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Implemented in an isolated worktree at fa4aaec and merged by the lead (2026-11-23); git.rs combined with R3.2's (mod environment; mod failure). The implementer's rust:ci and lint exit 0 there; after the merge cargo test -p alz-core gives 309 passed. FetchFailureLine (newtype over Stripped, proof the line is stripped, trimmed and non-empty) follows §6 steps 1–5; constructors in stack::resolution_failure (cannot_fetch, cannot_store, cache_differs_from_lock, layer_directory_not_found, cannot_resolve_ref, file_name_not_utf8, source_holds_credentials) take only Stripped and validated types. AC1 unit tests (empty, blank, control-only, CR inside a line, non-UTF-8, U+2028/U+2029), AC2 property tests (no control or format character, never ': ' at the end, raw source never printed), AC3 cache-store failure is code cache and not 'cannot fetch'. Code gains cache, cache-index, env-file. Spec gap for R3.11: the exact texts of the cache-integrity, 'layer directory not found' and 'cannot resolve ref' messages are not in the spec (decision-26: the spec owns every message); R3.11 adds them to §6/§10.1.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
The failed-fetch line is a type, FetchFailureLine, built from git's standard error by the five steps of spec §6 (decode, split on LF, strip, trim, first non-empty), and the resolution diagnostics are constructors that take only stripped text and validated names, so no raw source reaches a message; property tests show every rendered line is clean and never ends in ': '. A cache store failure is its own cache error.
<!-- SECTION:FINAL_SUMMARY:END -->
