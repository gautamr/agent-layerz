---
id: TASK-55.6
title: 'R5: Render, Claude adapter and commands in Rust'
status: In Progress
assignee:
  - '@architect'
created_date: '2026-11-22T12:54:00Z'
updated_date: '2026-11-24T07:17:00Z'
labels:
  - area:rust
milestone: m-7
dependencies:
  - TASK-52
  - TASK-53
  - TASK-55.5
  - TASK-63
  - TASK-66
  - TASK-69
  - TASK-71
  - TASK-76
  - TASK-79
  - TASK-80
parent_task_id: TASK-55
priority: medium
ordinal: 38390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
doc-5 phase R5: the Claude adapter, adoption, init, build, check, diff, ls, explain, doctor, lint.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Byte-identical generated output for every fixture, and the command cases pass against the Rust binary
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
### Scope
doc-5 R5: the Claude adapter (AGENTS.md, CLAUDE.md, .claude/agents, skills, commands, slot stubs, rules, settings.json with hooks and scripts, .mcp.json, passthrough), adoption (`alz build --adopt`), and the commands build, check, diff, ls (with --live), explain, doctor and init. `alz lint` is done (R4). Out of scope: the Codex adapter (the reference has none: a manifest with `targets: [codex]` stays the error [target] "not implemented yet", as in the reference; the Codex TOML work, including quoting every key that holds a dot, belongs to TASK-18); commit identity checks and writes (§9.4), the account-allow hint and `alz account which/allow` (R6, TASK-55.7). src/ is never changed (decision-30); the reference is read for behaviour only (decision-22). Work runs as subtasks R5.1 to R5.15, in order, in one pull request, as in R4.

### What exists (current code, main c133dd3)
- alz-core: `merge(&ResolvedStack, &Manifest, &Placements) -> Merging`; `Merged` with private fields and provenance (`Traced<T>`); the YAML emitter (render/yaml, byte-pinned by the golden samples), the stable JSON emitter (render/json), `Lock::emit`, budgets, `ProtectedPattern` (text only, no matcher), NativeMap in source order (TASK-66), `TargetName`.
- alz-io: `StackResolver`, `placements_of_stack` (not wired to merge yet), `Dir`/`ReadOnlyDir` no-follow reads, `CacheDir`, `write_file_below` (cache only), `GitRunner`, `EnvSnapshot`.
- alz: clap CLI with `lint` only, `report.rs` prints diagnostics.
- Missing: a file tree, header, glob matcher, every renderer, the repository writer, adoption, every command but lint.

