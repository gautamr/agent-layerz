---
id: TASK-55.5
title: 'R4: Merge rules in Rust'
status: In Progress
assignee:
  - '@architect'
created_date: '2026-11-22T12:54:00Z'
updated_date: '2026-11-23T16:13:00Z'
labels:
  - area:rust
milestone: m-7
dependencies:
  - TASK-55.4
parent_task_id: TASK-55
priority: medium
ordinal: 37390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
doc-5 phase R4: every merge rule, protected variables, who may declare.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Every merge rule of spec §7 has Rust tests, with property tests for its law (doc-6 §3.5), and each merge case of tests 02 to 11, 17 to 19, 26 and 29 has a Rust test with the same verdict (list in the notes)
- [ ] #2 The cases of tests 02 to 11, 17 to 19, 26 and 29 that alz lint can reach pass against the Rust binary; the rest of those files pass by R5 (TASK-55.6), and test 26's identity cases (§9.4) by R6 (TASK-55.7)
- [ ] #3 Settings files accept only the known target names at the top level and MCP files only server ids of the spec's grammar; any other top-level key is an error naming the file, whatever the key, and the merged settings and servers are ordered maps of validated keys
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
### Scope
doc-5 R4: every merge rule of spec §7 (7.1 instructions, 7.2 agents, overlays, native fields and tool narrowing, 7.3 skills and slots, 7.4 commands, 7.5 hooks with locks, 7.6 permissions with the floor, 7.7 MCP servers and the allowlist, 7.8 models, 7.9 settings and protected variables, 7.10 rules, 7.11 passthrough), and the shared "who may declare" rule of §7.12 and §7.13. Out of scope: rendering, adapters, adoption, `build`/`check`/`explain`/`doctor` (R5, TASK-55.6), git identity writes and `alz account which/allow` (R6, TASK-55.7). src/ is never changed (decision-30); the reference is read for behaviour only (decision-22). Work runs as subtasks R4.1 to R4.15, in order, in one pull request.

### What exists and what is missing (current code, 2026-11-23)
- alz-core: YAML tree with limits (decision-28), JSON tree (decision-33), LayerMeta, Manifest (mcp.enable as free text), Lock, ResolvedStack, AccountDeclaration, CommitIdentity types, PROTECTED_VARIABLES and the Codex pattern match (account/protected.rs, "the checks that use it are R4's"), TargetName, diagnostics with QuotedKey/Stripped.
- alz-io: the layer walk hashes every file it reads but keeps only digests (ResolvedLayer holds LayerFiles). R4 needs the bytes: the walk keeps the bytes it hashed, so the merge reads exactly the content the lock check covered (no second read, no gap between check and use).
- No Markdown frontmatter parser, no item types, no merge.

### Module layout (doc-6 §2)
alz-core:
- `markdown.rs` + `markdown/`: `MarkdownDoc { front: Frontmatter, body: Body }` from text: CRLF to LF, `---` open and close, frontmatter must be a mapping, the YAML limits and tag rules of every YAML file.
- `layer/contents.rs`: `LayerContents` (BTreeMap<SafeRelPath, Bytes>), with `files()` giving the LayerFiles digests, and `ItemPath`, a closed enum classifying each path of §4.1 (Instruction, Agent, AgentOverlay, Skill, Command, Rule, Hooks, Permissions, Mcp, Models, Settings, Passthrough { target, path }, ClaudeExtra, Other).
- `item.rs` + `item/`: one module per kind, each with its validated types and a pure per-layer reader `read(&LayerView) -> Checked<LayerItems>`: instruction, agent (Agent, Overlay, ToolGrant, NativeFields), skill (Skill, Command), rule, hook, permission, mcp, model, settings, passthrough.
- `merge.rs` + `merge/`: `algebra.rs` (the structures below), one module per kind (`absorb(merged, layer, items) -> Checked<Merged>`), `declarers.rs` (who may declare), `stack.rs` (the fold and `Merged`).
- `secret.rs`: the fixed token shapes of the built-in secret scan (hand-written matchers, no regex), used by mcp.yaml (§7.7) and the leak guard.
- `leak_guard.rs` (R4.14, see owner question 2).
alz-io: the walk returns `LayerContents`; `Placement` (inside or outside the repository, by the real path, case variants and links, as §7.12 counts them) per resolved layer.
alz: `command/lint.rs` runs the per-layer readers (R4.15).
alz-core's clippy.toml gains `disallowed-types` for std HashMap and HashSet, so output order cannot depend on hashing.

