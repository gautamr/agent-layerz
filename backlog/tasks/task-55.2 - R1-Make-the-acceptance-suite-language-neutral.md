---
id: TASK-55.2
title: 'R1: Make the acceptance suite language-neutral'
status: In Progress
assignee:
  - '@architect'
created_date: '2026-11-22T12:54:00Z'
updated_date: '2026-11-23T03:24:00Z'
labels:
  - area:rust
  - area:test
milestone: m-7
dependencies:
  - TASK-55.1
parent_task_id: TASK-55
priority: high
ordinal: 34390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
doc-5 phase R1. The suite runs against the binary named by ALZ_BIN (TypeScript bundle by default). The six source-importing files become CLI tests or move to TypeScript-only unit tests. Includes TASK-54: one temp root per run, removed at the end.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 With ALZ_BIN pointing at the TypeScript bundle the whole suite passes as before, no test imports src/ through the acceptance path, and a run leaves no alz-* folder in the temp folder
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
### Findings that shape the plan
- 30 of the 31 acceptance files run alz in process: `test/helpers.ts` `layers()` calls `run` from src/cli/main. Only 27 spawns, and it spawns the installed standalone bundle.
- Eight files import src/ directly, not six: 21, 24, 26, 28, 29, 30, 31 and 32. So do the helpers account-workspace.ts and allow-driver.ts. The reasons for each are in the notes, 2026-11-22 survey.
- The in-process forms that cannot survive a spawn:
  - the `vi.mock` fs and child_process trace (30, 31, 32);
  - source-graph scans of src/ (31, 32);
  - the fake `Terminal` (24, 32);
  - `updateAllowed` driven from 12 processes (32);
  - unit tables of internal functions (21, 26, 28, 29, 30, 31).

