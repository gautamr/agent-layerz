---
id: TASK-55.5.11
title: 'R4.11: MCP servers, the allowlist, models and the secret scan'
status: In Progress
assignee:
  - '@implementer'
created_date: '2026-11-23T12:27:00Z'
updated_date: '2026-11-23T14:30:00Z'
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
- [ ] #1 Test 09's verdict has a Rust test: a server defined but not enabled is not in the merged enabled list (unit test)
- [ ] #2 An mcp.yaml key outside the server-id grammar is refused naming the file, whatever the key, and so is an mcp.enable entry, as a manifest error naming its key path (table tests); env, headers, a token-like value and a protected env_from are refused, the last without a second mcp.enable error (unit tests)
- [ ] #3 Model aliases keyed by anything but a target are refused and roles resolve through aliases (unit tests)
- [ ] #4 The secret scan compiles every token shape with regex-lite =0.1.9 per decision-38, finds each shape and puts no value in a diagnostic (unit tests), and cargo deny passes with the new wrappers rule (mise run rust:ci)
<!-- AC:END -->
