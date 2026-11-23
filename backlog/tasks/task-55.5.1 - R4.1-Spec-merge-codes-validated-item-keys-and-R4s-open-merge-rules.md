---
id: TASK-55.5.1
title: 'R4.1: Spec: merge codes, validated item keys and R4''s open merge rules'
status: Done
assignee:
  - '@architect'
created_date: '2026-11-23T12:27:00Z'
updated_date: '2026-11-23T17:46:00Z'
labels:
  - area:rust
  - area:spec
milestone: m-7
dependencies: []
parent_task_id: TASK-55.5
priority: high
type: task
ordinal: 114390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Plan for this step: spec text only (docs/LAYERING-SPEC.md, CHANGELOG.md), on the R4 branch, so it ships in the R4 pull request with the code. It waits for the owner questions in TASK-55.5's notes.
- §7.9: the top level of settings.yaml takes only the target names claude and codex. Any other top-level key is a `settings` error that names the layer and the file, whatever the key, and that file's settings are not merged. The merged settings are ordered maps of validated keys.
- §7.7: the keys of mcp.yaml are server ids with the grammar the owner picks (question 1); any other key is an `mcp` error naming the file, printed as a quoted key. mcp.enable uses the same grammar.
- §7.8: each alias of models.yaml maps target names only.
- §10.1: every diagnostic code the merge emits, with its level, and how item names, refused keys and refused values print (decision-26 classes; protected and secret values never).
- The answers to questions 2 to 6 (leak guard phase, lint scope, test 26's phase, settings deep merge, tool scope containment), or a note that the reference behaviour stays.
- §13: a new entry for "a settings or MCP file whose top-level key is not a known target is refused, whatever the key".
- CHANGELOG: the refusals are user-facing (a layer that built before may fail now); minor version bump at cutover (doc-5).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 §7.7, §7.8 and §7.9 state which top-level keys settings.yaml, mcp.yaml and the aliases of models.yaml accept, and that any other key is an error naming the file (spec diff)
- [x] #2 §10.1 lists every diagnostic code the merge emits with its level, and says how item names, refused keys and refused values print (spec diff)
- [x] #3 Each answered owner question of TASK-55.5 has spec text or a note that the reference behaviour stays; §13 has the known-target entry; CHANGELOG lists the user-facing changes with the version note (spec and CHANGELOG diff)
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Architect (2026-11-23): the spec text is written on feat/rust-r4-task-55.5 and not committed, so it lands in the R4 pull request with the code. Files: docs/LAYERING-SPEC.md (§5.2, §7.2, §7.7, §7.8, §7.9, §9.2, §10.1, §13 item 29) and CHANGELOG.md (Unreleased, Changed).
- AC #1:
  - §7.9: the top level of settings.yaml takes only claude and codex. Any other key, whatever it is, is a settings error naming the layer and file, with the key quoted, and none of the file's settings are merged. The merged settings are ordered maps of the checked keys.
  - §7.7: mcp.yaml keys are server ids matching ^[A-Za-z0-9][A-Za-z0-9_-]{0,63}$. Any other key is an mcp error naming the file. §5.2 applies the same grammar to mcp.enable.
  - §7.8: an alias key that is not a target is a models error naming the file.
- AC #2: §10.1 has a "Merge diagnostics" table with 27 codes and their levels, taken from the reference's merge and leak guard plus R4's new refusals. It also says how names, refused keys and refused values print.
- AC #3:
  - Answer 1 is in §7.7 and §5.2.
  - Answer 2 is in §9.2: the regex-lite syntax (decision-38), refusal of look-around and backreferences with a message naming the feature, the limits (64 entries, 512 characters, 64 KiB compiled, nesting 32), and the secret scan with its codes.
  - Answer 3 is the §10.1 sentence "alz lint checks each layer alone and leaves out unresolved". There is no other lint change.
  - Answer 4 changes a phase, not behaviour, so the spec does not change.
  - Answer 5 is in §7.9: a left fold, a later kind replaces, deduplication by value.
  - Answer 6 is in §7.2: the containment rule, with the reference's prefix rule described in the open as reference drift (TASK-55.5.6).
  - §13 has the new item 29, and the CHANGELOG lists the user-facing refusals with the minor-bump note.
Checks: mise run lint passed (biome, tsc). No code changed. I searched the changed files for the private item's words and found none.

Correction: the §10.1 table lists 28 codes (leak-email and leak-host share one row), not 27.

Architect (2026-11-23), second pass after R4.2 (b42dc6c), still on the branch and not committed:
- §4.1 "Item paths": the item paths R4.2 classifies, layer.yaml and hooks/scripts/<path> included, and the rule for a targets/ folder that is not a known target (an error naming each file; a file directly in targets/ is no item).
- §4.1 "Markdown frontmatter": the opening line is exactly --- plus LF, with CRLF read as LF; the block closes at the first line that is exactly --- (reference drift); the body follows the closing line's LF; an empty, comment-only or null block is an empty mapping; a file without an opening line is body only and must be valid UTF-8 (reference drift: the reference reads it lossily); the 1 MiB cap and the YAML rules apply to a file with a block; YAML errors are located at the file's line and column.
- §7.5: run: names a script below hooks/scripts/, relative to hooks/. The reference takes any file below hooks/ (reference drift). Every fixture and .layer use scripts/.
- §7.11: only known-target folders hold passthrough files.
- §10.1: the frontmatter row quotes the two messages; the passthrough row gains the error case.
- CHANGELOG: two more bullets.
Checks: mise run lint exit 0. The added lines of the spec and CHANGELOG contain none of the private item's words.

Lead (2026-11-23): accepted, including the stricter hook run: rule (a script below hooks/scripts/, reference drift) and the unknown targets/<name> folder as a passthrough error. The R4 unit review checks the text against the code.

Architect (2026-11-23), R4 spec pass. I wrote every spec item from R4.4 to R4.14 in TASK-55.5.15's notes into docs/LAYERING-SPEC.md, on the branch and not committed. Unclear details were checked in crates/alz-core.
- §7.1: the instruction modes; replace or remove without a base is unresolved; White_Space trim and empty blocks dropped (drift: U+FEFF, TASK-55.5.4); the kind heading.
- §7.2:
  - the list-edit syntax;
  - the narrowing details: docs/**/** covers only equal; docs/** covers docs; ** covers any path without a dot segment; braces are literal; the base includes earlier overlays; a refused overlay is not applied; allow_widen in a public layer is always an error; the 12-tool vocabulary plus mcp__; an empty scope is taken as written;
  - the neutral keys of the claude and codex maps;
  - the native-field key order (TASK-66, drift);
  - the memory and permission_mode values;
  - the overlay error codes (drift, TASK-55.5.5);
  - removing a tool when the agent has no tool list.
- §7.3: SKILL.md checks; where the slot and slot-missing errors are located; lint-only provides_slots; no preload limit.
- §7.4: skill-command-clash drops the command; the command frontmatter fields.
- §7.5: hook ids, timeout (drift, TASK-55.5.8), the on_fail default, native events, targets: [], match.tools, the id rules.
- §7.6: list edits; rule grammar; permission-shadowed; the order of modes; floor; a looser default_mode ignored.
- §7.7: checks stop at the first failure; ignored fields; mcp.enable repeats; "no layer defines it" except for a protected refusal.
- §7.8: non-blank ids; a broken models.yaml gives nothing (drift, TASK-55.5.11).
- §7.9: a null target value; non-mapping policy and server maps; one error per protected variable; no line or column.
- §7.10: paths required; duplicate globs kept once (drift, TASK-55.5.4); extend without a base is unresolved.
- §7.11: both messages; a refused file gives no warning (drift); the non-ASCII rule for all of targets/; the TOML and JSON limits (depth 128, drift, TASK-55.5.12).
- §7.12: refusals, conflict, unopenable layers, reason wording, the account-resolution difference (TASK-55.5.13).
- §9.2: reporting per file without lines (drift); multi-line mode; lossy reading of binary files (drift); the layer.yaml span; locations; one message; entry 65; the heuristic patterns; JS white space (TASK-55.5.14).
- §10.1: the agent row (preload limit dropped), agent-overlay, slot-missing and hooks rows; new rows permission-shadowed, identity and account; a longer unresolved row; the line and column rule.
- §10.2: key order (TASK-66).
Checks: mise run lint exit 0. The added lines contain none of the private item's words. TASK-66 AC #1 is checked, with its evidence in TASK-66's notes.

Architect (2026-11-23), spec text for the R4 review fixes (18eba98, 63cc01d), on the branch and not committed. Each place the reference differs is marked reference drift (TASK-55.5).
- §7.9: the Codex reserved keys at the top level (model, mcp_servers, hooks, profile, approval_policy, sandbox_mode). Also at any depth: hooks and codex_hooks in every features mapping, and approval_policy and sandbox_mode in each profile. Each is a settings error naming the file and an alz-chosen key path (* for a profile name), never the value.
- §7.2:
  - A path candidate that starts with ~ or /, holds a backslash or starts with a drive letter and colon is covered only by an equal scope. The shell-operator list already matched the code, so it is unchanged.
  - New bullet: an overlay only makes an agent stricter. Its permission_mode may not be looser than the refined agent's or a more general floor, and claude.disallowedTools keeps every inherited entry. Otherwise it is an agent-overlay error and the overlay is not applied; the spec quotes the message.
- §7.7: the warning for a server inside the repository that replaces one outside it (message quoted, at the replacing mcp.yaml, the replacement still applies). An env_from protected-variable error names the file without a line and column.
- §9.2:
  - ^ and $ match at each line, ending at LF, CR or CR LF.
  - Files with a UTF-16 byte order mark (LE or BE) are decoded as UTF-16; other files are read as UTF-8 with U+FFFD for invalid bytes. The byte order mark is dropped, and NULs are removed before any check.
  - Only the forbid_patterns match skips that field's lines; the secret scan and the heuristics read the whole layer.yaml.
- §10.1: permission-shadowed is the one merge warning without <where>, in its table row and in the paragraph below the table.
- CHANGELOG: four bullets: Codex keys at any depth, overlay strictness, the path-scope rule, UTF-16 and multi-line leak checks.
Checks: mise run lint exit 0. The grep of the spec and CHANGELOG diff for the private items' words found nothing. I did not touch crates/, test/ or TASK-55.5.16.

Architect (2026-11-23), R4.16 spec text, on the branch and not committed: §7.9 has the table of security-relevant native keys (first-class, reserved and floor keys, key paths, the breach rule, the TASK-76 facts rule). §7.2 refuses servers and hooks in native fields and drops the per-agent hooks example. §7.6 has the three permissions floor fields. §10.1 rows are extended, and the CHANGELOG has three bullets. mise run lint exit 0; the leak grep found nothing.

Architect (2026-11-23), correction to the R4.16 pass: §7.9 now lists Claude disableAutoMode as ordered text with one rank, disable (once set, no more specific layer may change it; unset is free). The six Codex features.hooks and features.codex_hooks paths are now written out in full. A script checked all 98 path rows of native_keys.rs KEYS against §7.9 and found none missing. mise run lint exit 0; the leak grep found nothing. Not committed.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
The spec text and CHANGELOG entry for R4 are written, uncommitted, on the R4 branch: the known-target rule for settings.yaml, the server-id grammar for mcp.yaml and mcp.enable, target-only model aliases, the tool-scope containment rule with the reference's prefix rule as drift, the forbid_patterns syntax and limits (decision-38), the secret scan, the merge diagnostic codes in §10.1, and §13 item 29. mise run lint passes. It waits for review, then ships in the R4 pull request.
<!-- SECTION:FINAL_SUMMARY:END -->
