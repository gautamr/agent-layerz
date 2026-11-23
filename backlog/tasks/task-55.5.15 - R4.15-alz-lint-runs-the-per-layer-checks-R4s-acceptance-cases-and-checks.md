---
id: TASK-55.5.15
title: 'R4.15: alz lint runs the per-layer checks; R4''s acceptance cases and checks'
status: In Progress
assignee:
  - '@implementer'
created_date: '2026-11-23T12:28:00Z'
updated_date: '2026-11-23T15:27:00Z'
labels:
  - area:rust
  - area:lint
milestone: m-7
dependencies:
  - TASK-55.5.14
parent_task_id: TASK-55.5
priority: high
type: task
ordinal: 128390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Plan for this step.
- alz lint, with --layer and in stack mode, runs every kind's per-layer reader on each layer alone and drops `unresolved`, as spec §10 and the reference do (owner answer 3 in TASK-55.5: no cross-layer merge in lint, no spec change; merge errors show in build, R5), plus the provides_slots and requires_slots checks.
- Acceptance: test/acceptance/37-settings-targets.test.ts with fixture test/fixtures/37-settings-targets and its §13 entry (R4.1): "a settings or MCP file whose top-level key is not a known target is refused, whatever the key", one row per key, through alz lint --layer and through alz build (no output written). Both groups use referenceDrift("TASK-55.5", ...); the build rows pass by R5.
- Run tests 02 to 11, 17 to 19, 26, 29 and 37 against ALZ_BIN and write into TASK-55.5's notes each merge case with the Rust test that gives its verdict (TASK-55.5 AC #1).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 alz lint reports the per-layer errors of every item kind and no unresolved one, for one layer and for each layer of a stack alone (assert_cmd tests)
- [ ] #2 Test 37's lint rows pass against the Rust binary and are marked reference drift against the TypeScript reference (mise run test:acceptance, with and without ALZ_BIN)
- [ ] #3 Every case of tests 02 to 11, 17 to 19, 26, 29 and 37 that drives alz lint passes against ALZ_BIN, the 84 of the 2026-11-23 baseline included, and TASK-55.5's notes list each merge case with its Rust test
- [ ] #4 mise run rust:ci, mise run lint and mise run alz:check pass, and src/ is unchanged (git diff --stat main -- src)
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Spec items collected by the lead for a spec pass before the R4 review (2026-11-23), from R4.4: (a) instructions mode values: absent or append adds, replace, remove; anything else is an instructions error; (b) replace or remove with no base is an unresolved error (add to §10.1's unresolved list), dropped by single-layer lint; (c) a replacing rule without paths is a rule error 'paths is required; always-loaded text belongs in instructions/'; (d) bodies and CLAUDE.extra.md text are trimmed of Unicode White_Space and a block empty after trimming is dropped (the reference's JS trim also removes U+FEFF: drift); (e) the layer heading capitalises the first character of the layer's kind and is printed as untrusted text; (f) duplicate globs within one rule's paths are kept once (the reference kept them on replace: drift).

Spec items from R4.5 (2026-11-23, lead accepts the choices): (a) the keys that have a neutral field inside a target map: claude name, description, tools, model, permissionMode, skills, isolation, memory; codex name, description, model, developer_instructions; (b) memory accepts none, project, user and local (the reference's set); (c) permission_mode values bypass_permissions, auto, accept_edits, default, dont_ask, plan; (d) §10.1's 'too many preloaded skills' has no defined limit: drop it from the agent code or define one; (e) overlay schema errors use agent-overlay for extends and mode and agent for the rest (the reference uses agent-overlay for all: drift), a missing extends is agent-overlay 'extends is required'; (f) removing a tool from an agent with no tool list is an agent-overlay error; (g) the list-edit syntax (+[..], -[..], [..], comma split, one pair of quotes stripped, no ] inside an item, empty means no change); (h) name equals the file stem (§4.1); (i) native fields keep source order (TASK-66, lead): first appearance keeps its place, later-layer keys appended, whole-number keys in source order (the reference hoists them: drift); settings stay sorted.

