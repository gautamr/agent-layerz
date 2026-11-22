---
id: TASK-55.2.2
title: 'R1.2: Write the spec''s cache names, hashes and limits as test-side code'
status: To Do
assignee: []
created_date: '2026-11-22T17:55:00Z'
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
- [ ] #1 No file in test/acceptance or test/account-workspace.ts imports src/core/yaml, src/core/files, src/resolve/cache-paths, src/resolve/layer-hash, src/resolve/reader-load or ENV_NAMES (shown by the R1.1 guard list, which shrinks as these imports go, or by grep)
- [ ] #2 test/reference/spec-formulas.test.ts passes: every test-side formula equals its src/ counterpart on the table of inputs, and each formula's comment names a LAYERING-SPEC section
- [ ] #3 `mise run test` passes with the same number of cases as the R1.1 baseline, plus the new reference cases
<!-- AC:END -->
