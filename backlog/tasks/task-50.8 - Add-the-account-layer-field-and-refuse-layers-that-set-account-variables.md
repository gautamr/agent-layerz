---
id: TASK-50.8
title: Add the account layer field and refuse layers that set account variables
status: Done
assignee:
  - '@claude'
created_date: '2026-11-22T03:33:00Z'
updated_date: '2026-11-22T07:06:00Z'
labels:
  - area:accounts
  - area:security
milestone: m-3
dependencies:
  - TASK-50.1
  - TASK-50.7
  - TASK-51
references:
  - docs/LAYERING-SPEC.md
  - docs/target-facts.md
parent_task_id: TASK-50
priority: high
type: feature
ordinal: 25390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
First slice of TASK-50.2 (split 2026-11-22; mapping of the former criteria in TASK-50.2's notes). The build-time half of account selection: the layer format gains `account`, the build enforces who may declare it (the commit_identity rule, decision-13), and no layer may set the variables that choose the account folder or locate alz's own files and the layer cache. Nothing resolves or applies an account yet; the later slices do. Generated output does not change.

User-facing: a new layer.yaml field, and a new build error that can fail a build that passes today. Both need the minor release that ships TASK-50.2 and a release note (LAYERING-SPEC §7.13 'Layer format').

Out of scope: accounts.yaml and allowed.yaml, alz account which and allow, the cache index for pinned path: layers.

Spec: LAYERING-SPEC §5.1, §7.9, §7.12 'Who may declare it', §7.13 'Account names', 'Who may declare it', 'Layers cannot set the variables', §13 test 24. Facts: docs/target-facts.md §12 item 15 (Codex shell_environment_policy, including the policy-pattern row added 2026-11-22).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 A layer can declare an account name, and alz lint rejects anything in that field other than a name of at most 64 characters (no paths, no secrets)
- [x] #2 Only the first stack entry, or one private or confidential kind: client layer outside the repository, may declare account, and the client layer's value replaces the first layer's whole value, a per-target map included; a declaration by the repo layer (whatever its kind), a group layer, a public client layer or a second client layer fails alz build naming the layers, and alz init prints it as a warning (LAYERING-SPEC §7.13 'Who may declare it', §10 init, §13 test 24, in test/acceptance/24-untrusted-repo.test.ts; the init warning and which layer wins are in test/acceptance/29-account-layer.test.ts)
- [x] #3 A layer that sets CLAUDE_CONFIG_DIR, CODEX_HOME, HOME, XDG_CONFIG_HOME, XDG_STATE_HOME or XDG_CACHE_HOME through settings.yaml (Claude env, or a Codex shell_environment_policy whose set names one or whose exclude pattern matches one), an MCP environment or a passthrough file fails the build naming the layer (LAYERING-SPEC §7.9 and §13 test 24, in test/acceptance/24-untrusted-repo.test.ts)
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
User-facing: new layer.yaml field (layer format) and a new build error that can fail a build that passes today. Generated output does not change. Needs a minor release (0.x) and a release note; the release engineer names the version. Commit: feat(accounts)!: ... (task-50.8), with a BREAKING CHANGE footer for the new build error.

1. Spec (same pull request)
- §5.1: the example gains `account: client-a  # optional; first layer or one client layer only (§7.13)`; the version sentence covers commit_identity and account.
- §7.9: the refusal is built (drop 'Planned'), with every form named: Claude `env` keys; Codex `shell_environment_policy` anywhere in the `codex` tree (profiles included): `set` keys compared ignoring ASCII case, and `exclude` entries and `filters` entries set to `exclude` matched as Codex matches them (target-facts §12 item 15, 'policy patterns' row); MCP `env_from` names; passthrough files (step 4 below). `include_only` and `inherit` stay allowed (a known limit in §7.13 'What a session passes on').
- §7.13: 'Account names', 'Who may declare it' and 'Layers cannot set the variables' say they are built (TASK-50.8); the header note says which parts are built. §13 test 24 names `filters`.
- README unchanged: the field has no effect until TASK-50.11, and the README covers only what the CLI does today.

2. Model
- New src/model/account.ts: `ACCOUNT_TARGETS` ({claude: CLAUDE_CONFIG_DIR, codex: CODEX_HOME}); `type AccountTarget`; `ACCOUNT_NAME = /^[a-z0-9][a-z0-9-]*$/`, at most 64 characters; `isAccountName(text)` (TASK-50.10 reuses it for flags and allowed.yaml); `type AccountMap = Partial<Record<AccountTarget, string>>`; `parseAccount(raw): AccountMap`. A string becomes the same name for every target in ACCOUNT_TARGETS, whatever the manifest's targets (Codex selection needs no Codex adapter, §7.13). A map must be non-empty, use only target keys, and hold string names. null, numbers, lists and nested maps are refused. InputError messages name the field and the rule, never the value, which may be a path or a secret.
- `PROTECTED_VARIABLES` (the six names) in the same file. `isProtectedName(name)` ignores ASCII case. `codexPatternMatches(pattern, name)` mirrors wildmatch 2.6.1: `*` matches any run, `?` one character, and characters compare equal when equal or when their Unicode toLowerCase() is equal.
- src/model/layer-meta.ts: `account` in FIELDS; `LayerMeta.account?: AccountMap`, parsed when the key is present (null is an error, as for commit_identity).

3. Who may declare
- New src/merge/declarers.ts: move `reasons()` out of src/merge/commit-identity.ts as `declarerProblems(layer, index, repoRoot)`. Add a pure `evaluateDeclarations<T>(layers, repoRoot, pick)` that returns { winner?: { layer, value }, refused: { layer, reasons }[], clients: Layer[] }. It writes no diagnostics and runs no git (`isInsideRepo` only touches the file system), so TASK-50.9 and TASK-50.10 can reuse it.
- src/merge/commit-identity.ts uses it. Messages and codes stay byte-identical, and test 26 unchanged is the guard.
- New src/merge/account.ts: `mergeAccount(layers, repoRoot, diags): DeclaredAccount | undefined`, where `DeclaredAccount = { account: AccountMap; layer: Layer }`. Code `account`; messages parallel to commit_identity ("layer 'x' cannot declare account: <reasons>. Only the first layer, or one private layer with kind: client outside the repository, may"; "layers 'a' and 'b' both declare account; only one client layer may"). The winner is the first eligible client layer, otherwise the first layer; the client's whole map replaces the first layer's.
- src/model/config.ts: `ResolvedConfig.account?: DeclaredAccount`, set in src/merge/resolve-config.ts. No adapter reads it.
- src/cli/commands/init.ts: call mergeAccount next to mergeCommitIdentity, so problems are printed as warnings and no identity is written (§10 init row).

4. Layers that set the variables (code `protected-variable`, where = `<layer>:<file>`; the message names the key path and the variable, never a value)
- src/merge/settings.ts: `claude.env` keys. For `codex`, every `shell_environment_policy` map anywhere in the tree: its `set` keys, its `exclude` entries, and its `filters` keys whose value is `exclude`. A pattern that is not a string is an error too, since it cannot be checked.
- src/merge/mcp.ts readServer: an `env_from` entry that names a protected variable.
- src/merge/passthrough.ts: paths after `targets/<target>/` are compared ignoring case (macOS writes `.Claude/Settings.json` over `.claude/settings.json`). Claude `.claude/settings.json` and `.claude/settings.local.json` are parsed as JSON and their `env` keys checked; `.mcp.json`, its `mcpServers.*.env` keys. A file in this list that does not parse is refused. Codex `.codex/config.toml`: alz has no TOML parser (runtime dependencies are yaml and diff only), so it is refused when its text contains `shell_environment_policy` or a protected name, ignoring ASCII case. This is stricter than the spec's set/exclude test.
- lint runs these merges through lintLayer, so `alz lint --layer` reports them too.

5. Tests (fake HOME and an explicit env, so no __MISE_DIFF, DIRENV_DIFF or agent markers reach them)
- New test/acceptance/29-account-layer.test.ts, fixtures test/fixtures/29-account-layer/{personal,client,client-b,group,public-client,repo}:
  (a) accepted shapes: a name, a per-target map, `default`, 64 characters;
  (b) `alz lint --layer` refuses 65 characters, upper case, a leading '-', `~/x`, `/x`, `../x`, `a/b`, a token-like marker value, an unknown target key, an empty map, null, a number and a nested map, and no output contains the refused value;
  (c) the first layer alone wins; the client layer's map replaces the first layer's whole value; a name is normalised to both targets (via computeBuild and config.account);
  (d) generated files are byte-identical with and without a declaration, and neither they nor layer.lock contain the account name;
  (e) init prints a refused declaration as a warning, exits 0 and writes no identity;
  (f) a codexPatternMatches table: `*`, `?ODEX_HOME`, `*home*`, `XDG_*`, `*_HOME`, and no match for `*FOO*`.
- test/acceptance/24-untrusted-repo.test.ts, new cases on fixture 29:
  - an account declared by the repo layer (with kind: client too), a group layer, a public client layer, a second client layer, or a path: link inside the repository that points outside fails the build and names the layers;
  - it.each over the six variables and the forms: claude env; codex set (lower-case key too); exclude `*home*`; a filters entry set to exclude; the policy under codex profiles; mcp env_from; passthrough settings.json, settings.local.json, .mcp.json and config.toml; a case-variant passthrough path. Each exits 1 with `protected-variable` and the layer name;
  - controls that still build: exclude `*FOO*`, include_only, inherit core.
- Unchanged and passing: 26-commit-identity (refactor guard) and 01-determinism.

6. Untrusted inputs and threat-model checks
- Inputs: the `account` field of any layer (repository or remote); settings.yaml trees; mcp.yaml env_from; passthrough file paths and contents (JSON.parse only, nothing executed). All of them are repository or layer content.
- Checklist: 1 paths: no new writes. 2 subprocesses: none added. 4 secrets: no value is ever echoed, and the leak guard already scans the layer.yaml of public layers (§9.2). 5 security floor: the who-may-declare rule mirrors §7.12, plus the six variables. 7 symlinks: a link into the repository counts as inside (isInsideRepo).
- Error text: layer names are NAME_PATTERN-checked and `where` goes through formatDiagnostic's stripControl.

7. Verification (each command and its result goes in the notes)
- mise run test: all files, 24, 26 and 29 included.
- mise run lint.
- mise run alz:build, then mise run alz:check. This repository's stack sets none of the six variables (grep of .layer and the personal layer, 2026-11-22).
- A confidentiality grep of the diff.
- Reviewer and security-reviewer pass.
- Tick AC #1 to #3 only with that evidence.

Risks: the declarers refactor touches commit identity (guarded by test 26 unchanged); the TOML text rule may refuse a harmless Codex passthrough (it names the rule and the layer; the fix is a first-class item).
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
2026-11-22 (@architect): choices made within what the spec allows (decide, then notify; reported in the pull request):
- A Codex `filters` entry set to `exclude` is treated as an exclude pattern, because it drops the variable the same way (new fact: docs/target-facts.md §12 item 15, 'policy patterns' row; SOURCE at tag rust-v0.169.0). Patterns match as Codex matches them (case-insensitive, `*` and `?`). `set` keys are compared ignoring ASCII case, which is Codex's Windows behaviour and stricter on Unix.
- 'An MCP environment' is mcp.yaml `env_from`, the only environment form mcp.yaml has; `env` values are already refused.
- Passthrough 'settings or config files': Claude `.claude/settings.json`, `.claude/settings.local.json` and `.mcp.json` are parsed as JSON. Codex `.codex/config.toml` is refused when its text names `shell_environment_policy` or a protected variable, because alz has no TOML parser and adding one needs a decision (runtime dependencies are yaml and diff only). Paths are compared ignoring case.
- `account: <name>` means every account target alz knows (claude, codex), not only the manifest's targets. An empty map is a schema error.
- alz init prints account declaration problems as warnings, like commit_identity problems (§10 init row).

2026-11-22 owner: the strict text rule for a passthrough Codex config.toml is accepted (refuse any mention of shell_environment_policy or the six variables, naming the layer and the word); a TOML parser may come later.

2026-11-22 (@claude, implementer): built; not committed, version not bumped.

What changed
- src/model/account.ts (new): ACCOUNT_TARGETS, parseAccount, isAccountName, PROTECTED_VARIABLES, isProtectedName, codexPatternMatches (wildmatch 2.6.1 rules), matchedProtectedNames. src/model/layer-meta.ts: account field. src/model/config.ts and src/merge/resolve-config.ts: ResolvedConfig.account (no adapter reads it).
- src/merge/declarers.ts (new): declarerProblems, evaluateDeclarations (pure), reportDeclarations. src/merge/commit-identity.ts uses it; test 26 passes unchanged. src/merge/account.ts (new): mergeAccount, code 'account'. src/cli/commands/init.ts calls it, so init warns and writes no identity.
- src/merge/protected-variables.ts (new), called from settings.ts, mcp.ts and passthrough.ts; code 'protected-variable'. lint reports it through lintLayer.
- Spec: §5.1, §7.9, §7.13 (header and three bullets marked built), §10 init row, §13 test 24 (names filters; built and planned parts separated). docs/target-facts.md item 15 policy-pattern row: last cell now 'yes'.
- Tests: test/acceptance/29-account-layer.test.ts (49 cases) with test/fixtures/29-account-layer/; test/acceptance/24-untrusted-repo.test.ts (+125 cases: who may declare, 17 forms x 6 variables, controls that still build, unchecked inputs).

Deviations from the plan (smallest change each; the owner may overrule)
1. Fixture edit: test/fixtures/15-protected-paths/personal/targets/claude/.claude/settings.local.json held plain text, which the new 'a listed JSON file must parse' rule refuses. It is now valid JSON ({"marker": ...}); test 15 asserts nothing about its content.
2. Codex config.toml text rule: a protected variable name counts as mentioned only as a whole name (not preceded or followed by a letter, digit, '_' or '-'), ignoring ASCII case. A bare substring test would refuse any file containing 'home' (a path, 'homepage'). shell_environment_policy is still a plain case-insensitive substring test.
3. Added to that rule: a file with a \u, \U or \x escape is refused as unchecked, because TOML quoted keys let such an escape spell shell_environment_policy or a name past a text search. This closes a bypass of the stated guarantee; a TOML parser would remove the need.
4. Codex filters: an entry is checked unless its value is exactly 'include' (plan: only when the value is 'exclude'), so a misspelt action cannot hide a pattern. An exclude or filters value of the wrong type is refused as unchecked.
5. An exclude/filters pattern that matches several protected variables gives one error per variable.

Verification (2026-11-22)
- mise run test: 28 files passed; 350 tests passed, 1 skipped.
- mise run lint: biome check (118 files, no fixes) and tsc --noEmit clean.
- mise run alz:build: built 58 files, 0 created, 0 updated, 0 deleted (this repo's stack passes the new checks; no .layer/ change). mise run alz:check: check passed, 58 generated files match.
- Confidentiality grep of the diff and new files: only reserved example.com/example.invalid values.
- Not done: reviewer and security-reviewer pass (plan step 7).

Open points
- Release (for the release task that ships TASK-50.2): a MINOR bump (0.x; the release engineer names the version). Release note: 'New layer.yaml field account (a name of at most 64 characters, or a per-target map; only the first layer or one private client layer outside the repository may declare it). New build error: a layer that sets CLAUDE_CONFIG_DIR, CODEX_HOME, HOME, XDG_CONFIG_HOME, XDG_STATE_HOME or XDG_CACHE_HOME through settings.yaml, an MCP env_from or a passthrough settings, .mcp.json or Codex config.toml file now fails the build. This can fail a build that passes today. The field has no effect on output yet.' Commit: feat(accounts)!: ... (task-50.8) with a BREAKING CHANGE footer.
- A Codex passthrough config.toml that mentions shell_environment_policy or one of the names is refused even when harmless; the fix is a first-class item or a TOML parser (needs a decision).

2026-11-22 (@claude, implementer), round 2 after review and security review. Not committed, version not bumped.

Fixed
- S1: a variable name must be ^[A-Za-z_][A-Za-z0-9_]*$ in claude.env, policy set, mcp env_from, passthrough settings env and .mcp.json env, and in the new forms below. A name like CODEX_HOME=/x or one with NUL is refused naming the key path, never the text. N6: an env, set, exclude or filters value of the wrong type is refused as 'cannot be checked'.
- S2: JSON check for every passthrough path ending in .claude/settings.json or .claude/settings.local.json, every .mcp.json and every .claude-plugin/*.json, at any depth and for any target (root env, mcpServers env and any nested env); TOML text rule for every *.toml under targets/codex/; non-ASCII in a path under .claude/, .codex/, .claude-plugin/ or in a .mcp.json path is refused (N1). Paths compare with ASCII case folding only. Spec §7.9 lists the covered files.
- S3: native agent frontmatter claude.mcpServers (map and list shapes), codex.mcp_servers and a codex shell_environment_policy are checked; also .claude/agents/*.md passthrough frontmatter (unparsable frontmatter is refused).
- S4/N5: §7.9 has a 'Limit' paragraph (declarative config only; hooks and MCP command/args run code; the allow step, TASK-50.11, is the control; only set is a firm guarantee among the policy keys). §7.13 'Layers cannot set the variables' says the same instead of 'through any other item'.
- N2: evaluateDeclarations returns { winner, conflict, refused, clients }; no winner when two client layers declare or the first layer's declaration was refused. A refused later layer (for example a group) does not remove the winner. Unit tests in test 29.
- N3: Object.hasOwn for RESERVED lookups in settings.ts; passthrough lookups no longer use a keyed object. Tests with __proto__, constructor and toString names give no crash.
- N8: §10 init row says a refused account declaration also stops init from setting the commit identity.
- M1: a line-ending backslash in a Codex .toml is refused with the escapes (test with a triple-quoted CODEX_ then backslash newline then HOME). M2, L1: tests for a policy-table-only config.toml, filters HOME: exlude, filters: [HOME], and a \U and a \x escape.
- L2: spec status header and 'Account names' now say what is built; the 'default always means the default account' and 'names read from the environment or allowed.yaml' sentences are tagged planned (TASK-50.10). L3: spec §13 test 24 and AC #2 name test 29 for the init warning (AC #2 text reworded through the CLI, then re-ticked). L4: README Safety bullet. L5: a refused MCP server no longer also gets the 'mcp.enable lists ... no layer defines it' error.
- Nits: test 24 matches 'names <VARIABLE>;'; §7.9 states ASCII case folding and defines a whole name; a test with a confidential client layer as declarer.

Deviation: the JSON checks apply to the listed file names for any target, not only claude (stricter, simpler).

Release note, to add to the plan for the release that ships TASK-50.2 (minor bump, version named by the release engineer): New layer.yaml field 'account' (a name of at most 64 characters or a per-target map; only the first layer or one private or confidential client layer outside the repository may declare it; no effect on output yet). New build failures that can fail a build that passes today: (a) a layer's .claude/settings.json, .claude/settings.local.json, .mcp.json or .claude-plugin/*.json passthrough file that is not valid JSON, at any depth; (b) a Codex passthrough .toml file under targets/codex/ that mentions shell_environment_policy, or CLAUDE_CONFIG_DIR, CODEX_HOME, HOME, XDG_CONFIG_HOME, XDG_STATE_HOME or XDG_CACHE_HOME as a whole word anywhere, which includes Linux /home/... paths (N4); (c) such a .toml file with any \u, \U or \x escape or a backslash at the end of a line; (d) any layer that sets one of the six variables in claude.env, a Codex shell_environment_policy (set, exclude or filters), an mcp.yaml env_from, agent native mcpServers, or those passthrough files; an env or policy key that is not a plain variable name (for example NAME=x); an env, set, exclude or filters value of the wrong type; and a non-ASCII character in a passthrough path under .claude/, .codex/ or .claude-plugin/ or in a .mcp.json path. Commit: feat(accounts)!: ... (task-50.8) with a BREAKING CHANGE footer.

Follow-up (left out on purpose, N7): check and render read layer files at different times; record it for a later task.

Verification, 2026-11-22
- mise run lint: biome check (118 files) and tsc --noEmit clean.
- mise run alz:build: built 58 files, 0 created, 0 updated, 0 deleted. mise run alz:check: check passed, 58 generated files match the layers and the lock.
- All test files except 26 and 28: 26 files, 321 tests passed, 1 skipped (4 s). Files 24, 26 and 28 together once: 344 passed. Test 24: 219 passed. Test 29: 55 passed.
- mise run test as a whole did not pass in this session's later runs: single tests in 26-commit-identity and 28-cache-integrity (random ones, all spawning git) stall for about 145 s and hit the 20 s timeout. The same two files fail the same way (4 failures, 965 s) on an unmodified checkout of HEAD, so the cause is this machine's git or file system, not this change; both files pass when the machine is not stalling (125 tests passed in 28 s earlier). Needs a rerun on a healthy machine or in CI.

2026-11-22 (@claude, implementer), round 3 (both reviewers' blocker). Not committed, version not bumped.

- Blocker: the TOML text rule and the .claude/agents/*.md frontmatter check now run for every target, not only codex and claude, because an adapter writes any path a layer ships under its own target (for example targets/claude/.codex/config.toml). The agent match is .claude/agents/**/*.md at any depth. checkPassthroughFile no longer takes the target. Test-24 FORMS rows: targets/claude/.codex/config.toml, a *.toml of any name under the claude target, a .claude/agents file under the codex target, and one at depth.
- A non-ASCII character anywhere in a passthrough path (the whole path, target name included) is refused for every target; this replaces the folder-based trigger and covers .mcp.jſon. Cases added. The macOS APFS fact (SMOKE, 2026-11-22: a file created as .mcp.jſon opens as .mcp.json, Settings.json opens as settings.json) is recorded in docs/target-facts.md §12 ('File names, macOS' row) and cited in spec §7.9.
- A mcpServers env problem in a .mcp.json is reported once (the nested walk skips the root mcpServers). A JSON file nested deeper than 32 levels is refused as 'cannot be checked' once, instead of the walk stopping silently.
- Spec §7.9, §13 test 24 and the README Safety bullet match.

Release note, replacement for (b): a .toml passthrough file under any target (for example targets/claude/.codex/config.toml or targets/codex/agents/worker.toml) that mentions shell_environment_policy, or CLAUDE_CONFIG_DIR, CODEX_HOME, HOME, XDG_CONFIG_HOME, XDG_STATE_HOME or XDG_CACHE_HOME as a whole word anywhere, which includes Linux /home/... paths. Also new: (e) any passthrough path with a non-ASCII character, for every target; (f) a .claude/agents/**/*.md passthrough file whose frontmatter does not parse; (g) a JSON settings, .mcp.json or plugin file nested deeper than 32 levels. (a), (c) and (d) as in round 2.

Follow-up, no code (limit): a plugin's plugin.json can name another file for mcpServers, and that file is not read or checked. Record it in the follow-up with N7.

Verification, 2026-11-22 (after round 3)
- mise run test: 28 files passed; 475 tests passed, 1 skipped (23 s). Git did not stall in this run; it had stalled in earlier runs of 26-commit-identity and 28-cache-integrity, also on an unmodified HEAD checkout.
- mise run lint: biome check (118 files) and tsc --noEmit clean.
- mise run alz:build: built 58 files, 0 created, 0 updated, 0 deleted. mise run alz:check: check passed, 58 generated files match the layers and the lock.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Layers may declare account (one name for every target, or a per-target map), under the commit_identity who-may-declare rule now shared in src/merge/declarers.ts (commit identity unchanged, test 26 untouched; evaluateDeclarations reports a conflict and no winner for resolution to reuse). alz lint and build refuse invalid names without repeating them; init warns. No layer may set CLAUDE_CONFIG_DIR, CODEX_HOME, HOME or XDG_CONFIG_HOME/STATE_HOME/CACHE_HOME through settings.yaml env, Codex shell_environment_policy (set, exclude, include_only, filters), MCP env_from, agent frontmatter MCP servers, or passthrough files of any target (Claude settings and .mcp.json and plugin JSON at any depth, every .toml by a strict text rule, agent files); env keys must be plain names; non-ASCII passthrough paths are refused (APFS folds case and long s, SMOKE). Spec §5.1, §7.9 (with an honest Limit: hooks and MCP commands run code), §7.13, §10, §13; target-facts §12; README Safety. Tests: 29-account-layer (new) and 24-untrusted-repo; 475 passed, 1 skipped; lint and alz:check clean. Reviewed in three rounds by reviewer and security-reviewer. Release note and breaking changes in the notes; follow-ups: plugin.json-named MCP files, check vs render reads.
<!-- SECTION:FINAL_SUMMARY:END -->