### Module layout (doc-6 §2, decision-23)
alz-core (pure):
- `render/tree.rs`: `FileTree` (BTreeMap<OutputPath, GeneratedFile>), `OutputPath` (a SafeRelPath with a folded key: ASCII lower case of the NFC form), `GeneratedFile { bytes, mode: FileMode::{Regular, Executable} }`. Inserting a path whose folded key is taken is an error naming both, never an overwrite.
- `render/header.rs`: `SourceRef` (layer label and SafeRelPath) and the header text; `html_header`, `frontmatter_header`, `insert_frontmatter_comment`.
- `render/glob.rs`: `PathGlob` parsed from a ProtectedPattern; `**` spans folders, `*` and `?` do not, a pattern without `/` matches the file name at any depth; matched with ASCII case folding; a total matcher (no regex, no backtracking blow-up).
- `render/protected.rs`: the §4.5 defaults, the adapter's memory folders and `protected_paths`, dropped from the tree with a `protected` warning.
- `render/instructions.rs`: AGENTS.md (layer headings, blocks, the §7.12 Commits section from `Merged::commit_identity`) and the budget check of the composed text, Commits included.
- `target/claude.rs` + `target/claude/`: `agents.rs`, `skills.rs`, `rules.rs`, `settings.rs` (permissions, floor fields, additionalDirectories, hooks, default model, deep merge with settings.claude), `hooks.rs`, `mcp.rs`, `passthrough.rs`, `tools.rs` (neutral to native tool and mode names), `facts.rs` (the verified range `>=2.1.311 <2.2.0`, managed folders, memory folders, adopt-always files). The `Target` trait stays sealed; Claude is its only implementation in R5.
- `build.rs`: `render(&Merged, &Manifest) -> Rendered { tree, diagnostics }`, then `lock_of(&Rendered, &ResolvedStack) -> Lock`. Typestate (doc-6 §3.3): Merged -> Rendered -> (alz-io) Written. `Rendered` has no public constructor.
- `workspace.rs`: `plan_workspace(&Rendered, &Lock, previous: Option<&Lock>, &Observed, &Protected) -> WorkspacePlan { changes, collisions, lock_text, lock_changed }`: pure; `Observed` is what alz-io saw on disk.
- `gitignore.rs`: the managed `.gitignore` block (current and legacy markers), pure text in, text out.
- `diff.rs`: unified diff of two byte strings (owner question 3).
- `adopt.rs` + `adopt/`: `AdoptionPlan` from observed native files and the Merged stack before adoption; `claude_md.rs`, `agent.rs`, `settings_json.rs`, `mcp_json.rs`, `manifest_edit.rs`; the planned repo layer is checked with the R4 per-layer readers before it is accepted.
- `listing.rs` (ls rows and owners), `explain.rs` (sources and outputs), `doctor.rs` (findings from observations), `scaffold.rs` (init's texts), `probe.rs` (Claude's stream-json init event, parsed into names only).
alz-io (effects):
- `fs/repo_root.rs`: `RepoRoot`, the only capability that writes inside the repository. `observe(&FileTree, managed folders, previous lock)` reads each path one component at a time with O_NOFOLLOW (decision-27), regular files only, at most the 1 MiB cap plus one byte, and reports Absent / Regular { bytes, executable } / NotRegular (link, folder, FIFO). `apply(&WorkspacePlan, &Rendered)` writes each file as a temporary file in its folder (O_EXCL, fchmod 0644/0755) renamed over the target with renameat, deletes, prunes empty folders up to the root, and writes the lock last. Missing folders are made with mkdirat and opened no-follow. Every path is an OutputPath, so nothing can name a place outside the root.
- `fs/repo_walk.rs`: the walk for AGENTS.override.md (target-facts §8.4), no-follow, skipping `.git`.
- `tool.rs`: `ToolRunner` for the `claude` binary (`ALZ_CLAUDE_BIN` or `claude`): argument arrays, no shell, stdout and stderr capped, used only by doctor (version) and ls --live (probe).
- `account_names.rs`: names-only listing of `agents/` and `skills/` and the presence of `CLAUDE.md` in each Claude account folder, for doctor; never opens a file there (§9.1, invariant "read for names only").
alz (wiring): `command/{build,check,diff,ls,explain,doctor,init}.rs` and `command/pipeline.rs` (read manifest and lock, resolve, placements_of_stack, merge, render, lock), each a thin use case; output through report.rs.

### Command flows
- build: manifest and lock (R2/R3 readers) -> StackResolver -> placements_of_stack -> merge -> render -> drop protected -> AGENTS.override.md check -> lock_of -> observe -> plan_workspace -> collisions are errors (`collision`) unless --adopt -> apply -> .gitignore block when `commit_generated: false` -> summary line `built <n> files for <targets>: <c> created, <u> updated, <d> deleted (output visibility: <v>)`. Any error diagnostic before apply: exit 1 and nothing written (tree, lock and cache entries already published stay as R3 defines).
- build --adopt: as build up to the plan; then plan adoption (pure) over the collisions and the adopt-always files; any problem lists every problem, writes nothing, exit 1; else RepoRoot applies the adoption, the pipeline runs again from the manifest, and a collision left is an error.
- check: the same pipeline without apply: each change is `error [check] <path>: missing | differs from a fresh build | is stale`, each collision, `layer.lock: out of date`; then `check failed: run \`alz build\` and commit the result`, or `check passed: <n> generated files match the layers and the lock`. `--skip-identity` is parsed; the identity check itself is R6.
- diff: the plan's changes as unified diffs (`Binary file <path> changes` for binary content), the lock's diff, a note per collision, `no changes`.
- ls: sections agents, skills, commands, rules, hooks, mcp with counts and the deciding layer; `--live` runs the probe and compares built names with what Claude loaded.
- explain <type> <name>: types agent, skill, command, rule, hook, mcp, instruction; sources general to specific with their actions, then the merged output file(s) from the tree; usage error exit 2 for an unknown type.
- doctor: Claude account folders (names only) against the built names, the `claude --version` against the verified range, client root and ancestor CLAUDE.md/AGENTS.md/.git, workspace folders, then the build's diagnostics; exit 1 on any problem.
- init: `--kind repo|layer`, `--name`, `--visibility`, `--stack`, `--hub` (the hub tag through GitRunner, the tag parsed as a GitRef before it is written); create-only writes through RepoRoot; the identity set-up is R6.
Messages follow the spec (decision-26), not the reference; R5.1 writes every message the suite asserts into §10.

