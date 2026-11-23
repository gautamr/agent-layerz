---
id: TASK-64
title: Sort names by Unicode code point everywhere, as the spec says
status: Done
assignee:
  - '@lead'
created_date: '2026-11-22T19:20:00Z'
updated_date: '2026-11-23T04:28:00Z'
labels:
  - discovered
  - area:spec
milestone: m-7
dependencies: []
references:
  - docs/LAYERING-SPEC.md
  - src/core/files.ts
priority: high
type: bug
ordinal: 80390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Found in R1.2 (TASK-55.2.2). Spec §5.3 says code point order, but src/core/files.ts compareCodePoints uses `<` (UTF-16 code unit order) and many sites use plain .sort(); the content hash, the generated file tree (src/render/file-tree.ts) and account state follow it. Names with characters above U+FFFF sort differently from the spec, so a Rust port that sorts by code point would not be byte-identical. Owner decision (2026-11-23): keep code point order and fix the TypeScript reference (a fix both carry, decision-21). test/reference/spec-formulas.test.ts has an it.fails KNOWN DRIFT case for this.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The spec states name order once (§6, code point order, lone surrogates an error) and every rule that sorts points to it; src/ is unchanged (decision-30)
- [x] #2 test/acceptance/33-name-order.test.ts proves the order through the CLI; its three spec cases are marked reference drift (TASK-64) and fail as expected on the TypeScript reference, and the KNOWN DRIFT case in test/reference/spec-formulas.test.ts stays an expected failure
- [x] #3 The changelog notes what changes for a repository when it moves to the Rust alz: content hashes, the order in generated files and layer.lock, and lone surrogates
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
### Goal
One comparison, Unicode code point order (spec §6 "Name order"), at every place src/ sorts names, paths or keys. Owner decision 2026-11-23: code point order, fix the TypeScript reference (decision-21 allows fixes). The Rust port must be byte-identical to the fixed reference.

### Inventory (line numbers on main f17287f)
Reaches the lock, a hash or generated output:
1. src/core/files.ts:67-69 `compareCodePoints` uses `<` (UTF-16 unit order). The root cause. Callers:
   - files.ts:64 `listFiles` -> layer file list (resolve/reader-load.ts:50, resolve/layer-hash.ts:20) -> content hash in layer.lock and the cache check (§5.3); `filesIn` order (merge/layer-files.ts:41) -> AGENTS.md block order (merge/instructions.ts:18), and the insertion order of rules, agents, commands (rules.ts:25, agents.ts:71, skills.ts:105) and passthrough (passthrough.ts:26) -> `alz ls`/`explain` and diagnostics order; pipeline.ts:100 and render/workspace.ts:56 scans -> diagnostics and delete order.
   - render/file-tree.ts:30 `paths()` -> write order, lock `files`, protected.ts:14, build/check counts. Generated paths are ASCII in practice (SKILL_NAME, passthrough refuses non-ASCII), so no visible change; fixed by (1) anyway.
   - resolve/account-allow-state.ts:170 -> entry order of allowed.yaml (account state).
   - resolve/account-resolution.ts:365 `staleApprovals` -> `alz account which` listing.