### Types and states
- Typestate (doc-6 §3.3): `ResolvedStack` (R3) -> `merge(&ResolvedStack, &Manifest, &Placements) -> Merging` -> `Merged`. `Merging { merged: Option<Merged>, diagnostics: Diagnostics }`: every kind is merged and every problem reported in one run, as the reference does; `merged` is `None` when any error was found. `Merged` has private fields and no public constructor, so R5 can render only a merged stack.
- `Merged`: instructions (Vec<InstructionBlock> in stack order, then name order), agents (BTreeMap<AgentName, MergedAgent>), skills and commands (BTreeMap<SkillName, SkillDir>), rules (BTreeMap<RuleName, MergedRule>), hooks (Vec<MergedHook>, stack order then file order), permissions (MergedPermissions), mcp (defined: BTreeMap<McpServerId, McpServer>; enabled: Vec<McpServerId> in mcp.enable order), models, settings (BTreeMap<TargetName, SettingsTree>), passthrough (BTreeMap<TargetName, BTreeMap<SafeRelPath, PassthroughFile>>), claude extras, commit identity and account (`Declared<T>`). Every item carries `Provenance` (layer name and action per contribution), which `alz explain` needs in R5 (test 03 case 2).
- Validated keys everywhere: a map whose keys the spec names is keyed by a validated type in a BTreeMap. settings.yaml: BTreeMap<TargetName, SettingsTree>; any other top-level key is a `settings` error naming the file, whatever the key, and the layer's settings are not merged. models.yaml aliases: BTreeMap<TargetName, ModelId>. mcp.yaml: BTreeMap<McpServerId, McpServer> (grammar: owner question 1). SettingsTree is alz's own ordered tree (BTreeMap maps), never a structure with inherited keys.
- Sum types: `Event = Neutral(NeutralEvent) | Native { target, name }`; `HookAction = Run(SafeRelPath) | Command(CommandLine)` (exactly one, by construction); `HookEntry = Define(Hook) | Disable(HookId)`; `OverlayMode = Extend | Replace`; `InstructionMode = Add | Replace | Remove`; `RuleMode = Replace | Extend`; `PermissionMode` with `Ord` by strictness; `Transport = Stdio { command, args, env_from } | Http { url } | Sse { url }` (env_from only on stdio, by construction); `Declared<T> = None | Winner { layer, value } | Conflict`.
- Newtypes: AgentName, SkillName (kebab-case), RuleName, HookId, McpServerId, ToolGrant { tool: NeutralTool, scope: Option<ToolScope> }, EnvVarName (`^[A-Za-z_][A-Za-z0-9_]*$`, §7.9).

### The merge algebra (doc-6 §3.5) and the laws as property tests
- `Last<T>` (scalars, most specific wins): monoid, identity "not set"; law: associativity and identity.
- `OrderedSet<T>` (allow/ask/deny lists, rule paths, hook order, deduplicated lists): union keeping first occurrence; laws: associativity, idempotence, commutativity as sets.
- `ListEdit<T>` (`+[..]`, `-[..]`, `[..]` for skills and tools): edits compose; law: applying e1 then e2 equals applying their composition, and composition is associative.
- `Replace<K, V>` (skills, commands, rules without extend, MCP servers, passthrough paths): map union where the more specific value wins per key; laws: associativity, identity.
- `DeepMerge` (settings, native fields, models): maps merged key by key, lists concatenated and deduplicated by structural equality, anything else most specific wins; law: associativity over kind-consistent trees, and idempotence (owner question 5).
- Floor (deny rules, locked hooks, the default_mode floor): a lattice with a monotone merge; laws: for any stack and any further more specific layer, the merge either refuses or keeps every deny of the stack, every locked hook enabled, and a default_mode no looser than the floor.
- Narrowing: `covers` is a preorder on ToolGrant (reflexive, transitive); law: without allow_widen in a non-public layer every grant of a merged agent is covered by a grant of its base.
- Who may declare: at most one winner, and a winner is the first layer or a private `kind: client` layer, both outside the repository.
- Determinism: `merge(s) == merge(s)`, and no output depends on hash order (the clippy ban).
Generators live in each module's tests (proptest 1.12.0, decision-35); shrinking stays on.

### Diagnostics and who may declare
- One diagnostic per problem, `<level> [<code>] <where>: <message>` (§10.1, decision-26), `<where>` the layer and file. Codes are spec-owned: R4.1 lists the merge codes in §10.1 (the reference uses instructions, agent, agent-overlay, agent-replace (warning), tool-widen, skill, command, slot, slot-missing, rule, hooks, permissions, permission-floor, mcp, models, settings, protected-variable, leak-secret, frontmatter, passthrough (warning), unresolved).
- Names that passed their grammar print as validated types (Printable); keys that did not print as QuotedKey; values a person needs to see to act (a deny rule, a tool entry) print stripped (class 1); a value refused for a protected variable or a secret is never printed (§7.9).
- `unresolved`: a problem that needs a more general layer (an overlay without its base, a disable of an unknown hook, a role routed to an unknown alias). `alz lint` drops it, as the reference does.
- Who may declare (§7.12, §7.13, decision-13): `merge/declarers.rs` evaluates, purely, the declarations of commit_identity and account from each layer's position, kind, visibility and `Placement` (from alz-io). The first layer, or one private client layer, both outside the repository, may declare; the client's whole value wins; several clients, or a refused first layer, give `Conflict`. R6 consumes the winner. Other who-may rules stay with their kind: allow_widen only in a non-public layer (R4.6), a floor only binds more specific layers (R4.9), a locked hook cannot be disabled by any later layer (R4.8).