### Parity with the reference and the drift list
Generated files and layer.lock are byte-identical to the reference for every fixture (doc-5 principle 3), proven by golden trees (owner question 7) and the acceptance suite. Known drift, each a spec sentence plus a referenceDrift case or a golden sample with a drift field:
1. YAML emitter cases 1 to 4 of spec §10.2 and the line-break read-back case (TASK-73).
2. Native agent keys that are whole numbers keep source order (TASK-66).
3. Name order of AGENTS.md, settings.json and .mcp.json by code point (TASK-64, decision-30, test 33).
4. Text trimmed of Unicode White_Space only, not U+FEFF (TASK-55.5.4).
5. The permission floor fields written into settings.json (TASK-55.5.16), once TASK-76 records the keys.
6. Case-folded collisions and protected paths (R4 security review): a passthrough `.claude/Settings.json` collides with the generated `.claude/settings.json`; `Backlog/x` is protected like `backlog/x`.
7. Writes never go through a link; a generated path whose entry or a parent folder is a link is refused (owner question 1).
8. A file is overwritten or deleted only when alz can show it generated it (owner question 1).
9. Adoption refuses any text the reader would refuse, before it writes (TASK-73), and adoption problems print untrusted text stripped (class 1).
10. Every refusal R2 to R4 added (settings targets, Codex reserved keys, credentials in sources, account folders, XDG, hook environment, invalid UTF-8 names) through build and check.
11. The manifest edit of --adopt and the diff text (owner questions 2 and 3).

### Untrusted input handled
- Everything in Merged (already validated by R4): names, paths, bodies, native fields, settings trees. File names reach headers and output paths: header refs hold no control or format character and cannot close their comment (owner question 6).
- Files on disk in the repository (existing outputs, managed folders, adopted native files): links, FIFOs, folders, oversized files, invalid UTF-8, the generated mark, JSON with floats (owner question 4), YAML layer files adoption merges into.
- The previous layer.lock (untrusted): its `files` entries never make a path deletable on their own.
- The output of the `claude` binary (probe and version): capped, parsed into names, printed stripped.
- The hub's git tags (init): parsed as GitRef. CLI arguments (stack sources, names): parsed by the R2 types.

