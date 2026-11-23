---
id: TASK-55.4.4
title: 'R3.4: The failed-fetch line and the resolution diagnostics as pure values'
status: In Progress
assignee:
  - '@implementer'
created_date: '2026-11-23T07:42:00Z'
updated_date: '2026-11-23T08:04:00Z'
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
- [ ] #1 FetchFailureLine follows spec §6 steps 1 to 5: git wrote nothing, blank or whitespace-only first lines, a CR inside a line, and bytes that are not UTF-8 each give the spec's result (unit tests; TASK-68 AC #3)
- [ ] #2 No rendered source, cache or layer diagnostic holds a control or format character or a raw source, and cannot fetch has no trailing ': ' without a line (proptest laws)
- [ ] #3 The cache store failure has its own message, distinct from cannot fetch (unit test; TASK-68 AC #4's message)
<!-- AC:END -->
