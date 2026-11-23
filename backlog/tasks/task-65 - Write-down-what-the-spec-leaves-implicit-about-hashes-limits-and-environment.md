---
id: TASK-65
title: Write down what the spec leaves implicit about hashes, limits and environment
status: Done
assignee:
  - '@architect'
created_date: '2026-11-22T19:20:00Z'
updated_date: '2026-11-23T05:01:00Z'
labels:
  - area:spec
milestone: m-7
dependencies: []
references:
  - docs/LAYERING-SPEC.md
priority: high
ordinal: 81390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Found in R1.2 (TASK-55.2.2): the test-side formulas had to read src/ for five things the spec does not say. Clarify the current behaviour in LAYERING-SPEC with no behaviour change, before R2 ports the model. (1) §5.3 content hash byte layout: per file in order, path, NUL, sha256 hex of the bytes, LF, hashed as one text; the sha256: prefix in layer.lock; '.env files' means names starting with .env. (2) §7.13 the YAML alias limit's number (100), and that decision-28's limits supersede it for the Rust port where they differ. (3) §7.13 the full list of environment variables resolution reads. (4) Whether the 2000-entry limit counts links and skipped names. (5) XDG_CACHE_HOME empty or relative for the build cache is TASK-53's, not this task's.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Each of points 1 to 4 is stated in LAYERING-SPEC, matching the TypeScript reference or, where it differs, naming the reference-drift task (decision-30)
- [x] #2 The test-side formulas in test/spec/ cite those sections, and their notes no longer say the spec is silent
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
From the R1 review (2026-11-23): the spec names no diagnostic (code, wording) for a failed fetch of a remote source; acceptance 26 can assert only 'error [source]' plus the source named. Specify it.

From TASK-64 planning (2026-11-23): the spec does not say what alz does with a file name that is not valid UTF-8 (unverified what the TypeScript reference does). Settle it here. decision-30: where the TypeScript reference has a bug, the spec states the intended behaviour (the Rust port follows it), not the bug.

Spec text written (2026-11-23, architect). Not committed; ACs not checked.

### docs/LAYERING-SPEC.md
- §5.3 'Byte layout': one record per file in name order (path, NUL, lowercase sha256 hex of the bytes, LF), joined and hashed as UTF-8; layer.lock contentHash is 'sha256:' plus the hex, compared as exact text. Matches src/resolve/layer-hash.ts and src/model/lock.ts.
- §5.3 '.env files': a file whose own name starts with .env (.env, .env.dev, .envrc; not a file inside a folder named .env). Build error env-file; resolution miss layer-invalid after the content check. Matches src/core/files.ts isEnvFile and reader-load.ts.
- §5.3 'File names' and §6 tree bullet: every name the walk lists, and every path a pinned tree lists, must be valid Unicode (OsStr::to_str); otherwise build error layer naming the source and never the name, and resolution miss layer-invalid. Drift: TASK-69 (new). The reference uses readdirSync's utf8 decoding (U+FFFD), then cannot open the file. This was read from Node's behaviour, not run on Linux. SMOKE: APFS refuses a 0xFF name with EILSEQ (errno 92). The reference's tree2@ handling of such a path was not checked.
- §6 'A failed fetch': error source, message "cannot fetch '<source>': <line>". The line is git's first non-blank stderr line, trimmed. Both parts are stripped per decision-26 class 1, with no ': ' when git wrote nothing. Drift: TASK-68 (new). src/resolve/sources.ts prints both unstripped.
- §7.13 'No fetch on resolve': the 2000-entry count covers files, folders, links, other entries and .env files. The skipped names, everything below them, and the layer directory do not count. The walk stops at entry 2001. The 16 MiB counts only the files read. Matches listFiles (seen counted after the SKIPPED check) and readLayerForResolution.
- §7.13 'The YAML alias limit': at most 100 aliases per document, plus decision-28's depth, node and replay budgets. Drift: TASK-62 (note appended there). SMOKE with yaml 2.9.1: maxAliasCount 100 counts per anchor. 99 uses of one anchor parse, 100 are refused, and 51+50 uses over two anchors parse.
- §7.13 'Environment': values of HOME plus the seven ENV_NAMES. __MISE_DIFF and DIRENV_DIFF are checked for existence only. No other variable has an effect. An unset HOME means the user database (os.homedir()). Matches src/cli/io.ts and src/resolve/account-resolution.ts.
- XDG_CACHE_HOME for the build stays TASK-53's.

### Board
- New bugs: TASK-68 and TASK-69 (m-7, discovered, area:rust). TASK-55.4 (R3) now depends on them, next to its five existing dependencies.

### Tests
- Comments only, no behaviour change: test/spec/content-hash.ts, test/spec/limits.ts and test/spec/cache.ts cite the new sections, and none of them says the spec is silent any more. In test/acceptance/26, the comment on the remote fetch case cites §6 and TASK-68.
- No drift cases written. They are left to R3 with TASK-68 and TASK-69 (their ACs name the cases).
- test/spec ENV_NAMES still lists seven names. HOME is passed separately as the home directory, as the comment says.

