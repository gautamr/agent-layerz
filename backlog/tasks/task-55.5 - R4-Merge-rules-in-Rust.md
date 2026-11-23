---
id: TASK-55.5
title: 'R4: Merge rules in Rust'
status: In Progress
assignee:
  - '@architect'
created_date: '2026-11-22T12:54:00Z'
updated_date: '2026-11-23T12:43:00Z'
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
<!-- SECTION:NOTES:END -->

## Comments

<!-- COMMENTS:BEGIN -->
author: @architect
created: 2026-11-23T12:29:00Z

---

Owner questions 1 to 6 for R4 are in the notes (2026-11-23), each with options and a recommendation. Blocked on them: R4.1 (TASK-55.5.1), and through it R4.6, R4.10, R4.14 and R4.15. R4.2 to R4.5 can start now.

---
<!-- COMMENTS:END -->