### Tests
- Unit and property tests per module. Laws: render is deterministic (`render(m) == render(m)` byte for byte); FileTree never holds two paths with one folded key; plan_workspace after apply gives no changes (idempotence); apply never writes or deletes a path the plan does not name and never a path that is not owned; the glob matcher agrees with a reference matcher on generated paths; the diff applied to `before` gives `after`; adoption followed by a build gives the same settings and agents as the native files it adopted (test 20's law).
- Golden trees: the reference's output for every fixture repository, made by a script and committed, compared byte for byte by a Rust integration test (assert_cmd) against the Rust binary.
- alz-io integration tests with links, FIFOs, unreadable files and case-variant names in temporary repositories.
- The acceptance suite against ALZ_BIN (below).

### Acceptance files that pass against ALZ_BIN by the end of R5
Baseline 2026-11-23 (target/release/alz, R4 merged): 981 cases, 276 pass, 700 fail, 5 skipped.
- Whole file: 01 to 12, 14 to 25, 27, 28, 33 to 37, with test 36's Linux-only 0xFF cases where the platform runs them.
- Test 26: every case except the §9.4 ones (git identity setup by init and build --adopt, identity warnings of build, the identity part of check), which R6 passes.
- Test 29: every case; its init cases that set an identity are R6.
- Test 30: every case that drives build, check, init or lint (the pathref@ index written by build and the resolving commands); the account-which cases are R6.
- Tests 31 and 32: R6, except their build set-up steps, which must not fail.
R5.15 runs the whole suite, records the counts per file in this task's notes, and names every case left for R6 in TASK-55.7's notes.

### Carried criteria R5 closes
TASK-52 #4, TASK-53 #4, TASK-63 #4, TASK-71 #3 (build and check forms in test 36 and 24), TASK-66 #2 (generated agent with native keys 10 and 9), TASK-73 #4 (drift parity list), TASK-69 #5 where a Linux run is available (owner question 10), the build rows of test 37, and the decision-30 name-order cases of test 33.

### Spec changes in the same PR (R5.1)
§4.3 (the writer: ownership, links, case folding, lock last, the .gitignore block, the adoption manifest edit), §4.5 (case folding), §7.1 (the header and its sources), §8.1 (the Claude output per item, the floor field keys), §10 (each command's output lines and exit codes, the diff format), §10.1 (the codes collision, check, budget, protected, adopt, target, claude-agent, tool-scope, hook-on-fail, ls), §13 (new entries for the writer rules and the parity goldens). CHANGELOG: the stricter writer and adoption are user-facing (builds that worked may now refuse); minor version bump at cutover (doc-5).
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
decision-30: R5 implements name order (§6) for AGENTS.md, settings.json and .mcp.json; the reference-drift cases of test/acceptance/33-name-order.test.ts (TASK-64) must pass against the Rust binary. TASK-66 settles mapping-key order first.

Architect (2026-11-23, owner answer 6 in TASK-55.3's notes): R3 and R4 are judged by Rust tests plus the cases their CLI can reach, so the suite files they name (16, 24, 28, 30; 02 to 11, 17 to 19, 26, 29) must pass against the Rust binary in R5, together with this task's own command cases.

From TASK-73 (2026-11-23, lead): adoption refuses, before writing, any text the reader would refuse; see TASK-73.

From R2.7 (2026-11-23): serde_json without float_roundtrip (decision-33) can read a float literal up to 1 ulp off where JSON.parse is exact; check before R4/R5 read target JSON with floats.

From the R4 security review (2026-11-23, lead): generated-file collisions and the §4.5 protected paths compare with ASCII case folding (as item/passthrough/coverage.rs does), so a passthrough file such as targets/claude/.claude/Settings.json counts as colliding with .claude/settings.json on a case-insensitive file system; add an acceptance case.

From the R4 review fixes (2026-11-23): the TOML emitter for Codex config quotes every key that holds a dot, so a YAML key written as one dotted string stays one literal TOML key; add a test.

Architect (2026-11-23, R5 plan). Subtasks R5.1 to R5.15 are TASK-55.6.1 to TASK-55.6.15 (one pull request, in dependency order). R5.2 to R5.6 can start now; R5.1, and through it R5.7, R5.12 to R5.15, waits for the owner answers below. R5.5 also waits for TASK-76.

Baseline (2026-11-23, ALZ_BIN=target/release/alz built after R4, `vitest run --project acceptance`, 23 s): 981 cases, 276 pass, 700 fail, 5 skipped. Passing now: 10 (3), 16 (3), 24 (18), 25 (1), 26 (37), 27 (2), 28 (38), 29 (46), 30 (5), 31 (11), 32 (4), 34 (18), 35 (5), 36 (8), 37 (77). Every other case of 01 to 23 and 33 fails because it drives build, check, diff, ls, explain, doctor or init. Of the failures, 31 (105) and 32 (57) are account which and allow (R6), test 26's §9.4 groups (setting it up, alz check, part of hardening) are R6, and the rest are R5's.

Owner questions for R5 (each with options and a recommendation; R5.1 is labelled needs:owner-approval):
1. Which files the writer may overwrite or delete. The previous layer.lock is untrusted. (A) Recommended: a file is owned only when it carries the generated mark, or the previous lock lists it and its bytes match the hash recorded there; a lock entry alone never makes a file deletable or overwritable; an owned file is never under a link. A hand-edited JSON output (settings.json, .mcp.json have no mark) then becomes a collision instead of being silently replaced. (B) As the reference: a lock entry or the mark is enough. (C) A lock entry is enough only for paths this build makes or that lie in the managed folders. User-facing, reference drift, minor bump at cutover.
2. Writes and links. doc-5 item 6 leaves this to R5. (A) Recommended: refuse a generated path whose entry or any parent folder inside the repository is a link, as a collision, and never write through one. (B) Follow a link whose real path stays inside the repository. (C) As the reference (follows links).
3. alz diff. The reference prints jsdiff's patch text. (A) Recommended: an alz-owned Myers diff in alz-core, unified format with three lines of context and a/ b/ headers, stated in §10; not byte-identical to jsdiff (stdout is not a generated file). (B) The `similar` crate (new dependency: intake check and a decision). (C) Match jsdiff's bytes exactly.
4. JSON floats in adopted settings.json and .mcp.json. serde_json without float_roundtrip can read a float 1 ulp off (R2.7 note), so an adopted settings.yaml could differ from the reference's. (A) Recommended: enable serde_json's float_roundtrip feature (an amendment of decision-33, no new crate). (B) Refuse to adopt a float, naming the file. (C) Accept the rare difference as drift.
5. Target facts not yet verified. (A) Recommended: R5 emits exactly what the reference emits, since parity is the criterion; R5.1 lists in §8.1 each target fact the Claude output relies on with its target-facts row; the facts that are UNVERIFIED today (relative additionalDirectories, an unknown settings key) get a docs task, and nothing new is emitted on an UNVERIFIED fact (the floor fields wait for TASK-76, as §7.6 says). (B) Block R5 until every fact the adapter uses is at least DOCS. (C) Drop the output that rests on an UNVERIFIED fact (breaks parity).
6. Text in generated headers. A layer file name may hold a line feed or other control character today (SafeRelPath refuses only NUL), and an instruction or extra file's name reaches the AGENTS.md and CLAUDE.md headers; in a frontmatter header comment a line feed would start a new YAML line. (A) Recommended: the layer walk refuses a file or folder name holding a control or format character (C0, C1, U+2028, U+2029, Cf) as error [layer] naming the layer, like the invalid UTF-8 rule (§5.3), and a header source ref that holds `-->` is the error layer too. (B) Write such refs stripped in the header only. User-facing, reference drift.
7. How output parity is proven. (A) Recommended: golden trees made once from the reference by a script for every fixture repository, committed with drift samples marked, and compared by a Rust integration test; they survive R8 as Rust test fixtures. (B) A two-binary live comparison (an xtask) on each run. (C) Only the acceptance suite.
8. The adoption manifest edit keeps comments and layout (test 20). (A) Recommended: an alz-owned minimal text edit that appends entries to workspace.add_dirs and mcp.enable in block or flow form and keeps every other byte; a form it cannot edit safely is an adopt problem naming what to add by hand; not byte-identical to the yaml package's re-serialisation (drift). (B) A YAML CST crate (new dependency, decision). (C) Never edit the manifest; print the lines to add (spec and test 20 change).
9. Identity and the allow hint in R5's build, check and init. (A) Recommended: R5 builds them without §9.4 identity checks and writes and without the §7.13 account-allow hint; R6 adds both through a seam R5 leaves; test 26's §9.4 cases and test 32's hint cases stay R6. (B) R5 also does the read-only identity warnings; R6 the writes.
10. TASK-69 AC #5 needs a Linux run of the acceptance suite, which has no Linux CI yet (TASK-4). (A) Recommended: add an ubuntu job that runs the acceptance suite against the Rust binary only (a CI change, so the owner decides). (B) Leave AC #5 open until R7 or R8.
Not a question, recorded: the Codex adapter is out of R5 (the reference has none; targets: [codex] stays error [target]); the note that the Codex TOML emitter quotes every key holding a dot belongs to TASK-18.

Owner answers (2026-11-23): (1) ownership: a file is alz's own only if it carries the generated mark, or the lock lists it and its current bytes match the hash the lock recorded; anything else is never overwritten or deleted (R5.7, R5.8); (4) serde_json float_roundtrip is enabled, as an amendment to decision-33 (R5.1); (10) an ubuntu CI job runs mise run test, and later the acceptance suite against the Rust binary, added in R5 by the release-engineer (new subtask). Lead decisions: (2) any path under a link is refused; (3) alz's own Myers diff, jsdiff's exact bytes not required; (5) emit exactly what the reference emits, nothing new on an UNVERIFIED fact, and a docs task for gaps; (6) control and format characters in layer file names, and a header ref holding '-->', are refused; (7) golden trees made once from the reference and checked by a Rust test; (8) alz's own minimal text edit of layer.yaml on adopt, keeping comments; (9) identity checks and the account-allow hint stay in R6. New tasks blocking R5: TASK-79 (Claude settings keys with a recorded type only) and TASK-80 (Codex wrongly typed key behaviour).

From R5.15 (implementer, 2026-11-24): reference drift list, spec 13 'The reference drift list', by task: TASK-52 (36: layers and cache inside an account folder or the home folder refused); TASK-53 (36: empty or relative XDG_CACHE_HOME counts as unset); TASK-55.3 (26, 29, 34, 35: YAML tags, escapes decoding to control characters, version 1.10, account fields as schema errors); TASK-55.4 (24: refused hook or source neither run nor written); TASK-55.5 (37: unknown Claude or Codex keys in settings.yaml refused); TASK-55.5.6 (04: an overlay may not widen a path or command scope); TASK-55.5.12 (24: nested env checked to depth 128, not 32); TASK-55.5.16 (24, 37: agent native mcpServers, mcp_servers, hooks refused as agent; floor field flags); TASK-55.6 (22: ls in name order, explain prints the diagnostic prefix); TASK-55.6.13 and .14 (20: adoption keeps agents with native hooks out, hook scripts move to hooks/scripts/); TASK-55.6.15 (24, 28: wording of refused lock, cache entry, pinned tree; control-character file names; AGENTS.override.md search); TASK-59 to 62 (34, 35: YAML aliases, depth and size limits, manifest entry errors); TASK-63 (36: linked worktree leaves the index alone); TASK-64 (33: name order by code point); TASK-68 (26: first line left after control characters removed); TASK-71 (24: credentials in a layer source refused); TASK-73 (38: text quoted as an integer risk, one escaped space between line feeds); TASK-79 (37: settings keys and agent fields need a recorded type).

Unit review (2026-11-24, reviewer and security-reviewer). Must fix before the R5 PR: settings numbers finite and documented ranges checked (with a build-form case in test 37); ls --live probes with hooks, MCP servers and tools off (owner decision, target facts first); hook script names without shell metacharacters; the probe's process group always cleaned up; the CI check fails on a test file that does not load or a drop in the case count; the writer creates without replacing and re-checks before each change; lock entries parsed as output paths and reserved names compared case-folded; every in-repo repository layer folder is an input; protected paths compared after NFD folding; a case-only rename of a generated file; acceptance cases for a case-variant passthrough settings file and a case-variant protected path. Follow-ups: TASK-84 (repository walks), TASK-85 (claude program from the repository), TASK-86 (one collision helper, one output helper), TASK-87 (read-only view for read-only commands), TASK-88 (where the generated mark counts), TASK-89 (small items).

R5 review fixes, writer and CI findings (implementer, 2026-11-24). Items 1 to 7 are all done; nothing is left from this list. (1) CI check: test/ci/check-r6-pending.ts now also fails on a test file that failed to load or collect (status failed, no failed case), on more failed suites than the files and describe blocks that hold a failed case (vitest counts describe blocks, so a bare numFailedTestSuites > 0 would fail every R6 run), on a file name without test/acceptance/, and on fewer cases than the floor in test/ci/case-floor.json (1044 now; raise when cases are added). Tested by test/guards/r6-pending-check.test.ts (8 cases, small JSON reports). (2) alz-io writer: Change::Create goes through create_file (linkat, never replaces; a name that appeared is RepoError::Changed); Change::Update and Change::Delete carry the bytes the plan observed and the writer re-probes the path first (an edited, replaced or, for an update, removed file is Changed and nothing more is written; a delete of a file already gone is fine). Three tests in crates/alz-io/tests/repo_root.rs. A race between the re-probe and the rename remains (no lock); it is narrowed, not closed. (3) layer.lock file keys are parsed as OutputPath (no control or format character); the error is LockError::FilePath { entry } printed as 'the lock's a key of files is missing or not valid (entry N); delete the lock and rebuild', no value. layer.lock, .gitignore and the inputs are reserved after NFD plus ASCII lower case folding. Tests incl. .GITIGNORE and .g U+200C it/config. Spec section 5.3 list of lock fields should mention the entry number (not edited by me; docs-writer or architect). (4) Inputs (new alz-core type, render/write_plan/inputs.rs): every manifest stack entry whose path: source is a relative folder inside the repository (Source::repository_folder, shared with adoption) is an input; absolute path: sources inside the repository are not recognised (open, TASK-84 area). (5) PathGlob parses the pattern and compares the path in NFD then ASCII lower case; ? absorbs following combining marks so it still matches one letter; consequence: E-acute and capital E-acute are one name (as fold() already had it), test only_ascii_case_is_folded now uses sigma. (6) Case-only rename: a listed case variant is the old spelling when the lock lists it and its bytes still match the recorded hash: its Delete is planned first, then the Create; otherwise DiffersOnlyInCase collision (stricter than before: a lock entry alone no longer excuses the variant). Unit tests on a model case-sensitive and case-insensitive file system plus a proptest over both, and a second build plans nothing. The existing property tests do not model variants beyond that. (7) test/acceptance/39-writer-rules.test.ts: three referenceDrift(TASK-55.6.7) cases (case-variant passthrough Settings.json, layer file at Backlog/x.md, lock entry alone does not delete a hand-edited file), pass with the Rust binary, expected-fail against the reference. Evidence: mise run rust:ci exit 0 (fmt, clippy -D warnings, tests, doc, MSRV, deny, audit); mise run lint clean; acceptance against target/release/alz with the JSON reporter: 1044 cases, 256 failing, all 256 listed R6, check exit 0. mise run test failed once with 5 timeouts under load (another build ran in parallel); the same files passed alone (482 passed). Not run: workflows:lint (shellcheck shim has no version in this environment; the tests.yml change is a comment).

Review fixes done (2026-11-24): settings numbers finite with documented ranges (smoke-tested against Claude Code 2.1.323; target-facts §14.6), ls --live probes with hooks, MCP servers and tools off (target-facts §16; per-server findings dropped since no server starts), hook script names in a safe set, the probe's process group ends with the program; the writer creates without replacing and re-checks before each change, lock keys are output paths, reserved names and every in-repo layer folder compared folded, protected paths NFD-folded, case-only renames, CI check fails on unloaded files and below a case floor (1083), test 39 has the case-variant cases. On the merged tree: mise run rust:ci, lint, test (53 files, 1181 passed, 285 expected fail) and alz:check exit 0; acceptance against the release Rust binary 1083 cases, 256 failing, all listed as R6 pending (check exits 0).
<!-- SECTION:NOTES:END -->

## Comments

<!-- COMMENTS:BEGIN -->
author: @architect
created: 2026-11-23T18:10:00Z

---

R5 plan and subtasks TASK-55.6.1 to TASK-55.6.15 are in. Owner questions 1 to 10 are in the notes (2026-11-23), each with options and a recommendation. Blocked on them: R5.1 (TASK-55.6.1), and through it R5.7 and R5.12 to R5.15. R5.2 to R5.6 can start now; R5.5 also needs TASK-76.

---
<!-- COMMENTS:END -->
