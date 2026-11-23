---
id: TASK-68
title: >-
  A failed fetch prints the source and git's first line stripped of control
  characters
status: To Do
assignee: []
created_date: '2026-11-23T04:40:00Z'
updated_date: '2026-11-23T04:52:00Z'
labels:
  - discovered
  - area:rust
  - area:security
milestone: m-7
dependencies: []
references:
  - docs/LAYERING-SPEC.md
  - src/resolve/sources.ts
priority: medium
type: bug
ordinal: 85390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Found in TASK-65. Spec §6 step 1 'A failed fetch' states the diagnostic for a github: or git: source that cannot be cloned or whose ref does not resolve: error code source, message cannot fetch '<source>': <first non-blank line of git's standard error, trimmed>, both stripped of control and format characters (decision-26 class 1), and no ': ' when git wrote nothing. git's standard error can carry what a remote server sent, so it is untrusted. The TypeScript reference (src/resolve/sources.ts, cloneGit catch) prints the source and git's line unstripped and ends the message with ': ' when git wrote nothing. decision-30: src/ is never changed; this is fixed only in Rust, in R3 (TASK-55.4), and its acceptance cases are marked reference drift. Out of scope: the wording of 'cannot read <source> at <ref>' for pinned path: layers, which already strips both.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The Rust alz prints error [source] with cannot fetch '<source>' and git's first non-blank line, both stripped per decision-26 class 1, and no trailing ': ' when git wrote nothing (acceptance test 26)
- [ ] #2 Acceptance 26 asserts the whole message for the remote case, and a referenceDrift case with a fake ssh that writes ESC, CR and U+202E to standard error shows that the output holds none of them
- [ ] #3 Acceptance 26 or Rust tests cover: git writes nothing (the message ends after the quoted source, no ': '); blank or whitespace-only first lines (the first line left after steps 1-5 of spec §6 'A failed fetch' is used); a CR inside a line (removed, the line does not split there); bytes that are not valid UTF-8 (shown as U+FFFD, never raw); and a ref that does not resolve to a commit (the rev-parse step, not the clone), each with error [source] and no location
- [ ] #4 A failure to move the clone into the cache or to read it for the hash comparison gives error [cache] with the §6 message 'cannot store ... in the layer cache at ...', not 'cannot fetch' (Rust test)
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
TASK-65 review (2026-11-23): §6 'A failed fetch' now fixes the order to get <line>: decode with U+FFFD, split on LF only, strip (class 1), trim White_Space, take the first non-empty line. It covers three git steps: clone, rev-parse of the ref or the lock's sha, and checkout. Read or rename failures after git are error cache. The reference differs in each of these points (spec drift list); src/resolve/sources.ts:203-224, detail at line 223.
<!-- SECTION:NOTES:END -->
