---
id: TASK-55.5.11
title: 'R4.11: MCP servers, the allowlist, models and the secret scan'
status: Done
assignee:
  - '@implementer'
created_date: '2026-11-23T12:27:00Z'
updated_date: '2026-11-23T14:52:00Z'
labels:
  - area:rust
  - area:merge
  - area:security
milestone: m-7
dependencies:
  - TASK-55.5.10
parent_task_id: TASK-55.5
priority: high
type: task
ordinal: 124390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Plan for this step (§7.7, §7.8, §9.2 secret scan).
- McpServerId (owner answer 1 in TASK-55.5): `^[A-Za-z0-9][A-Za-z0-9_-]{0,63}$`, one type for mcp.yaml keys and the manifest's mcp.enable. The manifest reads mcp.enable entries as McpServerId instead of the free-text McpName, and an entry that breaks the grammar is a `manifest` error naming its key path, never its text.
- alz-core `secret.rs`: the built-in secret scan. Its fixed token shapes are compiled with regex-lite =0.1.9 (decision-38), once, and a test proves each compiles. This step adds the dependency to the workspace Cargo.toml and alz-core, and to deny.toml with `wrappers = ["alz-core"]` and the version pin. A finding names the file and the kind of token, never the value.
- item/mcp.rs: mcp.yaml parses into an ordered map of McpServerId to McpServer. A key that is not a server id is an `mcp` error naming the file, whatever the key, printed as a quoted key. Fields are closed (transport, url, command, args, env_from); env and headers are refused; Transport = Stdio { command, args, env_from } | Http { url } | Sse { url }; env_from entries are plain names and a protected one is a `protected-variable` error, after which mcp.enable adds no "no layer defines it" error for that id; a value that looks like a token is `leak-secret`.
- merge: Replace per id, with provenance marking overridden definitions; enabled = the ids of mcp.enable in order, and an id no layer defines is an error.
- item/model.rs: aliases (alias to an ordered map of TargetName to model id), roles, default; DeepMerge; a role or default naming an unknown alias is `unresolved`.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Test 09's verdict has a Rust test: a server defined but not enabled is not in the merged enabled list (unit test)
- [x] #2 An mcp.yaml key outside the server-id grammar is refused naming the file, whatever the key, and so is an mcp.enable entry, as a manifest error naming its key path (table tests); env, headers, a token-like value and a protected env_from are refused, the last without a second mcp.enable error (unit tests)
- [x] #3 Model aliases keyed by anything but a target are refused and roles resolve through aliases (unit tests)
- [x] #4 The secret scan compiles every token shape with regex-lite =0.1.9 per decision-38, finds each shape and puts no value in a diagnostic (unit tests), and cargo deny passes with the new wrappers rule (mise run rust:ci)
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Implementer (R4.11), evidence. mise run rust:ci exit 0 (fmt, clippy -D warnings, 831 alz-core tests, doc, MSRV, cargo deny advisories/bans/licenses/sources ok, audit); mise run lint ok.
Dependency: regex-lite =0.1.9, default-features off, features std and string (workspace Cargo.toml, alz-core only); deny.toml has { name = regex-lite, wrappers = [alz-core] } and the four version-pin lines. Patterns compile with size_limit 64 KiB and nest_limit 32 (secret::SIZE_LIMIT, NEST_LIMIT).
AC1: merge::mcp::tests::a_server_defined_but_not_enabled_is_not_in_the_enabled_list (reads test/fixtures/09-mcp-allowlist/personal files; enabled = [context7], playwright defined and inactive). Also enabled order and de-duplication, replace per id with provenance (Define then Replace), proptest enabled_is_the_manifest_list_without_repeats_and_the_last_definition_wins.
AC2: item::mcp::tests::a_key_that_is_not_a_server_id_is_refused_naming_the_file_whatever_the_key (table; error [mcp] <layer>:mcp.yaml:<line>:<col>, key printed quoted, other servers still read), a_scalar_key_that_is_not_text_is_refused_by_its_kind, item::mcp::server_id tests (grammar examples, refusals, proptest against the regex by hand), stack::tests::an_mcp_enable_entry_outside_the_server_id_grammar_is_a_manifest_error_naming_its_key_path (manifest error naming mcp.enable[1], no text), an_mcp_enable_entry_that_is_not_text...; env and headers refused (env_and_headers_are_refused_because_no_value_is_stored); token-like value refused as leak-secret naming the kind, never the value (a_value_that_looks_like_a_token_is_refused_naming_the_kind_and_never_the_value); protected env_from refused, and merge::mcp::tests::a_server_refused_for_a_protected_variable_adds_no_second_error_for_the_allowlist.
AC3: item::model::tests::an_alias_key_that_is_not_a_target_is_refused_naming_the_file_whatever_the_key, a_key_that_is_not_text..., merge::models::tests::roles_resolve_through_aliases_to_the_model_of_each_target, deep merge and provenance, a_role_or_default_that_names_an_unknown_alias_is_unresolved.
AC4: secret::tests::every_token_shape_compiles, each_shape_is_found_by_its_own_kind, a_github_token_is_found_in_each_of_its_prefixes, a_finding_holds_the_kind_and_never_the_text (samples are assembled at run time); cargo deny passes (rust:ci).
Case-to-test mapping. Test 09 (the one case): merge::mcp::tests::a_server_defined_but_not_enabled_is_not_in_the_enabled_list. Test 10 'fake token' (public-layer leak guard, applies the scan to every file): the shape part is secret::tests::each_shape_is_found_by_its_own_kind and a_finding_holds_the_kind_and_never_the_text; the public-layer scan and the lint run are R4.14/R4.15. Test 24 mcp rows: 'mcp env_from' protected-variable table row and 'a variable name that is not a plain name' -> item::mcp::tests::a_protected_variable_in_env_from_is_refused_and_the_server_is_remembered and an_env_from_entry_that_is_not_a_plain_name_is_refused_as_a_variable_problem; 'an mcp env_from of another variable' still builds -> an_env_from_of_other_variables_is_accepted; 'an MCP server refused for a protected variable does not also report mcp.enable' -> merge::mcp::tests::a_server_refused_for_a_protected_variable_adds_no_second_error_for_the_allowlist. These cases drive alz build and pass in R5; not changed.
Choices and spec gaps (spec not edited): (1) per-server checks stop at the first failing step as the reference does (shape, env/headers, unknown field, secret, field types, env_from); the spec does not say whether one server reports several problems. (2) a command on an http or sse server, or url on a stdio server, is ignored as in the reference; the spec is silent. (3) mcp.enable repeats are de-duplicated keeping the first; the spec is silent. (4) a server refused for any reason but a protected env_from still gets the 'no layer defines it' error, as the reference does; the spec only names the protected case. (5) model alias names, role names and model ids are any non-blank text (spec gives no grammar). (6) spec 7.8 lists a warning for an alias without a model for a target and an error for an agent model that is not an alias; both belong to resolving an agent (R5), only MergedModels::model/for_role/for_default are provided here. (7) the secret scan's credential-assignment shape writes JavaScript's whitespace set explicitly because regex-lite's \s is ASCII only; \b and \w are ASCII as in the reference. (8) leak-secret, mcp and models are new Code variants (spec 10.1 already lists them). (9) models.yaml with any problem gives no item, like settings; the reference kept the valid aliases of a bad file.

Verified by the lead (2026-11-23): mise run rust:ci exits 0 (1070 tests; deny ok with regex-lite), lint passes, src/ and test/ unchanged. Choices accepted and collected for the spec.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
MCP servers merge as an ordered map keyed by McpServerId (one grammar for mcp.yaml and mcp.enable; any other key is an error naming the file), with stdio, http and sse transports, protected env_from refused, and the allowlist giving the enabled list in order; models map aliases to the known targets only and resolve roles; the secret scan finds the nine token shapes with regex-lite (decision-38) and reports a kind, never the text. Test 09 and the mcp cases of 24 have Rust tests with the same verdicts.
<!-- SECTION:FINAL_SUMMARY:END -->