### How the merged model feeds R5
R5's renderer and adapters take `&Merged` only. Each item is target-neutral with its provenance; native fields and passthrough are already split per TargetName; the MCP allowlist is applied (enabled ids in order); hook scripts are SafeRelPaths inside their layer; settings are ordered trees ready for the JSON emitter. The instructions_max_lines budget counts the composed AGENTS.md, Commits section included, so it is checked by R5's render, not here.

### Untrusted input handled
Every layer file (item Markdown, frontmatter, YAML, passthrough JSON and TOML text), the manifest's mcp.enable, and all keys and strings in them. All of it is parsed into validated types before anything else uses it; nothing reaches the file system or a subprocess in R4 (alz-core is pure).

### Tests
- Unit and proptest laws per module (above).
- Each merge case of tests 02 to 11, 17 to 19, 26 and 29 gets a Rust test with the same verdict, over in-memory LayerContents stacks; R4.15 writes the list into this task's notes.
- alz-io integration tests from the fixture directories for the walk bytes and Placement.
- assert_cmd tests for the per-layer checks of `alz lint`.

### Acceptance cases R4 must pass against ALZ_BIN (baseline run 2026-11-23, target/release/alz at 6c5c9fe)
Of the 199 cases in tests 02-11, 17-19, 26 and 29, 84 pass on the Rust binary now (10: 1, 26: 37, 29: 46) and must keep passing. The only failing cases that drive `alz lint` are test 10's "forbidden pattern" and "fake token" (leak guard and secret scan, owner question 2). Every other failing case drives build, check, explain, doctor or init, so it passes by R5 (TASK-55.6), or by R6 for test 26's §9.4 cases (owner question 4). R4 also adds test 37 (below), whose lint cases pass in R4.
New acceptance case (R4.15, new §13 entry and test/acceptance/37-settings-targets.test.ts): "a settings or MCP file whose top-level key is not a known target is refused, whatever the key", as a table of keys through `alz lint --layer`, and the same through `alz build` (no output written), both marked referenceDrift; the build rows pass by R5.

### Spec changes in the same PR (R4.1)
The known-target rule of settings.yaml and the key rule of mcp.yaml (§7.7, §7.9); validated keys of models.yaml aliases (§7.8); the merge diagnostic codes (§10.1); the answers to the owner questions; the new §13 entry. CHANGELOG: the refusals are user-facing (layers that built before may now fail): minor version bump at cutover (doc-5).
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
decision-30: R3 closes TASK-52, TASK-53 and TASK-63 (fixed in Rust only).

Architect (2026-11-23): criteria restated per the owner's answer 6 in TASK-55.3's notes: R4 is judged by Rust tests plus the acceptance cases its CLI can reach, and the full suite files must pass by R5. The known-target criterion is unchanged in wording and meaning; it is now #3.

Lead (2026-11-23): TASK-52, 53, 63, 69 and 71 keep one criterion each that only R5's alz build/check can meet (and TASK-69's Linux acceptance case needs a Linux run of the TypeScript suite, TASK-4), so R5 (TASK-55.6) depends on them instead of R4. TASK-68 and TASK-70 close with R3.

Architect (2026-11-23): R4 planned (plan above) and split into TASK-55.5.1 to TASK-55.5.15 (R4.1 to R4.15), one chain in order; R4.1 (spec) feeds R4.6, R4.10, R4.14 and R4.15 and waits for the questions below. R4.2 to R4.5 can start now. No decision recorded yet: each lasting choice below waits for the owner; a regex crate needs the dependency intake check first (R4.14).

Baseline (2026-11-23, `ALZ_BIN=target/release/alz` at 6c5c9fe, `vitest run --project acceptance` on tests 02-11, 17-19, 26, 29): 199 cases, 84 pass (10: the clean layer; 26: 37, the 34 schema cases through `lint --layer`, apostrophe and valid declaration, and the three `§13.28 a failed fetch through alz lint` cases; 29: 46, every schema case through `lint --layer`), 115 fail. Of the failing ones only test 10's "forbidden pattern" and "fake token" drive `alz lint`; every other drives build, check, explain, doctor or init. So no merge-rule case of these files is reachable through lint: the merge verdicts are proven by Rust tests in R4 and by these files in R5.