### Checks
- mise run lint: ok (biome checked 178 files, tsc clean).
- npx vitest run test/reference/spec-formulas.test.ts: 32 passed, 1 expected fail.
- mise run test: 46 files, 1081 passed, 4 expected fail, 1 skipped, exit 0.

### User-facing
These rules change what users see only at cutover. That is covered by the minor 0.x bump decision-28 names. No version bump in this change.

Review fixes (2026-11-23, architect; no blocker found, every match with src/ confirmed). Not committed.

### Should-fix
1. §6 'A failed fetch': exact five-step rule for <line>. Decode with U+FFFD, split on LF only, strip (decision-26 class 1), trim Unicode White_Space, take the first non-empty line. The TASK-68 drift list names how the reference differs: whole-text JS trim (also removes U+FEFF) then the first line, so CR and trailing white space stay; no stripping; a trailing ': '.
2. Same section: names the three git steps (clone; rev-parse of the ref, or the lock's sha when the lock has an entry; checkout). States 'no location'. Read and rename failures after git are error cache ('cannot store <source> in the layer cache at <cache directory>'), or the §5.3 name error. The reference reports them as 'cannot fetch' with an empty detail.
3. §5.3 'File names': the tree case is separate. tree-cache.ts:90 decodes ls-tree -z with U+FFFD and reads blobs by id, so the reference probably writes and hashes the U+FFFD name (marked not run).
4. §5.3: fixed order of checks. Resolution: entry limit (unreadable), invalid name (layer-invalid), reading within 16 MiB (unreadable), content hash, .env, layer.yaml. Build: name check first. Recorded that the reference gives unreadable in resolution. The order was added to TASK-69's ACs.
5. §7.13 'YAML limits' (renamed from 'The YAML alias limit'): decision-28's rule for every YAML file. Strict UTF-8, 1 MiB cap (whole file for frontmatter), the four budgets. TASK-62 notes now say its AC #1 is met by this text and what is left is the Rust entry point and drift. Its ACs were not changed.
6. TASK-68 ACs added: empty stderr, blank first lines, CR inside a line, non-UTF-8 bytes, the rev-parse path, and the cache-step error.

### Nits
1. Alias drift corrected: count starts at 1 and the weight is the max over items (0 for an empty collection). SMOKE with yaml 2.9.1: '&a []' and '&a {}' with 1000 uses parse; a scalar or list anchor refuses its 100th use.
2. Aliases are counted as written. Depth is defined as nested mappings and sequences ('a: [x]' has two). Node and replay events are defined. R2 tests that serde-saphyr's options match.
3. An unset HOME means the user database entry for the effective uid. With no entry, alz acts as for a HOME that is not absolute (fail closed). The reference crashes: new TASK-72, which R6 (TASK-55.7) now depends on.
4. The invalid-name message is quoted.
5. 'Nothing else is printed' is narrowed to git's stdout and stderr. ssh can prompt on /dev/tty: new TASK-70 (needs:owner-approval, since BatchMode or setsid breaks passphrase keys without an agent).
6. §5.2 'Credentials in a source' (new): refuse userinfo in http(s) URLs and a password in any other URL or scp form. A user name stays allowed for ssh. The message never prints the source. Drift: new TASK-71.
7. §5.3: 'not hashed' (hash) vs 'lists' (walk).

### Board
TASK-55.4 (R3) now also depends on TASK-70 and TASK-71.

### Files
docs/LAYERING-SPEC.md; test/spec/limits.ts (comment cites §7.13 'YAML limits').

### Checks
- mise run lint: ok (178 files).
- npx vitest run test/reference/spec-formulas.test.ts: 32 passed, 1 expected fail.
- git diff --check: clean.
- mise run test was not rerun: since the earlier full run, only comments changed in test/.

Owner (2026-11-23): credentials rule (§5.2, TASK-71) accepted as written; ssh and git never prompt (TASK-70, spec §6 updated by the lead); files inside a folder named .env stay readable and hashed, as written.

Verified by the lead (2026-11-23): src/ identical to main; mise run test 46 files, 1081 passed, 4 expected fail, 1 skipped; mise run lint passes; no 'silent' note left in test/spec/. AC #1 reworded for decision-30 (point 2, the alias limit, is reference drift, TASK-62). Reviewed by the reviewer; all six should-fixes and seven nits applied.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
LAYERING-SPEC now states what it left implicit: the content-hash byte layout and the .env rule (§5.3), file names that are not valid UTF-8 and the order of checks (§5.3), credentials in a source URL (§5.2), the failed-fetch diagnostic with an exact rule for git's line and no prompting (§6), the YAML limits for every file alz reads, the environment variables resolution reads and what the 2000 entries count (§7.13). Where the TypeScript reference differs, the spec names the drift task fixed in Rust (TASK-62, TASK-68 to TASK-72). test/spec/ cites the sections.
<!-- SECTION:FINAL_SUMMARY:END -->
