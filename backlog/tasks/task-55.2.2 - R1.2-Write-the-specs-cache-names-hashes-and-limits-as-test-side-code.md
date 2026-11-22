---
id: TASK-55.2.2
title: 'R1.2: Write the spec''s cache names, hashes and limits as test-side code'
status: Done
assignee:
  - '@implementer'
created_date: '2026-11-22T17:55:00Z'
updated_date: '2026-11-22T19:20:00Z'
labels:
  - area:test
  - area:rust
milestone: m-7
dependencies:
  - TASK-55.2.1
references:
  - docs/LAYERING-SPEC.md
  - test/account-workspace.ts
parent_task_id: TASK-55.2
priority: high
type: task
ordinal: 72390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Fixture code in the acceptance path imports src/ to learn what the spec already defines. That makes the TypeScript code agree with itself rather than with the spec.

- New test/spec-formulas.ts (or a test/spec/ folder of small files), written from docs/LAYERING-SPEC.md and not from src/:
  - the cache root (§6);
  - the git cache key `git-<sha256 of what is cloned>` and its entry `<key>@<sha>` (§6);
  - `tree2@<tree>` and `pathref@<sha256 of the real dir>-<sha>` (§7.13);
  - the layer content hash (§5.3 "What the content hash covers");
  - the GitHub clone URL;
  - the reader limits (1 MiB, entry and byte limits) and the env-name list, written out as 32 already does with AGENT_SESSION_MARKERS.
  Each item names its spec section in a comment.
- YAML that tests write as input (allowed.yaml, accounts.yaml) and read back for inspection uses the `yaml` package directly, or literal text. A test that asserts the exact bytes alz wrote compares against literal expected text.
- Replace the src/ imports used for fixtures in test/account-workspace.ts (stringifyYaml, gitEntryName, hashLayerDir), 28 (`keyOf` and gitCacheKey), 30 (entry names, hashLayerDir, sha256, MAX_*, cacheRoot), 31 (sha256, ENV_NAMES) and 32 (parseYamlLimited where it only reads back allowed.yaml).
- test/reference/spec-formulas.test.ts checks each test-side function against the src/ function on a table of inputs, so a drift between the spec copy and the reference shows up while both exist.
- Out of scope: moving unit cases (R1.3 to R1.6). The `Terminal` type import in account-workspace stays until R1.6.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 No file in test/acceptance or test/account-workspace.ts imports src/core/yaml, src/core/files, src/resolve/cache-paths, src/resolve/layer-hash, src/resolve/reader-load or ENV_NAMES for fixtures or read-backs (shown by grep); the one remaining import, readRegularFileWithId in 31 for the O_NONBLOCK trace case, moves with that case in R1.5 (TASK-55.2.5)
- [x] #2 test/reference/spec-formulas.test.ts passes: every test-side formula equals its src/ counterpart on the table of inputs, and each formula's comment names a LAYERING-SPEC section
- [x] #3 `mise run test` passes with the same number of cases as the R1.1 baseline, plus the new reference cases
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Implementation (implementer, 2026-11-22).
Files: new test/spec/cache.ts (sha256Hex, cacheRoot, githubCloneUrl, gitCacheKey, gitEntryName, treeEntryName, pathIndexName; spec 6 and 7.13), test/spec/content-hash.ts (contentHash, hashedFiles; spec 5.3), test/spec/limits.ts (MAX_LAYER_ENTRIES 2000, MAX_LAYER_BYTES 16 MiB, MAX_READ_FILE_BYTES 1 MiB, YAML_ALIAS_LIMIT, ENV_NAMES; spec 7.13); new test/reference/spec-formulas.test.ts (drift test, 33 cases incl. 1 expected fail). Edited test/account-workspace.ts, 28, 30, 31, 32 to use them (yaml package directly for stringify and parse of allowed.yaml; literal text where bytes are asserted was already the case).
Spec gaps (read from src/ to know the answer; not in LAYERING-SPEC):
1. 5.3 does not give the byte layout of the content hash: for each file in order, path, NUL, sha256 hex of its bytes, LF, hashed as one text. Taken from src/resolve/layer-hash.ts. Also silent on the 'sha256:' prefix the lock adds to contentHash. 5.3 says '.env files'; the rule used is names starting '.env' (9.1 and 7.6 write .env*), as in src/core/files.ts.
2. 7.13 names 'the YAML alias limit' without its number; 100 is from src/core/yaml.ts.
3. 7.13 never lists the variables resolution reads (ENV_NAMES); the list is the union of the names the text mentions (CLAUDE_CONFIG_DIR, CODEX_HOME, ALZ_SET_* markers, three XDG variables). It matches src.
4. 6 says the cache is $XDG_CACHE_HOME/agent-layerz; 7.13 says an empty or relative XDG_CACHE_HOME counts as unset for resolution. src cacheRoot (used by build) uses the raw value. The test-side cacheRoot covers set-absolute and unset only; the empty/relative build case is unspecified.
5. 7.13 limit '2000 files and folders': the spec does not say whether links, and the skipped names (.git, node_modules, .DS_Store), count; only the numbers are written test-side.
Divergence found (spec vs src): 5.3 says code point order; src/core/files.ts compareCodePoints uses '<' (UTF-16 unit order), so a layer with a file name outside the basic plane hashes differently from the spec. Recorded as it.fails in spec-formulas.test.ts (KNOWN DRIFT); the Rust port sorts by code point. Owner decision: fix src (fixes-only allowed) or change the spec to say UTF-16 order. Not changed here.
AC #1 not fully literal: 31 still imports readRegularFileWithId from src/core/files (the O_NONBLOCK-under-trace unit case, R1.5 scope; the task lists only sha256 and ENV_NAMES for 31). Every other listed import is gone from test/acceptance and test/account-workspace.ts (grep). The R1.1 guard list is unchanged: all 8 direct files still import other src modules, the 22 others reach src only through helpers.ts.
Checks: mise run test 880 passed, 1 expected fail, 1 skipped (882); acceptance 841 (840 passed, 1 skipped) as the R1.1 baseline; reference 8 baseline + 33 new = 41 (40 passed, 1 expected fail). mise run lint clean. mise run alz:check passed (68 generated files).

AC #1 reworded (2026-11-23): the task assigned 31's O_NONBLOCK trace case to R1.5, so its readRegularFileWithId import moves there. Orchestrator rerun: mise run test 33 files, 880 passed, 1 expected fail (KNOWN DRIFT, TASK-64), 1 skipped; lint clean; grep shows only that import left. Owner decisions: sort by code point and fix TypeScript (TASK-64); the five spec gaps go to TASK-65. Both block R2 (TASK-55.3).
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Spec formulas written from LAYERING-SPEC as test code in test/spec/ (cache root, GitHub clone URL, git cache key and entry, tree2 and pathref names, content hash, reader limits, env names), each citing its section. Fixture code in account-workspace, 28, 30, 31 and 32 uses them instead of src/; allowed.yaml is written and read with the yaml package. test/reference/spec-formulas.test.ts (33 cases) checks each against src/. Found: TypeScript sorts by UTF-16 unit, not code point (TASK-64, recorded as it.fails), and five spec gaps (TASK-65). Acceptance case count unchanged at 841.
<!-- SECTION:FINAL_SUMMARY:END -->