Owner questions (options; recommendation first):
1. AC #3 says settings and MCP files accept only target names at the top level, but the top level of mcp.yaml is server ids (§7.7), and the manifest's mcp.enable is free text today (McpName). (a) Recommended: settings.yaml takes only claude and codex; mcp.yaml keys are server ids with a grammar such as `^[A-Za-z0-9][A-Za-z0-9_-]{0,63}$` (letters, digits, `_` and `-`, not starting with `_` or `-`), the same type for mcp.enable, refused keys printed as quoted keys; both merge into ordered maps of validated keys. AC #3 would then read "settings files accept only known targets and MCP files only server ids". (b) Server ids stay free text, kept as ordered-map keys and printed stripped. (c) Kebab-case server ids, like skills (refuses ids with `_` that people use).
2. Test 10's "forbidden pattern" and "fake token" cases drive `alz lint`, so AC #2 puts them in R4, but the leak guard (§9.2) is not in doc-5's R4 line and forbid_patterns need a regex engine. (A) Recommended: R4.14 does the leak guard in R4, after a dependency intake check and a decision on a regex crate; the spec states the pattern syntax (Rust regex has no look-around or backreferences, so some patterns written for the reference would be refused: user-facing). (B) The secret scan in R4 (it is needed for mcp.yaml anyway), so "fake token" passes in R4, and forbid_patterns move to R5. (C) The whole leak guard in R5; R4.14 is dropped.
3. Scope of stack-mode `alz lint`: (A) Recommended: as the reference and §10 say, each layer alone, `unresolved` dropped; cross-layer verdicts (floor, locks, narrowing) are proven by Rust tests and reach the CLI through build in R5. (B) Stack-mode lint also runs the whole merge (new behaviour, a spec change and a minor bump at cutover); still no acceptance case of these files would reach it, since they drive build.
4. AC #2 says the rest of test 26 passes by R5, but its §9.4 cases (check on identities, build --adopt writing .git/config, init) need R6's identity work (doc-5). Recommended: restate as "by R5, and test 26's identity cases by R6 (TASK-55.7)". Alternative: move the identity writes into R5.
5. Settings deep merge (§7.9): when a key changes kind between layers (a list, then a scalar, then a list) the left fold gives the last list, but merging the last two first gives both lists, so the merge is not associative; and the reference deduplicates list items by their JSON text, so two mappings with keys in a different order count as different. (A) Recommended: keep the reference's left fold (a later kind replaces), state the associativity law for kind-consistent trees only, and deduplicate by structural equality of ordered trees (the spec says so). (B) A kind change of one key between layers is a `settings` error, which makes the merge associative (stricter, user-facing).
6. Tool narrowing (§7.2) does not say how one scope covers another. Recommended rule for the spec: a path scope covers another when equal, or when it ends in `/**` and the other lies below its prefix, with `*` never crossing `/`; a command scope (bash) covers another when equal, or when it ends in ` *` and the other starts with its prefix and holds no shell control operator (`;`, `&`, `|`, a line feed, a backtick, `$(`, `<`, `>`). Cases where the reference accepts more become referenceDrift. Alternative: keep the reference's prefix rule.

Owner answers (2026-11-23): (1) MCP server ids follow a strict grammar ^[A-Za-z0-9][A-Za-z0-9_-]{0,63}$, shared with mcp.enable; any other key is an error naming the file; AC #3 reworded to match. (2) The leak guard (forbid_patterns) and the secret scan are both ported in R4, with a regex crate decision; the spec states the pattern syntax and refuses look-around and backreferences with a clear error. (3) Stack-mode alz lint checks each layer alone, as the reference; merge errors show in build (R5); no spec change. (4, lead) Test 26's §9.4 identity cases pass by R6; AC #2 reworded. (5) Settings deep merge keeps the reference's left fold (a later kind replaces); the laws are stated for kind-consistent trees; list items are deduplicated by structural equality. (6) Tool scopes use the stricter containment rule: path scopes compare by whole segments (docs/** never covers docs-private/), and a narrowed command scope may not add shell operators beyond its base; the reference's prefix rule is reference drift, described in the open on R4.6's task.

