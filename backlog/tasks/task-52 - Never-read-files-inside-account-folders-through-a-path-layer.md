---
id: TASK-52
title: 'Never read files inside account folders through a path: layer'
status: To Do
assignee: []
created_date: '2026-11-22T07:23:00Z'
updated_date: '2026-11-23T07:59:00Z'
labels:
  - discovered
  - area:security
  - area:resolve
  - area:rust
milestone: m-3
dependencies:
  - TASK-50.9
references:
  - docs/LAYERING-SPEC.md
priority: low
type: bug
ordinal: 29390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Found while planning TASK-50.9. LAYERING-SPEC §9.1 says alz never opens a file inside an account folder (~/.claude, ~/.claude.json, ~/.codex, ~/.claude-*, ~/.codex-*, or the folders CLAUDE_CONFIG_DIR and CODEX_HOME name). Today loadLayer lstats layer.yaml and hashLayerDir reads every file of an unpinned path: layer, so a stack entry whose directory is $HOME or above (with a layer.yaml there), lies inside an account folder (for example a repo layer that is a link into ~/.claude-work), or holds a CLAUDE_CONFIG_DIR folder makes build, check and the other resolving commands open files there. The same holds for a cache root that XDG_CACHE_HOME puts inside an account folder. TASK-50.9 adds touchesAccountFolder (src/resolve/account-folders.ts) and uses it in the account reader only; this task applies it to every command that loads a stack, as a build error naming the layer without its path content. Out of scope: account resolution itself (TASK-50.9, TASK-50.10).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 alz lint in a repository refuses an unpinned path: layer whose real directory is $HOME or above it, inside an account folder, or above a folder CLAUDE_CONFIG_DIR or CODEX_HOME names, before opening any file in it, with an error that names the layer (acceptance lint twin with a fake HOME, R3.11, and a Rust test with account folders made mode 000, R3.6)
- [ ] #2 A layer cache root inside an account folder is refused the same way (test)
- [ ] #3 The component walk that alz lint uses and account resolution will use (R3.5) resolves repository-named paths one component at a time and never lstats, readlinks or stats a name inside an account folder, even through a symlink planted in the repository (proptest on the walk, R3.5, and an alz lint twin, R3.11)
- [ ] #4 The alz build form of AC #1 is in the acceptance suite (written in R3.11 where it is missing) and is checked again against the Rust alz in R5 (TASK-55.6); account resolution's use of the walk is checked in R6 (TASK-55.7)
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
2026-11-22 from the TASK-50.9 security review (NOTE): the account reader's lexical account-folder check sees only the text, so a symlink planted in the repository (path:./alias or git:./alias@v1 pointing into ~/.claude-x) still makes realpath and gitTarget lstat, readlink and stat names inside an account folder before the real-path check; nothing is opened. Fix: resolve one path component at a time with lstat/readlink and check each resolved prefix with touchesAccountFolder before going deeper. Also normalise Unicode in namesAccountFolder (text.normalize('NFC').toLowerCase()) for non-ASCII homes on APFS. Covers the reader and build.

2026-11-22: paused by decision-21 (Rust port, doc-5). Built in Rust in TASK-55.4, where openat-relative access closes it properly.

Owner (2026-11-23, decision-30): src/ is never changed; this bug is fixed only in the Rust alz, in R3 (TASK-55.4, sources and the cache). Its acceptance cases assert the spec and are marked reference drift (test/drift.ts) where the TypeScript reference fails them. R4 depends on this task, so it closes within R3.

Architect (2026-11-23, owner answer 7 in TASK-55.4, delegated by the lead): criteria reworded to the lint twins that R3.11 adds; the build, check and account-resolution forms are checked again in R5 (TASK-55.6) or R6 (TASK-55.7), as the last criterion says. The other criteria are unchanged.
<!-- SECTION:NOTES:END -->