2. src/core/stable-json.ts:11 own `<` comparator, and `Object.fromEntries` re-hoists keys that are whole numbers ("0", "9", "10"; not "007") to the front in numeric order (checked with node). Reaches layer.lock (model/lock.ts:90), .claude/settings.json and .mcp.json (adapters/claude/index.ts:42,44).
3. src/resolve/tree-cache.ts:105 own `<` comparator on a tree2@ file list -> order of a tree cache entry's files. Confirm in the PR that no hash or output depends on it; switch to `compareCodePoints` either way.
User-visible listings only (switch for "everywhere"; no change for valid names):
4. resolve/account-folders.ts:160 `.sort()` account names (ASCII by ACCOUNT_NAME).
5. doctor/account-dirs.ts:21,41,45 `.sort()` -> `alz doctor` listings of $HOME and account folder entries (any name).
6. merge/skills.ts:71 `.sort()` skill directory names -> diagnostic and listing order.
7. render/workspace.ts:69 `.sort()` collisions -> the collision error and adoption candidates.
8. adopt/index.ts:35 `.sort()` -> adoption order, the report and entries appended to the adopted layer's files.
Not name order, leave: account-allow-state.ts:150 (numbers), :243 (real path length), account-declaration.ts:109 (index), identity/report.ts:66 (fixed field/role order), cli/commands/init.ts:80 (git's own `--sort=-v:refname` of tags). No `localeCompare` or `toSorted` in src/. Insertion order of YAML mappings (frontmatter native keys, which also hoist whole numbers) is not a sort: TASK-66.

### The comparison
Keep `compareCodePoints(a, b)` in src/core/files.ts (exported, already imported by three modules); every site above calls it. Implementation (implementer's choice): walk both strings by code point (`codePointAt`, step 2 at >= 0x10000) and return -1/0/1 at the first difference, else by length; or the ICU fix-up (at the first differing unit, map D800-DFFF up by 0x2000 and E000-FFFF down by 0x800, then compare). It must be total over any JS string, lone surrogates included (YAML escapes can make them), never throw and not allocate Buffers per call. For well-formed strings it equals UTF-8 byte order, Rust's `str` Ord.

### stableJson
Write the text directly instead of rebuilding objects (any plain-object round trip re-hoists whole-number keys). Same format as `JSON.stringify(v, null, 2)`: 2-space indent, `": "`, `{}` and `[]` when empty, scalars through `JSON.stringify(scalar)`, `undefined` members dropped, `undefined` array items as `null`, trailing newline. Keys by `compareCodePoints` at every level. Check that no caller passes a value with `toJSON` (Buffer, Date).

### Spec changes (written on this branch, uncommitted; ship in this PR)
- §6 new paragraph "Name order": code point order, prefix first, paths as whole `/`-strings (`a-b` before `a/b`), no locale/case/normalization, digit keys are strings (`10` before `9`), equals UTF-8 byte order and Rust `str` order, not UTF-16 unit order.
- §5.3: "code point order (name order, §6)" plus one sentence that hashes of such layers differ from releases before TASK-64. Kept minimal for TASK-65.
- §7.1 and §7.13 allow state point to §6.
- Add in the PR, §13: "25. **Name order** (TASK-64, `test/acceptance/33-name-order.test.ts`): instruction files `z.md`, `｡.md` (U+FF61) and `😀.md` (U+1F600) give `AGENTS.md` blocks in that order; the layer's content hash in `layer.lock` lists its files in that order (§5.3); `.claude/settings.json` keys `-x`, `10`, `9`, `a`, `｡`, `😀` appear in that order; `alz check` then passes."

### Changelog and version (user-facing)
Generated output and lock hashes change for affected layers. CHANGELOG.md did not exist (TASK-8 will add 0.1.0); this branch creates it with Keep a Changelog and an Unreleased/Fixed entry (uncommitted; ship in this PR). Commit `fix(core): ... (task-64)`, no `BREAKING CHANGE` footer: the lock format and `lockVersion` stay (bumping would invalidate every lock). Recommended: the next release is a minor bump (0.2.0); owner decides.

### Files touched
src/core/files.ts, src/core/stable-json.ts, src/resolve/tree-cache.ts, src/resolve/account-folders.ts, src/doctor/account-dirs.ts, src/merge/skills.ts, src/render/workspace.ts, src/adopt/index.ts; test/acceptance/33-name-order.test.ts, test/fixtures/33-name-order/, test/reference/name-order.test.ts, test/reference/spec-formulas.test.ts; docs/LAYERING-SPEC.md, CHANGELOG.md.

### Tests
- Acceptance test/acceptance/33-name-order.test.ts, fixture test/fixtures/33-name-order/ (personal layer + repo, ASCII only). The test writes the exotic names into the copied fixture after `fixture()`, so no committed file name leaves ASCII. Through the CLI only:
  C1 "orders instruction blocks by code point": `instructions/z.md`, `instructions/\u{FF61}.md`, `instructions/\u{1F600}.md` with markers; `alz build` exits 0; in AGENTS.md MARK-Z < MARK-FF61 < MARK-1F600 by index. The current code gives Z, 1F600, FF61.
  C2 "hashes the layer with its files in code point order": the lock entry's `contentHash` equals `sha256:` + `contentHash(layerDir)` from test/spec/content-hash.ts.
  C3 "sorts JSON keys by code point, whole numbers included": settings.yaml `claude:` keys `\u{1F600}`, `\u{FF61}`, `10`, `9`, `-x`, `a`; read the key order from the raw text of .claude/settings.json (never JSON.parse, which re-hoists); expect `-x`, `10`, `9`, `a`, `｡`, `😀`. If settings keys are refused, use mcp.yaml server ids and .mcp.json; note which.
  Then `alz check` exits 0. Record each case red on the old code, green after.
- test/reference/spec-formulas.test.ts:161-168: `it.fails` -> `it`, drop the KNOWN DRIFT comment; add a U+FF61 pair to `cases`.
- test/reference/name-order.test.ts (TS-only, deleted at cutover), laws: (L1) sign(compareCodePoints(a,b)) == sign(Buffer.compare(utf8 a, utf8 b)) over pairs at U+007F/0080, U+07FF/0800, U+D7FF, U+E000, U+FF61, U+FFFD, U+FFFF, U+10000, U+1F600, U+10FFFF, prefixes and ""; (L2) compare(a,b) == -compare(b,a) and 0 only when a === b, lone surrogates included; (L3) prefix first; (L4) JSON.parse(stableJson(v)) equals v without undefined members; keys appear in compareCodePoints order at every level, whole numbers included; for objects without whole-number keys the bytes equal the old output.
- Fixtures: none change. Scanned test/fixtures and .layer: no names or content at or above U+E000, no whole-number YAML/JSON keys. This repo's layer.lock should not change; `mise run alz:check` confirms (it also covers the personal layer, not scanned).

### Untrusted inputs
Layer file names (any Unicode), YAML keys in settings.yaml and mcp.yaml (lone surrogates possible), lock keys. The comparator is total and pure; stableJson recursion depth is bounded by the existing YAML size and alias limits. Nothing new reaches the file system or a subprocess.

### Notes for the Rust port (R2)
`str`/`String` Ord and serde_json's default BTreeMap give this order. Never sort paths with `Path`/`PathBuf` Ord, which compares by component (`a/b` before `a-b`).

### Evidence for the ACs
AC1: `grep -rnE "\.sort\(\)|localeCompare|toSorted|\? -1 :" src/` prints nothing, and every remaining `.sort(` is `compareCodePoints` or an entry under "leave"; law test passes. AC2: `npx vitest run test/acceptance/33-name-order.test.ts test/reference/spec-formulas.test.ts test/reference/name-order.test.ts`, then `mise run test`, `mise run lint`, `mise run alz:check`. AC3: spec §6 and CHANGELOG.md in the diff.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Planning (architect, 2026-11-23): plan written. Spec text (§6 Name order, pointers in §5.3, §7.1, §7.13) and a new CHANGELOG.md (Unreleased/Fixed) are edited on branch fix/code-point-order-task-64, uncommitted, to ship in this task's PR. Discovered TASK-66 (YAML mapping keys that are whole numbers lose source order in the TS reference; out of scope here). stableJson also re-hoists whole-number keys, which this task fixes under AC #1.

Implementation (implementer, 2026-11-23). Changes: compareCodePoints (src/core/files.ts) now maps UTF-16 surrogates above U+E000-U+FFFF and compares unit by unit (total over lone surrogates, no allocation, equals UTF-8 byte order); stableJson (src/core/stable-json.ts) writes the JSON text directly, so whole-number keys are no longer re-hoisted (same bytes as JSON.stringify(v,null,2) otherwise; toJSON was never honoured by the old code either, so not added); every sort site in the plan inventory (tree-cache.ts:105, account-folders.ts, doctor/account-dirs.ts x3, merge/skills.ts, render/workspace.ts, adopt/index.ts) calls compareCodePoints. Spec: added section 13 entry 25; the architect's section 6, 5.3, 7.1, 7.13 text and CHANGELOG.md left as written (no sentence proved wrong by the code).

AC1 evidence: grep -rnE '\.sort\(\)|localeCompare|toSorted|\? -1 :' src/ now prints only the comparator itself (files.ts:79). Every remaining .sort( is compareCodePoints or an entry under 'leave' (identity/report.ts:66, account-allow-state.ts:150 and :243, account-declaration.ts:109). Question 'does tree-cache.ts:105 reach a hash': no. listTree's order only decides the write order in writeTree and which problem verifyTreeEntry reports first (a diagnostic text); the lock hash comes from the layer file list (layer-hash.ts via listFiles), not from the tree list. Switched anyway.

AC2 evidence, red on unfixed src/ (new tests written first): test/acceptance/33-name-order.test.ts 3 of 4 failed: block order gave Z, 1F600, FF61; lock contentHash sha256:f3b9a727... vs spec-side sha256:3a46d2cf...; settings.json keys came out [9,10,-x,a,U+1F600,U+FF61] (whole numbers hoisted and UTF-16 order) instead of [-x,10,9,a,U+FF61,U+1F600]. The 4th (alz check after build) passes before and after by design. test/reference/name-order.test.ts: 3 failed (UTF-8 byte-order law L1, FF61 before 1F600, stableJson key order L4); test/reference/spec-formulas.test.ts: the new U+FF61 case failed, and the former it.fails case, now a plain it, failed as expected. After the fix: 3 files, 47 tests passed. Settings keys under claude: were accepted, so no mcp.yaml fallback was needed. The KNOWN DRIFT it.fails is now a normal it.

AC3 evidence: spec section 6 'Name order' paragraph (once), pointers in 5.3, 7.1, 7.13, section 13 entry 25; CHANGELOG.md Unreleased/Fixed says affected layers get a new content hash.

Checks: mise run test 47 files, 1095 passed + 1 skipped (acceptance project 32 files, 705 passed + 1 skipped; reference project 15 files, 390 passed). mise run lint clean. mise run alz:check: 'check passed: 68 generated files match the layers and the lock'; layer.lock is unchanged (nothing in this repo or the personal layer has names at or above U+E000), so no alz:build was needed. ALZ_BIN run not done: no Rust binary exists yet. Open: TASK-66 (YAML mapping key order) untouched; release version bump left to the owner.

Owner (2026-11-23): commit as fix: with the changelog note and no version decision now (release tooling, CI only, and TASK-8 decide); keep CHANGELOG.md in this PR. TASK-66 blocks R5 (TASK-55.6).

Owner (2026-11-23): never change src/ again; every bug is fixed in Rust only (decision-30). The src/ fix, its reference tests (test/reference/name-order.test.ts) and the spec-formulas change were reverted; the scope is now the spec, the changelog and the acceptance cases, marked as reference drift with the new test/drift.ts helper. The Rust port implements §6 in R2 (lock and content hash model) and R5 (AGENTS.md, settings.json, .mcp.json); test 33's drift cases must pass against the Rust binary there. Review findings on the reverted stableJson (sparse arrays) no longer apply; the lone-surrogate finding is settled in §6 (an error); the 'array-index key' and layer.lock wording nits are applied to the changelog.

Verified by the lead (2026-11-23): src/ identical to main; `mise run test` 46 files, 1081 passed, 4 expected fail (test 33's three drift cases and the spec-formulas KNOWN DRIFT case), 1 skipped; `mise run lint` and `mise run alz:check` pass. With ALZ_BIN at a 755 copy of dist/alz-standalone.mjs, test 33 gives 3 failed, 1 passed, so the drift marker turns into ordinary tests for any ALZ_BIN. Consequence for R1.7 AC #2: the whole acceptance project no longer passes against a TypeScript copy given as ALZ_BIN; drift cases must pass only on the Rust binary. The reviewer and security-reviewer ran on the earlier src/ version; after the revert the diff is the reviewed spec and test plus test/drift.ts and one provide() in test/setup/run-root.ts, checked by the runs above.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
The spec states name order once (§6): code point order everywhere alz sorts names, paths and keys, with digit keys as plain strings and lone surrogates an error. Per decision-30 the TypeScript reference is not changed: test/acceptance/33-name-order.test.ts asserts the spec through the CLI with three cases marked reference drift (new test/drift.ts: an expected failure on the bundled reference, an ordinary test for any ALZ_BIN), and the Rust port implements the order in R2 and R5. CHANGELOG.md is added with what changes for a repository moving to the Rust alz.
<!-- SECTION:FINAL_SUMMARY:END -->