Architect (2026-11-23, after the owner's answers): the answers are folded into R4.3, R4.6, R4.10, R4.11, R4.14 and R4.15, with their descriptions and criteria rewritten. R4.11 adds regex-lite for the secret scan, and R4.14 uses it for forbid_patterns. decision-38 is recorded as accepted: regex-lite =0.1.9, std and string, with size and nest limits; its intake is in the decision's Context. R4.1's spec text and CHANGELOG are on the branch and not committed. R4.2 is in progress with the implementer; I did not touch its files.

R4 merge cases to Rust tests (implementer, R4.15, 2026-11-23; TASK-55.5 AC #1). All cases of tests 02-11, 17-19, 26 and 29 other than the lint-driven ones drive build, check, explain, doctor or init and pass against ALZ_BIN by R5 (TASK-55.6), or by R6 for test 26's section 9.4 cases; the merge verdict of each is given by this Rust test (alz-core, crate path crates/alz-core/src/).
Test 02 (instructions, 1 case): merge::instructions::tests personal_client_and_repo_give_three_blocks_in_stack_order_and_remove_drops_one, the_blocks_compose_the_text_of_the_expected_agents_md, the_removed_block_is_gone_and_the_others_keep_who_made_them. The CLAUDE.md half is the renderer's (R5).
Test 03 (agent overlay, 2 cases): case 1 -> merge::agents::tests a_base_a_skills_overlay_and_a_body_overlay_give_one_agent and the_body_has_one_section_per_layer_under_the_layer_kind_and_name; case 2 (explain agent architect) -> the_agent_lists_its_three_sources_in_stack_order. The rendered file, the write:docs/** warning and the explain text are R5.
Test 04 (tool narrowing, 4 cases): 'an overlay that adds a tool fails' -> merge::agents::narrowing_tests an_overlay_that_adds_a_tool_fails_naming_the_agent_the_tool_and_the_layer; 'removes a tool or narrows a scope passes' -> an_overlay_that_removes_a_tool_or_narrows_a_scope_passes; the two reference-drift cases (trailing-star path scope, chained shell command) -> merge::algebra::narrowing::tests a_path_scope_covers_by_whole_segments and a_command_scope_covers_what_its_star_stands_for_unless_it_holds_an_operator.
Test 05 (skill override, 1 case): merge::skills::tests case_05_the_repo_skill_replaces_the_personal_one, case_05_the_provenance_lists_the_overridden_layer_first, the_whole_directory_is_replaced_and_files_of_the_older_one_are_dropped. The doctor half (user-dir collision) is R5.
Test 06 (slots, 2 cases): merge::skills::tests case_06_a_missing_slot_fails_with_strict_slots and case_06_a_missing_slot_gives_a_stub_and_a_warning_without_strict_slots. The files on disk (stub text) are the renderer's (R5).
Test 07 (permission floor, 2 cases): merge::permissions::tests case_07_a_repo_layer_removing_a_personal_deny_fails_naming_the_layer_and_the_rule and case_07_allow_ask_and_deny_lists_combine_across_layers.
Test 08 (locked hook, 2 cases): merge::hooks::tests case_08_disabling_a_locked_hook_fails_naming_the_layer and case_08_disabling_an_unlocked_hook_removes_it_and_keeps_the_locked_one.
Test 09 (MCP allowlist, 1 case): merge::mcp::tests a_server_defined_but_not_enabled_is_not_in_the_enabled_list.
Test 10 (leak guard, 3 cases, drive alz lint, pass now): leak_guard::tests and crates/alz/tests/lint.rs (a_public_layer_with_a_forbidden_pattern_fails_naming_the_file_and_the_entry_only and the secret-shape tests); secret::tests each_shape_is_found_by_its_own_kind.
Test 11 (visibility, 1 case): stack::resolved::tests a_public_layer_after_a_private_one_is_the_error_visibility_at_its_source (R3's stack validation; the case drives build, R5).
Test 17 (rules, 3 cases): merge::rules::tests a_repo_rule_replaces_the_personal_one_whole and an_extend_rule_appends_a_section_and_unions_the_paths; case 3 (a hand-written .claude/rules file is a collision) is a build case with no merge verdict (R5).
Test 18 (native fields, 2 cases): merge::agents::tests claude_fields_pass_through_and_overlays_deep_merge_them and a_claude_key_with_a_neutral_field_is_refused_naming_the_key.
Test 19 (hook commands, 2 cases): merge::hooks::tests case_19_command_hooks_are_kept_as_written_with_the_project_dir_variable, case_19_a_native_event_keeps_its_matcher_and_runs_on_its_target_only, case_19_a_hook_with_both_run_and_command_is_refused.
Test 26 (commit identity, 121 cases): the merge verdicts -> merge::declarers::tests (23 tests) a_declaration_in_the_repo_layer_is_refused_whatever_its_kind_says, a_declaration_in_a_group_layer_is_refused_and_names_its_kind, a_public_layer_that_calls_itself_client_is_refused, a_second_client_layer_is_refused_and_both_are_named, every_client_layer_is_named_when_more_than_two_declare, a_first_layer_inside_the_repository_is_refused_and_leaves_no_winner, a_client_layer_inside_the_repository_is_refused, a_kind_with_control_characters_prints_without_them, the_client_layer_overrides..., the_first_layers_identity_applies..., no_declaration_is_no_winner_and_no_error; the 34 schema cases and the section 13.28 fetch cases drive alz lint and pass now (crates/alz/tests/lint.rs, stack_lint.rs, alz-core lint and layer tests); the section 9.4 cases need R6.
Test 29 (account layer, 54 cases): merge::declarers::tests the_first_layers_account_applies..., the_client_layers_whole_account_replaces..., a_confidential_client_layer_may_declare, no_declaration_is_no_winner_and_no_error, a_group_layer_may_not_declare_an_account, two_client_layers_conflict_over_the_account..., the_two_fields_follow_the_rule_independently; the schema cases drive alz lint --layer and pass now (46); the others drive build or init (R5/R6).
Not a case of the listed files, but R4's: test 24 (fixture 29-account-layer) holds the settings, MCP and passthrough protected-variable cases: item::protected_variable::tests, item::settings::tests, item::mcp::tests, item::passthrough::tests and merge::mcp::tests (see TASK-55.5.10 to .12). Test 37 (new, R4.15) holds the settings and MCP top-level key rows: item::settings::tests a_top_level_key_that_is_not_a_known_target_is_refused_naming_the_file_whatever_the_key and item::mcp::tests (server-id grammar) give the verdict; the lint rows pass against the Rust binary now, the build rows by R5.

Owner answers to the R4 security review (2026-11-23): (1) Codex keys that duplicate first-class items are reserved at any depth (features.hooks and features.codex_hooks anywhere; profile; approval_policy, sandbox_mode and features.hooks under profiles.*) — fixed in R4; (2) an agent overlay may only make an agent stricter: its permission_mode may not be looser than the base agent's or the permissions floor, and claude.disallowedTools only grows down the stack (like deny), spec change and reference drift; (3) a named list of security-relevant native settings gets floor semantics (only stricter down the stack), and plugin and marketplace keys (enabledPlugins, extraKnownMarketplaces) are reserved — the architect drafts the list for the owner's review; (4) a warning when a layer inside the repository replaces an MCP server defined outside it. Lead decisions: the secret scan runs over the whole layer.yaml (forbid_patterns lines only skipped for the forbid match); a path scope starting with ~ or / (or holding \ or a drive prefix) is covered only by an equal scope; UTF-16 files (BOM) are decoded and NULs stripped before the secret scan; for R5, passthrough collisions and protected paths compare with ASCII case folding.

Architect (2026-11-23): DRAFT for the owner's review (security review answer 3). These are the native settings that get floor semantics, meaning they may only become stricter down the stack, and the keys that are reserved, meaning settings.yaml refuses them. Nothing in the spec or code has changed yet.

Sources checked on 2026-11-23:
- Claude Code 2.1.323: the settings reference page (code.claude.com/docs/en/settings-reference.md) and the settings page.
- Codex 0.169.0: codex-rs/core/config.schema.json and codex-rs/config/src/config_toml.rs at tag rust-v0.169.0. This was read from source, not run.
- Status column: "TF §n" means the fact is recorded in docs/target-facts.md. UNVERIFIED means the key is not recorded there yet; it was seen only in the docs or schema named above. Adapters may rely only on TF rows, so every UNVERIFIED row needs a target-facts entry (DOCS at least) before R5 depends on it.

How the floor works:
- A key with a floor, once set by a layer, can only move up its order in more specific layers. A looser value is a `settings` error that names the layer, the file and the key path, like permission-floor.
- Lists marked "no additions" may not gain entries in a more specific layer once a more general layer has set them. Removal has no syntax in settings.yaml.
- Lists marked "union" may only grow. A later kind change (a list replaced by a scalar or by null) is refused for every floor key, which departs from §7.9's left fold for these keys only.

### Claude (under `claude:` in settings.yaml)

| Key path | Proposal | Stricter order | Status |
| --- | --- | --- | --- |
| sandbox.enabled | floor | true > false | UNVERIFIED (settings-reference) |
| sandbox.failIfUnavailable | floor | true > false | UNVERIFIED |
| sandbox.autoAllowBashIfSandboxed | floor | false > true (true skips the prompts) | UNVERIFIED |
| sandbox.allowUnsandboxedCommands | floor | false > true | UNVERIFIED |
| sandbox.excludedCommands | floor | list, no additions | UNVERIFIED |
| sandbox.filesystem.allowWrite, sandbox.filesystem.allowRead | floor | list, no additions | UNVERIFIED |
| sandbox.filesystem.denyWrite, sandbox.filesystem.denyRead | floor | list, union | TF §12 item 21 (DOCS: they cover shell commands and their children) |
| sandbox.ignoreViolations | floor | no additions (hides reports only) | UNVERIFIED |
| sandbox.enableWeakerNestedSandbox, sandbox.enableWeakerNetworkIsolation | floor | false > true | UNVERIFIED |
| sandbox.network.allowedDomains, sandbox.network.allowUnixSockets, sandbox.network.allowMachLookup | floor | list, no additions | UNVERIFIED |
| sandbox.network.allowAllUnixSockets, sandbox.network.allowLocalBinding | floor | false > true | UNVERIFIED |
| sandbox.network.deniedDomains | floor | list, union | UNVERIFIED |
| sandbox.network.httpProxyPort, sandbox.network.socksProxyPort | reserve | (routing; no order) | UNVERIFIED |
| sandbox.filesystem.disabled, sandbox.allowAppleEvents, sandbox.allowNestedUserNamespaces, sandbox.network.strictAllowlist, sandbox.network.allowManagedDomainsOnly | reserve | (user or managed scope only; Claude ignores them in a project) | UNVERIFIED |
| sandbox.credentials.* | reserve | (carries or pins a login) | TF §12 item 16 |
| apiKeyHelper | reserve | (runs a command; carries a login) | TF §12 item 16 |
| permissions.* (defaultMode, disableBypassPermissionsMode, additionalDirectories, blockReadsOutsideWorkingDirectories, blockWritesOutsideWorkingDirectories, allow/ask/deny) | already reserved: `claude.permissions` is refused as a whole (§7.9) | see the question below | TF §7b, §7c (allow/ask/deny, additionalDirectories); the rest UNVERIFIED |
| disableAutoMode | floor | "disable" > unset | UNVERIFIED |
| autoMode, skipAutoPermissionPrompt, skipDangerousModePermissionPrompt, permissions.denyInsteadOfPrompting | reserve | (user, local or managed scope only) | UNVERIFIED |
| disableAllHooks, hooks | already reserved | n/a | TF §4 (disableAllHooks) |
| allowManagedHooksOnly | reserve | (managed only) | UNVERIFIED |
| allowedHttpHookUrls, httpHookAllowedEnvVars | floor | list, no additions | UNVERIFIED |
| statusLine, fileSuggestion | reserve (proposed) | (each runs a command outside hooks.yaml and its locks) | UNVERIFIED |
| disableSkillShellExecution | floor | true > false | UNVERIFIED |
| enableAllProjectMcpServers, enabledMcpjsonServers, disabledMcpjsonServers, mcpServers | already reserved | n/a | UNVERIFIED (the settings-reference scope is "any file") |
| allowedMcpServers | floor | list, no additions | UNVERIFIED |
| deniedMcpServers | floor | list, union | UNVERIFIED |
| allowManagedMcpServersOnly, managedMcpServers | reserve | (managed only) | UNVERIFIED |
| disableClaudeAiConnectors | floor | true > false | UNVERIFIED |
| allowAllClaudeAiMcps | reserve | (widens connector MCPs) | UNVERIFIED |
| enabledPlugins, extraKnownMarketplaces | reserve (owner) | (bring hooks and MCP servers past the allowlist) | TF §12 item 14 records only that plugins work as links; the semantics are UNVERIFIED |
| prependPlugins, appendPlugins, pluginConfigs, strictKnownMarketplaces, blockedMarketplaces, strictPluginOnlyCustomization, syncClaudeAiPlugins | reserve | (user or managed scope, or plugin sources) | UNVERIFIED |
| env | existing protected-variable check (§7.9) | n/a | TF §12 |

### Codex (under `codex:` in settings.yaml)

| Key path | Proposal | Stricter order | Status |
| --- | --- | --- | --- |
| approval_policy, sandbox_mode (top level and profiles.*) | already reserved at any depth (owner answer 1) | n/a | TF §6.6 (DOCS) |
| features.hooks, features.codex_hooks (anywhere), hooks, profile | already reserved (owner answer 1) | n/a | TF §6.4 |
| mcp_servers (incl. per-server default_tools_approval_mode, enabled_tools, disabled_tools) | already reserved | n/a | TF §6.5 |
| sandbox_workspace_write.network_access | floor | false > true | TF §6.6 names sandbox_workspace_write.* (DOCS); the field is UNVERIFIED (0.169.0 schema) |
| sandbox_workspace_write.writable_roots | floor | list, no additions | UNVERIFIED (schema) |
| sandbox_workspace_write.exclude_tmpdir_env_var, sandbox_workspace_write.exclude_slash_tmp | floor | true > false (fewer writable folders) | UNVERIFIED (schema) |
| approvals_reviewer (top and profiles.*) | floor | user > auto_review, guardian_subagent | UNVERIFIED (schema) |
| auto_review | reserve | (no defined order) | UNVERIFIED |
| default_permissions, permissions.* (named permission profiles: filesystem, network, workspace_roots) | reserve | (no total order between profiles) | TF §6.6 (exists; detail UNVERIFIED) |
| allow_login_shell | floor | false > true | UNVERIFIED (schema) |
| web_search (top and profiles.*), tools.web_search | floor | disabled > cached > indexed > live | UNVERIFIED (schema enum) |
| projects.*.trust_level | reserve | (a layer must not mark a project trusted) | UNVERIFIED (TF §0.1 mentions trust only as a test setup) |
| plugins, marketplaces | reserve (the Codex counterparts of enabledPlugins and extraKnownMarketplaces) | n/a | UNVERIFIED (schema) |
| apps (connector tool approval: auto, prompt, writes, approve) | reserve | (no order across connectors) | UNVERIFIED (schema) |
| windows.sandbox | reserve | (elevated, unelevated, mxc: no order) | UNVERIFIED (schema) |
| notify, model_provider, model_providers, openai_base_url, chatgpt_base_url, otel | reserve | (run a program or route traffic; several cannot be set at project level) | TF §6.6 (notify, model_provider and otel cannot be set at project level, DOCS); the rest UNVERIFIED |
| shell_environment_policy | existing protected-variable check (§7.9) | n/a | TF §12 item 15 |

Questions for the owner, each with a recommendation:
1. The `claude.permissions` block is reserved as a whole. So disableBypassPermissionsMode, blockReadsOutsideWorkingDirectories and blockWritesOutsideWorkingDirectories cannot be set at all today. Recommended: make them first-class in permissions.yaml as floor fields (disable_bypass_mode, block_reads_outside, block_writes_outside; true sticks). additionalDirectories is already the manifest's workspace.add_dirs. Alternative: leave them unreachable.
2. Reserving enabledPlugins contradicts decision-5's title ("community skills as Claude Code plugins declared in a layer's settings.yaml enabledPlugins"). No layer or fixture here uses it. Recommended: a superseding decision with a first-class plugin item behind an allowlist in the manifest, like mcp.enable. Alternative: reserve now and decide plugins later.
3. An agent's native `claude.mcpServers` (and per-agent `hooks`) can bring MCP servers and hooks past the mcp.enable allowlist and hooks.yaml. Recommended: refuse them in native fields too, or route them through the same allowlists.
4. How a floor list refuses changes: "no additions" lists cannot be narrowed either, because settings.yaml has no removal syntax. Recommended: accept that for now; a `-[...]` form for settings can come later.
5. Every UNVERIFIED row needs a target-facts entry first. Recommended: one docs task that records them at DOCS level from the two sources above, before R5's adapter relies on any of them.

Owner answers on the security-key list (2026-11-23): (1) the architect's draft list is approved as written: floors (only stricter down the stack; a looser value is a settings error naming the layer, file and key path; 'no additions' and 'union' lists; a kind change refused for floor keys) and reserved keys; every unverified key is recorded in docs/target-facts.md through a docs task before R5's adapter relies on it; (2) disableBypassPermissionsMode and the block-reads/writes-outside-working-directories keys become first-class floor fields of permissions.yaml (once true, stays true); (3) enabledPlugins and extraKnownMarketplaces are reserved now; a decision superseding decision-5 and a later task plan a first-class plugin item behind a manifest allowlist; (4) claude.mcpServers and per-agent hooks in an agent's native fields are refused (servers and hooks come only through mcp.yaml and hooks.yaml). Lead: 'no additions' lists cannot be narrowed either (no removal syntax), accepted for now.

Implementer (2026-11-23), R4 security review fixes: (1) item/settings.rs: Codex keys that duplicate first-class items are reserved at any depth (features.hooks and features.codex_hooks wherever a features mapping stands, profile, approval_policy and sandbox_mode under profiles.*); error [settings] with a fixed key path, no value and no profile name printed; neutral rows in test 37 (lint). (2) merge/agents.rs, merge/permissions.rs (DeclaredFloor), merge/stack.rs: an agent overlay may not set a permission_mode looser than the agent it refines or the permissions floor declared by more general layers, and may not remove entries of claude.disallowedTools a more general layer set; error [agent-overlay], overlay not applied; tests in merge/agents/overlay_strictness.rs incl. the property an_overlay_never_loosens_the_permission_mode. (3) merge/mcp.rs: warning [mcp] naming the server and both layers when a layer inside the repository replaces a server of a layer outside it. (4) settings.rs read_target: the unreachable non-mapping branch now returns a settings diagnostic instead of an empty error. Spec text for the architect: 7.9 'For Codex the keys that duplicate a first-class item are reserved at any depth: features.hooks and features.codex_hooks wherever a features mapping stands, profile, and approval_policy and sandbox_mode in a profile (profiles.*); a file that sets one is a settings error naming the file and the key path, with no value printed.' 7.2 'An overlay may only make an agent stricter: its permission_mode may not be looser than that of the agent it refines nor than the permissions floor (7.6) declared by a more general layer, and claude.disallowedTools keeps every entry a more general layer set; otherwise it is an agent-overlay error and the overlay is not applied.' 7.7 'A layer inside the repository that replaces a server defined by a layer outside it gets an mcp warning naming the server and both layers.' Reference drift: items 2 and 3 are Rust-only behaviour with no acceptance case yet.
<!-- SECTION:NOTES:END -->

## Comments

<!-- COMMENTS:BEGIN -->
author: @architect
created: 2026-11-23T12:29:00Z

---

Owner questions 1 to 6 for R4 are in the notes (2026-11-23), each with options and a recommendation. Blocked on them: R4.1 (TASK-55.5.1), and through it R4.6, R4.10, R4.14 and R4.15. R4.2 to R4.5 can start now.

---
<!-- COMMENTS:END -->