### Shape
- Two vitest projects:
  - `acceptance` (test/acceptance/**) is the language-neutral oracle; it imports nothing from src/ and drives the binary named by ALZ_BIN.
  - `reference` (test/reference/**, test/guards/**) holds TypeScript-only cases; they may import src/ and are deleted with src/ in R8.
  - `mise run test` runs both. No case is dropped: a moved case keeps its name and body, and gains a CLI counterpart in acceptance when the CLI can show the behaviour.
- Spec formulas that fixtures need (cache entry names, the content hash, limits, env names) are written in test code from LAYERING-SPEC. A reference test checks them against src/.
- Language-neutral proxies replace the trace in acceptance:
  - a before and after snapshot of HOME, the cache, the state folder and the repo (no writes);
  - a `git` shim on PATH that records calls, with a positive control (no git);
  - a FIFO swapped in for a file that is read (no blocking read).
  The exact read set and reads inside account folders stay reference-only; R3 and R6 cover them through alz-io capability types.

### ALZ_BIN
- An absolute path to an executable regular file, checked after realpath and never run through a shell. Anything else stops the run before the first test.
- When it is unset, the global setup builds src/cli/bin.ts with the `build:standalone` esbuild options into `<run root>/bin/alz`, mode 0755, with `#!<absolute node path>`, because 26 runs alz with `PATH=/nonexistent`.
- Children get exactly {HOME, PATH, ALZ_CLAUDE_BIN, TMPDIR, ...extra}, stdin ignored, their own process group, and a 15 s timeout that records a process listing.
- `mise run test:acceptance` runs only the oracle. Later Rust phases run `ALZ_BIN=$PWD/target/release/alz mise run test:acceptance -- <their share>`. The local `alz-rs` from TASK-55.1.6 works too. CI changes wait for those phases and the owner.

### TASK-54
- R1.7 does its AC #2: one `alz-suite-*` root per run, holding all test temp folders and the children's TMPDIR, removed by the teardown; stale roots are removed only under strict checks; the spawn timeout and listing.
- TASK-54 stays open for AC #1, the cause. It depends on R1.7, and its notes hold the hypotheses: a temp folder with tens of thousands of entries; the xcrun git shim and its cache in TMPDIR; a network wait.

### Subtasks (one PR each)
1. TASK-55.2.1: the project split, the in-process driver copied into test/reference, and an import guard with a list that must shrink (baseline counts recorded).
2. TASK-55.2.2: test-side spec formulas (depends on .1).
3. TASK-55.2.3: 21, 24 (resolver), 26, 28, 29 (depends on .2).
4. TASK-55.2.4: 30 (depends on .2).
5. TASK-55.2.5: 31 (depends on .4, reuses its proxies).
6. TASK-55.2.6: the fake-terminal, writer and scan cases of 24 and 32 (depends on .5).
7. TASK-55.2.7: the harness: ALZ_BIN, spawning, the run root, timeouts, the decision-26 assertion forms in 25, 27, 31 and 32, and the guard list emptied (depends on .3 to .6).
8. TASK-55.2.8: a pty driver, so the allow cases return to the oracle (needs:owner-approval; depends on .7).
9. TASK-55.2.9: the repository layer's skill, rules and map text (needs:owner-approval; depends on .7).

### Proof of AC #1
- R1.7 AC #1: the suite passes with ALZ_BIN unset, with case counts per project against the R1.1 baseline.
- R1.1's guard, with its list empty in R1.7: no acceptance file reaches src/ by any import, type-only included, transitively, by `import()` or `vi.mock` factories, or through `SRC`. It has positive controls.
- R1.7 AC #3: the alz-* listings of the temp folder before and after a passing run and a failing run are equal.

### Spec, output and version
None. Tests only: src/, generated output and the layer format do not change, so there is no version bump and no target fact.

### Untrusted inputs
- ALZ_BIN, from the environment of a developer or CI.
- The stale-root clean-up, which deletes. It is limited to `alz-suite-*` entries that are real directories directly in the temp folder, owned by the current uid, with a dead `pid`.
- Fixture content stays untrusted input to the binary and is passed as argument arrays.

### Risks
- Spawning roughly 2,000 processes makes the suite slower; R1.7 records durations.
- An agent-layer change (R1.9) and the pty driver (R1.8) wait for the owner. R1 can close without R1.8 only if the owner picks option (d).
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
2026-11-22 survey (architect), read against the code on feat/rust-r1-task-55.2.

How the suite runs alz today: in process, for every file but 27. `test/helpers.ts` imports `run` from src/cli/main and `layers()` calls it with fake stdout/stderr and an optional fake `Terminal`; `test/account-workspace.ts` (`alz()`) wraps `layers()`. So 30 of 31 acceptance files reach src/ through the helpers. Only 27 spawns: `sh scripts/install-local.sh`, then the installed standalone bundle.

Files that import src/ directly: eight, not six (doc-5 counted six). Why each one does:
- 21: `parseStackOption` (unit table of --stack forms).
- 24: `run` plus a fake `Terminal` (alz account allow with scripted answers, 2 cases); `resolveAccounts` (resolver internals, 2 cases).
- 26: `isInsideRepo` (predicate tables, 3 cases); `guardedClone` and `git` (proves the guard drops GIT_ALLOW_PROTOCOL).
- 28: `gitCacheKey` (fixture code: where the cache entry lives, plus a key test); `githubCloneUrl`; `verifyCachedLayer` and `lockedEntryFor` (unit cases).
- 29: `computeBuild` (reads the resolved config in process, 8 uses); `parseAccount`, `isAccountName`, `codexPatternMatches` and `evaluateDeclarations` (unit tables).
- 30: `readAccountDeclaration` and `recordedSource` (the reader, called directly under a `vi.mock` fs/child_process trace that proves it only reads and runs no git); `readManifest` and `cacheRoot` (inputs); `gitEntryName`, `treeEntryName`, `pathIndexName`, `hashLayerDir` and `sha256` (fixture code: cache entry names and lock hashes, all spec formulas §5.3, §6, §7.13); `evaluateAccount`, `mergeAccount` and `Diagnostics` (an agreement test); `MAX_LAYER_BYTES` and `MAX_LAYER_ENTRIES` (limits).
- 31: `resolveAccounts` (13 uses, some with inputs the CLI cannot give yet, such as `run:` from the paused alz run); `ENV_NAMES`; `parseYamlLimited` (alias limit control); `readRegularFileWithId` (O_NONBLOCK under the trace); `sha256`; `valueImports` (source-graph proofs that account resolution imports no git or writer); the fs trace.
- 32: `Terminal` and `createTerminal` (fake terminal, about 60 cases); `updateAllowed` through `test/allow-driver.ts` (12-process lock test, bundled with esbuild); `parseYamlLimited` (reads back allowed.yaml); `importedSpecifiers`, `valueImports` and `SRC` (stdin is read only in terminal.ts; "real wiring" bundles src/cli/bin.ts with esbuild); the fs trace.
- Helpers: `account-workspace.ts` imports `stringifyYaml`, `gitEntryName`, `hashLayerDir` and `Terminal`; `allow-driver.ts` imports `updateAllowed`.

TypeScript-specific assertions: 24 lines 589 and 599 (`not.toContain("TypeError")`); every fs-trace and import-graph proof in 30, 31 and 32. The decision-26 assertions: 25 (`--version` is exactly the bare version), 27 (installed `--version` is the bare version, `--help` contains `usage: alz`), 31 lines 258-270 and 32 line 228 (`usage: alz`). The control-character case in 31 stays. No test checks `-v`.

Other findings:
- 26 line 844 runs alz with `PATH=/nonexistent`. A default bundle that starts with `#!/usr/bin/env node` would fail there, so the default binary needs an absolute node path in its shebang.
- 32 already has the convention "written out, not imported, so a change to the list in src fails here" (AGENT_SESSION_MARKERS). Test-side spec formulas follow it.
- The temp folder holds 33,102 alz-* entries again, after 64,000 were removed earlier on 2026-11-22. That is about 1,000 per run.
- git here is Apple's /usr/bin/git, an xcrun shim with its cache at $TMPDIR/xcrun_db. Children get an explicit env without TMPDIR.

Owner answers (2026-11-22): PTY driver deferred to R6 (TASK-55.2.8 now blocks TASK-55.7, and R1 can finish without it); leftover temp folders deleted; TASK-55.2.9 layer text approved.

Review fixes on R1 (implementer): (1) test/exec.ts killGroup refuses any pid that is not an integer > 1 (isStartedPid); capture kills the group only for a child that started (a failed start reports pid 0). (2) capture now sends stdout/stderr to files under the run root instead of pipes, so spawnSync returns when the direct child ends; the group is then listed (before the kill) and killed. 'sh -c "sleep 30 & sleep 31"' with a 500 ms timeout throws in well under 5 s, listing sleep 30, and none survives. (3) Import guard walks test/acceptance recursively, allows leading whitespace, follows absolute specifiers (reported as src/...); positive control for each; tmpdir guard comment says it is discipline, not a boundary. (4) 27 split: acceptance keeps decision-26 --version/--help through layers(); the install-script cases moved to test/reference/27-install-local.test.ts under their names with an allow-listed env (PATH, HOME, BIN_DIR, PREFIX, npm_config_cache, npm_config_userconfig=/dev/null) and a HOME for every alz. (5) 26: the guarded-clone case is renamed to what it proves (inside-the-repo rejection; the GIT_ALLOW_PROTOCOL guard stays in reference under the old name); the git: remote case puts a fake ssh and the git stand-in on PATH, asserts 'error [source]', the source named, and a recorded clone of host.invalid:client.git. The spec names no wording for a failed fetch, so none is asserted (open: spec gap). (6) stale-roots: per-root catch, stderr report, sweep continues; processExists treats only ESRCH as gone. (7) alz-bin.ts shebangLine refuses a node path with whitespace or over 200 bytes. (8) mise test:acceptance uses npm exec --no. (9) nits: stale R1.7 comments, word-bounded version match (holdsVersion), git-proxy PATH-only comment, in-process runs get TMPDIR=<run root>/tmp (done in test/reference/in-process.ts, no src/ change). (10) .layer skill and rule text updated, alz:build and alz:check run. Not fixed by design: Ctrl-C orphans of detached children, smoke test 12 env.

Review fix round verified by the lead (2026-11-23): `mise run test` 45 files, 1080 passed, 1 expected fail, 1 skipped (1082), 93 s; `mise run lint` and `mise run alz:check` pass; ALZ_BIN=target/release/alz on 25 and 27: 3 passed; no acceptance file spreads process.env except the opt-in smoke test 12. Mutation checks against the pre-fix test/exec.ts: (a) the pid-0 finding is real — running test/reference/exec.test.ts on the old helper killed the caller's whole process group, the lead's shell included; (b) the grandchild-pipe finding did not reproduce — the new case 'fails within the timeout, ... leaves no sleep 30' passes on the old helper in 1 s, because Node's spawnSync closes the child's pipes when its timeout fires. The file-based capture stays (harmless, simpler to reason about), but that case is not a regression test, and the 132 s stall in TASK-54 is still unexplained. Not fixed, recorded: Ctrl-C leaves detached children running; opt-in smoke test 12 passes the parent environment; run-root teardown can still throw if its own root cannot be removed. Spec gap: LAYERING-SPEC names no diagnostic for a failed fetch, so 26 asserts only 'error [source]' and the source; added to TASK-65.
<!-- SECTION:NOTES:END -->

## Comments

<!-- COMMENTS:BEGIN -->
author: @architect
created: 2026-11-22T17:58:00Z

---

Owner questions from planning (architect, 2026-11-22). R1.1 to R1.7 can go ahead without answers.

Q1. Pseudo-terminal driver for alz account allow (TASK-55.2.8). About 60 cases in 32, and 2 in 24, need a real terminal once alz is spawned.
- (a) A rustix-based pty driver in alz-test-support. R6 reuses it. It needs a decision that allows rustix in alz-test-support outside alz-io (decision-27).
- (b) node-pty as a dev dependency (a native addon).
- (c) python3 pty.
- (d) No driver: the cases stay reference-only until R6.
Recommendation: (a). Default until you answer: (d) for the time being, with the cases running under test/reference.

Q2. Can R1 (TASK-55.2) close with R1.8 still open, if the answer is (a), (b) or (c)? Recommendation: yes. Move R1.8 to depend on nothing in R2 and to block R6 (TASK-55.7) instead, so R2 is not held up.

Q3. The temp folder holds 33,102 leftover alz-* entries again. May I delete them, as on 2026-11-22? R1.7 AC #3 compares listings from before and after a run, so the clean-up is not required, but H1 in TASK-54 suggests the folder size may cause the stall.

Q4. R1.9 changes the repository layer (the add-acceptance-test skill, the tests rule, the map). It is drafted in the task and waits for your approval.

---
<!-- COMMENTS:END -->
