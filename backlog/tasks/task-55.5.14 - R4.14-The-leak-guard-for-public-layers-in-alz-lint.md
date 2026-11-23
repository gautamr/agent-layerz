---
id: TASK-55.5.14
title: 'R4.14: The leak guard for public layers in alz lint'
status: To Do
assignee:
  - '@implementer'
created_date: '2026-11-23T12:28:00Z'
updated_date: '2026-11-23T12:39:00Z'
labels:
  - area:rust
  - area:security
milestone: m-7
dependencies:
  - TASK-55.5.13
  - TASK-55.5.1
parent_task_id: TASK-55.5
priority: high
type: task
ordinal: 127390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Plan for this step (§9.2 as R4.1 writes it; owner answer 2 and decision-38).
- alz-core `leak_guard.rs`, for a layer with visibility public. It checks every file of LayerContents (layer.yaml without its forbid_patterns, as the reference does) against three things:
  - the layer's forbid_patterns;
  - the secret scan of R4.11;
  - the email and private-host heuristics, which give the warnings `leak-email` and `leak-host`.
- forbid_patterns compile with regex-lite (decision-38), with size limit 64 KiB and nest limit 32. A layer has at most 64 entries, each at most 512 characters. Before compiling, a look-around (`(?=`, `(?!`, `(?<=`, `(?<!`) or a backreference (`\1` to `\9`, `\k<`) is found and refused with a message that says these are not supported. Every refusal is `leak-forbid` and names the entry by its index, never its text.
- Matches use is_match only. A diagnostic names the file and the pattern's index, never the matched text.
- alz lint runs the guard on each public layer, in both modes.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Test 10's forbidden-pattern and fake-token cases pass against the Rust binary (mise run test:acceptance with ALZ_BIN on test 10)
- [ ] #2 A forbid_patterns entry with look-around or a backreference is refused with leak-forbid and a message naming the feature; one that does not compile, is longer than 512 characters, passes the size or nest limit, or is the 65th entry is refused naming its index; no diagnostic holds the entry's or the matched text (unit tests)
- [ ] #3 The email and private-host heuristics warn as leak-email and leak-host, as the reference does (unit tests)
<!-- AC:END -->
