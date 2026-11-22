---
id: TASK-52
title: 'Never read files inside account folders through a path: layer'
status: To Do
assignee: []
created_date: '2026-11-22T07:23:00Z'
updated_date: '2026-11-22T12:55:00Z'
labels:
  - discovered
  - area:security
  - area:resolve
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
- [ ] #1 alz build refuses an unpinned path: layer whose real directory is $HOME or above it, inside an account folder, or above a folder CLAUDE_CONFIG_DIR or CODEX_HOME names, before opening any file in it, with an error that names the layer (test with a fake HOME and a file-access trace)
- [ ] #2 A layer cache root inside an account folder is refused the same way (test)
- [ ] #3 Account resolution resolves repository-named paths one component at a time and never lstats, readlinks or stats a name inside an account folder, even through a symlink planted in the repository (trace test)
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
2026-11-22 from the TASK-50.9 security review (NOTE): the account reader's lexical account-folder check sees only the text, so a symlink planted in the repository (path:./alias or git:./alias@v1 pointing into ~/.claude-x) still makes realpath and gitTarget lstat, readlink and stat names inside an account folder before the real-path check; nothing is opened. Fix: resolve one path component at a time with lstat/readlink and check each resolved prefix with touchesAccountFolder before going deeper. Also normalise Unicode in namesAccountFolder (text.normalize('NFC').toLowerCase()) for non-ASCII homes on APFS. Covers the reader and build.

2026-11-22: paused by decision-21 (Rust port, doc-5). Built in Rust in TASK-55.4, where openat-relative access closes it properly.
<!-- SECTION:NOTES:END -->
