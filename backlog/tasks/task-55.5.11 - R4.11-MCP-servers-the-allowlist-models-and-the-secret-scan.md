---
id: TASK-55.5.11
title: 'R4.11: MCP servers, the allowlist, models and the secret scan'
status: To Do
assignee:
  - '@implementer'
created_date: '2026-11-23T12:27:00Z'
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
Plan for this step (§7.7, §7.8).
- alz-core `secret.rs`: the fixed token shapes of the built-in secret scan, as hand-written matchers (no new crate); a finding names the file and never holds the value.
- item/mcp.rs: mcp.yaml parses into an ordered map of McpServerId to McpServer. A key that is not a server id (grammar from R4.1) is an `mcp` error naming the file, whatever the key. Fields are closed (transport, url, command, args, env_from); env and headers are refused; Transport = Stdio { command, args, env_from } | Http { url } | Sse { url }; env_from entries are plain names and a protected one is a `protected-variable` error, after which mcp.enable adds no "no layer defines it" error for that id; a value that looks like a token is `leak-secret`.
- merge: Replace per id, with provenance marking overridden definitions; enabled = the ids of mcp.enable in order, and an id no layer defines is an error.
- item/model.rs: aliases (alias to an ordered map of TargetName to model id), roles, default; DeepMerge; a role or default naming an unknown alias is `unresolved`.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Test 09's verdict has a Rust test: a server defined but not enabled is not in the merged enabled list (unit test)
- [ ] #2 An mcp.yaml key that is not a server id is refused naming the file, whatever the key (the table of R4.10's test), and env, headers, a token-like value and a protected env_from are refused, the last without a second mcp.enable error (unit tests)
- [ ] #3 Model aliases keyed by anything but a target are refused, roles resolve through aliases, and the secret scan finds each token shape without putting the value in a diagnostic (unit tests)
<!-- AC:END -->