Spec items from R4.6 (2026-11-23): (a) a base path scope whose segments other than the last end in ** covers only an equal scope (keeps covers transitive); (b) the neutral tool vocabulary (the reference's 12 names) plus any name starting mcp__; anything else is an agent error, also inside -[..]; an empty scope (write:) is accepted as written; (c) 'the base' is the agent the overlay refines; a later overlay cannot re-widen what an earlier one narrowed; (d) a refused overlay is not applied; (e) allow_widen in a public layer is an error even without a tools edit or a base; (f) docs/** covers docs itself, ** covers any path without a dot segment, braces are literal.

Spec items from R4.7 (2026-11-23): (a) drop 'too many preloaded skills' from §10.1's agent code (lead: no limit in the spec or the reference); (b) command frontmatter: description required, argument_hint optional, any other field a command error; (c) provides_slots is checked by lint only (R4.15 calls SkillItems::check_provided); a requires_slots entry that is not a skill name is a slot error in build and lint; (d) slot-missing is located at <layer>:layer.yaml for requires_slots and at the agent's defining file for a preload; (e) a command and a skill with one name in one layer: the command is dropped with one error; (f) SKILL.md frontmatter other than name and description is not checked.

Spec items from R4.8 (2026-11-23): (a) hook ids: any text with a visible character, printed as a quoted key; (b) timeout: a positive whole number of seconds (the reference accepted fractions: drift; no fixture or layer uses one); (c) on_fail defaults to warn; (d) native events: target claude or codex, name ^[A-Z][A-Za-z]+$, a native hook lists only its own target, targets: [] is an error; (e) match.tools: bare neutral tools or mcp__ tools, no scope; (f) a disabled id stays taken (redefining it later is a hooks error), disabling a hook of the same layer or one already disabled is unresolved, a duplicate id in one layer is a hooks error; (g) merge diagnostics carry no line and column; reader errors do.

Spec items from R4.9 (2026-11-23): (a) allow, ask and deny are written as a plain YAML list (adds) or +[..], -[..], [..] edits; allow and ask may drop rules, deny may not; note that a plain list adds here but replaces in §7.2 overlays; (b) the permission modes and their strictness order: bypass_permissions < auto < accept_edits < default < dont_ask < plan; (c) a rule is tool[:pattern] in the neutral tool grammar; an unknown tool or an empty pattern is a permissions error; (d) add the warning code permission-shadowed to §10.1 (a rule in both allow and deny; lead keeps the reference's warning); (e) floor must be a boolean and needs default_mode (floor: false without default_mode is an error too); (f) a default_mode looser than the floor is refused and that layer's mode and floor are ignored; a layer's own floor does not bind its own mode; (g) permission-floor carries no line, permissions file errors do.

Spec items from R4.10 (2026-11-23): (a) a target whose value is null or not a mapping is a settings error; (b) lead: shell_environment_policy, mcpServers and mcp_servers that are not mappings are refused (they can carry environment settings the protected-variable checks must see); (c) protected-variable diagnostics name the file without line and column; (d) a pattern matching several protected variables gives one error per variable; (e) protected names match in any ASCII case and the written text is never printed. Also: the settings and protected-variable cases live in test 24 (fixture 29-account-layer), not in 11, 26 or 29 (mapping in TASK-55.5.10's notes).

Spec items from R4.11 (2026-11-23): (a) per-server checks stop at the first failing step; (b) a command on an http/sse server or a url on a stdio server is ignored; (c) mcp.enable repeats are kept once; (d) a server refused for any reason still gets the 'no layer defines it' error for mcp.enable (the spec names only the protected case); (e) alias, role and concrete model ids are any non-blank text; (f) a models.yaml with any problem gives no item (the reference kept the valid aliases: drift); (g) the credential-assignment secret shape matches JavaScript's whitespace set explicitly; (h) forbid_patterns compile with the same limits as the secret scan (secret::SIZE_LIMIT, NEST_LIMIT).

Spec items from R4.12 (2026-11-23): (a) the passthrough warning 'raw passthrough to <path>; consider turning it into a first-class item' and the unknown-folder error text; (b) §7.9's non-ASCII path rule covers any folder of targets/, not only passthrough paths; (c) a .toml that is not valid UTF-8 is refused as unchecked; (d) the TOML text rule reports each distinct protected variable mentioned; (e) a non-mapping, non-list mcpServers (a plugin manifest naming a file) and a JSON root that is not an object are not checked; (f) nested env depth is bounded by the JSON reader's 128 (the reference refused past 32); (g) a refused passthrough file gives errors only (the reference also warned).

Spec items from R4.13 (2026-11-23): (a) add identity and account rows to §10.1's merge table; (b) say which rule each consumer uses: the build reports every refused layer as an error and a refused first layer gives Conflict; account resolution (§7.13) ignores an in-repository declaration (neither wins nor conflicts); (c) a refused first layer with a declaring client gives Conflict, no winner; (d) a layer that cannot be opened counts as inside the repository; (e) the refusal messages keep the reference's wording.

Spec items from R4.14 (2026-11-23): (a) one leak diagnostic per file and entry, per file and secret kind, and one email and one host warning per file, without line numbers (the reference reported per line); (b) patterns match the whole file text ((?m) for ^ and $); (c) binary and non-UTF-8 files are scanned too, read lossily (the reference skipped files with a NUL in the first 8 KiB); (d) layer.yaml is scanned without the forbid_patterns span, removed line-wise with comment-only lines kept; (e) one refusal message for a pattern that does not compile, is too large or nests too deep; (f) only entry 65 is reported past the cap; (g) a refused entry is located at <layer>:layer.yaml; (h) the email and host heuristics are the reference's patterns.
<!-- SECTION:NOTES:END -->
