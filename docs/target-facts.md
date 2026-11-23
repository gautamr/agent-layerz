# target-facts.md — verified facts about Claude Code and Codex

**Date:** 2026-11-19; §10–§11 added 2026-11-21; §12 added 2026-11-22 (item 15 policy-pattern row added the same day, TASK-50.2); §13 added 2026-11-23 (security-relevant settings keys, TASK-76); §14 added 2026-11-24 (typed Claude settings keys and subagent frontmatter, TASK-79)
**Spec checked:** `docs/LAYERING-SPEC.md` Draft v1, §12 only (items 8–9 added later the same day: §7c); §7.12/§9.4 commit identity (§10–§11 here); §7.13 account selection, §12 items 10–21 (§12 here)
**Tools:** Claude Code **2.1.311** (`/opt/homebrew/bin/claude`), Codex CLI **0.169.0** (`/opt/homebrew/bin/codex`). §10–§11: Claude Code **2.1.323**, Codex CLI **0.169.0**, git **2.50.1** (Apple Git-155). §12: Claude Code **2.1.323**, Codex CLI **0.169.0** (source tag `rust-v0.169.0`), mise **2026.9.1**
**Host:** macOS (Darwin 25.6.0), both installed through Homebrew casks

## How to read this

Status values, strongest first:

| Status | Meaning |
|---|---|
| **SMOKE** | Observed by running the installed tool in a throwaway fixture. |
| **HELP** | Read from the installed tool's `--help` or `features list` output. |
| **DOCS** | Stated by the vendor documentation (URL in §0), not reproduced locally. Treat as a claim. |
| **UNVERIFIED** | Neither tested nor documented clearly. Do not build on it. |

Each row says what the spec **requires/assumes**, what the tool **does now**, and how it was checked. Proposals are only in §8.

### 0. Sources

- Claude: `code.claude.com/docs/en/{sub-agents,skills,hooks,memory,permissions}`
- Codex: `learn.chatgpt.com/docs/{agent-configuration/agents-md,agent-configuration/subagents,config-file/config-reference,build-skills,hooks}` (the old `developers.openai.com/codex/*` URLs redirect there)

### 0.1 Test limits (read before trusting §1–§7)

- **No isolated login.** `CLAUDE_CONFIG_DIR=<empty dir> claude -p …` answers `Not logged in`. Codex has no credentials on this machine (`codex doctor`: `auth.credentials: fail`). I did not copy credentials.
- **So Claude smoke tests used the real default config**, with fixtures under the scratchpad. I wrote nothing under `~/.claude*` or `~/.codex` (Claude does write its own session transcripts under `~/.claude/projects/`; I did not touch them). Anything needing a write to a user dir is DOCS or UNVERIFIED.
- **Codex model-driven tests were impossible.** I used `codex debug prompt-input` (renders the model-visible input, no model call, no login) and `codex mcp list` / `codex doctor --json`.
- **Codex project trust could not be granted.** `-c 'projects."<path>".trust_level="trusted"'` does **not** enable the project layer: `codex exec` still prints `Untrusted project-local configuration is disabled`. The only supported grant is `codex exec --trust`, which writes to `~/.codex`, so I did not run it. Result: every Codex test of `.codex/config.toml`, `.codex/agents/`, `.codex/hooks.json` and project MCP ran **untrusted** and proves nothing about trusted behaviour.
- Claude counts of "how many times does a token appear in the context" use the model to count, with `memory_paths` from the init event as a cross-check. Treat them as strong but not formal evidence.
- Scratchpad fixtures: throwaway repositories in a temporary session folder (`repo2`, `repo3a`, `repo3b`, `repo3c`, `repo4`, `parent`, `cx`), deleted after the run.

### 0.2 Introspection tools found (useful for `alz doctor` and the acceptance tests)

| Tool | Command | Needs login? | Gives |
|---|---|---|---|
| Claude | `claude -p <prompt> --output-format stream-json --verbose`, `system`/`init` event | yes + one model call | `skills`, `agents`, `slash_commands`, `plugins`, `mcp_servers`, `memory_paths`, `tools`. **SMOKE** |
| Codex | `codex debug prompt-input [PROMPT]` | **no** | Full model-visible input as JSON: merged `AGENTS.md` text and the skills catalog with file paths. Does **not** list custom agents. **SMOKE** |
| Codex | `codex doctor --json` | no | Config/MCP/auth health checks; no per-project skill/agent listing. **SMOKE** |
| Claude | `claude agents` | — | Lists *background sessions*, **not** agent definitions. **HELP** |

---

## 1. Claude Code — subagent frontmatter (spec §12.1, §7.2, §8.1)

| Item | Spec assumes | Verified now | Status / how |
|---|---|---|---|
| Field names | neutral snake_case (`permission_mode`) | Native names are **camelCase**: `permissionMode`, `disallowedTools`, `maxTurns`, `mcpServers`, `omitClaudeMd`, … Must match exactly. | DOCS (sub-agents) |
| Required | `name`, `description` | Same. `name` ≤256 chars, cannot start with `-` or contain `:`. Files without both are skipped. | DOCS |
| Unknown fields | — | **Silently ignored.** Agent with `bogusField: 1` loaded and ran. | SMOKE (repo2 `dup-agent`) + DOCS |
| `tools` | list like `[read, grep, write:docs/**, bash:mise run *]` | Allowlist of **tool names** (`Read, Grep, Bash`), comma string or YAML list; `Agent(a,b)` limits spawnable subagents. **No path/command scoping.** `disallowedTools` is a separate denylist applied first; a scoped entry such as `Bash(git push *)` removes the **whole** tool. | DOCS |
| `skills` | list, slots allowed | YAML list of skill names, full content **preloaded**, max **32** distinct. A skill with `disable-model-invocation: true` cannot be preloaded. | DOCS |
| `model` | alias or `inherit` | `sonnet`, `opus`, `haiku`, `fable`, a full model id, or `inherit`. | DOCS |
| `memory` | `none \| project \| user` | `user \| project \| local`. **No `none`** (omit the field). | DOCS. SMOKE: `memory: none` did not stop the agent loading, but whether it was ignored or applied is unknown. |
| Isolation field | `isolation: none \| worktree` | `isolation: worktree` only; omit for none. | DOCS. SMOKE: `isolation: none` did not break loading (same caveat). |
| Permission mode | `permission_mode: default` | `permissionMode` ∈ `default, acceptEdits, auto, dontAsk, bypassPermissions, plan` (`manual` = alias of `default`). **Ignored for plugin agents.** | DOCS; CLI `--permission-mode` choices match (HELP) |
| Hooks in frontmatter | — | `hooks:` supported; **ignored for plugin agents** (also `mcpServers`, `permissionMode`). | DOCS |
| Other fields | — | `maxTurns`, `autoCompactWindow`(≥2.1.296), `mcpServers`, `background`, `omitClaudeMd`(≥2.1.271), `effort`, `color`, `initialPrompt`, `experimental.cacheTtl`. | DOCS |
| Loaded agent list | — | `init.agents` listed the project agent: `['claude','dup-agent','Explore','general-purpose','Plan','statusline-setup']`. | SMOKE |

## 2. Claude Code — same-name precedence (spec §12.2, §1, §8.4)

| Item | Spec expects | Verified now | Status / how |
|---|---|---|---|
| Agents | project > user > plugin | Documented order: managed > `--agents` JSON > project `.claude/agents/` > `~/.claude/agents/` > plugin. Nearest `.claude/agents/` wins when nested. | DOCS |
| `--agents` beats project | — | Same name in `--agents` JSON and `.claude/agents/`: **`--agents` ran** (`MARKER-AGENT-CLI`). Project-only run returned `MARKER-AGENT-PROJECT`. | SMOKE |
| Project beats user (agents) | yes | **Not tested**: needs a file in `~/.claude/agents/` (forbidden); the dir is empty on this machine. | DOCS only |
| Skills | personal > project | Enterprise > **personal > project**. Same name in `~/.claude/skills/gr-typescript-domain` and project `.claude/skills/gr-typescript-domain`: invoking it returned the **user** body, not `MARKER-PROJECT-GTD`. `init.skills` shows the name **once**, with no warning. | SMOKE |
| Plugin skills | — | Namespaced `/plugin:skill`, never collide. Shown in `init.skills` as `postman:…`, `figma:…`. | SMOKE + DOCS |
| `--add-dir` | — | Added dirs lose to the main project and get a prefix on collision. They also contribute `.claude/agents/`. | DOCS |

Conclusion: the premise of §8.4 holds for skills (**SMOKE**) and is documented for agents.

## 3. Claude Code — `CLAUDE.md`, `@import`, `AGENTS.md` (spec §12.3, §7.1, §8.1)

| Fixture (`AGENTS.md` contains `TOKEN-ALPHA` once) | Observed | Status |
|---|---|---|
| `AGENTS.md` only (repo3a) | token seen **1×** | SMOKE |
| `CLAUDE.md` = `@AGENTS.md` + `## Claude-specific` (repo3b) | token seen **1×** (loaded once, not twice) | SMOKE |
| `CLAUDE.md` **without** import, plus `AGENTS.md` (repo3c) | token seen **0×**: `AGENTS.md` is ignored | SMOKE |

- Rule (DOCS): `AGENTS.md` is read natively **only if no `CLAUDE.md`/`CLAUDE.local.md` exists in the working directory or above**. A "Project instructions" setting can change it.
- `@path` imports (DOCS): relative to the importing file, absolute allowed, max **4** hops, ignored inside code spans/fences, external (outside the working dir) paths need a one-time approval dialog. Same-directory `@AGENTS.md` is not external.
- `CLAUDE.md` is also loaded from every parent directory (root→cwd, concatenated); subdirectory ones load lazily. User scope `~/.claude/CLAUDE.md` is loaded too. DOCS
- Block-level HTML comments in `CLAUDE.md` are stripped before injection (DOCS). The §4.3 generated header written as an HTML comment therefore costs no context in Claude.

## 4. Claude Code — hooks (spec §12.4, §7.5)

| Item | Spec assumes | Verified now | Status / how |
|---|---|---|---|
| Config shape | neutral list | JSON in settings: `hooks.<Event>[] → {matcher, hooks:[{type, command, args?, if?, timeout?, onFailure?}]}` | DOCS; SMOKE (works) |
| Event names | `session_start, pre_tool, post_tool, stop, subagent_stop, prompt_submit` | PascalCase: `SessionStart, UserPromptSubmit, PreToolUse, PostToolUse, Stop, SubagentStop, …` (30+ events: `PermissionRequest`, `PreCompact`, `InstructionsLoaded`, …) | DOCS; SMOKE: warning text lists valid events |
| Unknown event | must not break startup | `NotARealEvent` in `.claude/settings.json`: warning `Unknown hook event … was ignored`, the rest of the file stays in effect, run succeeded. | SMOKE |
| stdin JSON | — | `session_id, transcript_path, cwd, prompt_id, permission_mode, effort, hook_event_name, tool_name, tool_input, tool_use_id` | SMOKE (captured from a `PreToolUse` Bash hook) |
| Project dir variable | "project-dir variable" | `${CLAUDE_PROJECT_DIR}` is expanded in `command` **and** exported as env var `CLAUDE_PROJECT_DIR` (value = repo root). | SMOKE |
| Block | `on_fail: deny` | **Exit 2** blocks (stderr = reason; model saw `PreToolUse:Bash hook error: … BLOCKED-BY-HOOK-A`). **Exit 1 does not block.** JSON decisions are also supported. | SMOKE (exit 2) + DOCS (exit 1) |
| Merge across scopes | "all combined, stack order" | Entries from `settings.json` and `settings.local.json` **both fired** on one tool call. Docs: user/project/local/plugin hooks add to managed ones; identical handlers dedupe; matching hooks run **in parallel**. **No ordering guarantee.** | SMOKE (project+local both ran), DOCS (parallel, user scope). User+project merge not tested (needs write to `~/.claude`). |
| Disable | `disabled`/`locked` | `disableAllHooks` exists; "the value left after normal settings precedence applies". Only managed settings protect managed hooks. | DOCS |

## 5. Claude Code — skills in parent/nested directories (spec §12.5, §4.2)

| Fixture | Observed | Status |
|---|---|---|
| Non-git parent dir `parent/.claude/skills/parent-skill`, session started in git repo `parent/child` | `parent-skill` **not** loaded; `child-skill` loaded | SMOKE |
| `repo2/sub/.claude/skills/nested-skill`, session started at `repo2/` | not in `init.skills` at start (lazy) | SMOKE |
| Parents **inside** a git repo | Documented: `.claude/skills/` of the start dir and every parent **up to the repo root**; nested ones load lazily on first file access; in a linked worktree the search stops at the worktree root. | DOCS |

So a client root that is **not** a git repo cannot inject skills (matches the spec's "should not be a git repo"). A client root that *is* a repo would. The tool should check for it.

Other Claude facts that touch the spec (DOCS):
- Commands→skills flag is `disable-model-invocation: true` (description removed from context, cannot be preloaded, model calls are blocked). `user-invocable: false` is the inverse.
- Skill frontmatter unknown fields are silently ignored (**SMOKE**: `unknownFutureField` on `proj-only` loaded).
- Claude does **not** read `.agents/skills/`; Codex does not read `.claude/skills/` (DOCS). Both dirs are needed.

## 6. Codex 0.169.0 (spec §12.6, §7, §8.2)

### 6.1 `AGENTS.md` (SMOKE via `codex debug prompt-input`, run from `repo/sub`)

| Fixture | Observed |
|---|---|
| root has `AGENTS.md` **and** `AGENTS.override.md`; `sub/` has `AGENTS.md` | Root: **only** `ROOT-OVERRIDE-MD` appeared (`ROOT-AGENTS-MD` count 0). `sub/`: `SUB-AGENTS-MD` appeared after it. |
| `sub/deeper/TEAM.md` + `-c project_doc_fallback_filenames=["TEAM.md"]` | `DEEPER-FALLBACK` loaded in addition to root/sub files. |
| Trust | These loaded with the project **untrusted**. |

Documented (DOCS): global `~/.codex/AGENTS.override.md` else `AGENTS.md`; per directory from repo root to cwd, first of `AGENTS.override.md` > `AGENTS.md` > fallback names; one file per directory; concatenated root→cwd; capped at `project_doc_max_bytes` (**32 KiB** default).

### 6.2 Skills

| Item | Observed | Status |
|---|---|---|
| Repo skills | `.agents/skills/cx-skill` (repo root) and `sub/.agents/skills/sub-skill` (cwd) were both in the skills catalog, with file paths. | SMOKE |
| Above repo root | `cx/.agents/skills/above-skill` (parent of the git root) was **not** found. | SMOKE |
| Locations | `$CWD/.agents/skills` up to repo root, `$HOME/.agents/skills`, `/etc/codex/skills`, bundled system skills. Symlinked skill dirs followed. | DOCS |
| Same-name | Codex **does not merge** same-name skills; "both can appear in selectors". No documented precedence. Disable with `[[skills.config]] name/path/enabled=false` (last match wins). | DOCS |
| Explicit-only skills | `agents/openai.yaml` → `policy.allow_implicit_invocation: false` (explicit `$skill` still works). | DOCS |
| Frontmatter | `name` and `description` required. | DOCS |

### 6.3 Custom agents

| Item | Documented | Status |
|---|---|---|
| Location/format | `~/.codex/agents/` and `.codex/agents/`, one TOML per agent. | DOCS |
| Fields | required `name`, `description`, `developer_instructions`; optional `model`, `model_reasoning_effort`, `model_reasoning_summary`, `model_verbosity`, `features.{shell_tool,apps,plugins,memories,request_permissions_tool}`, `skills.config`, `skills.bundled.enabled`, `skills.include_instructions`. | DOCS |
| Not per agent | `sandbox_mode`, `approval_policy`, `model_provider`, `mcp_servers` do **not** override the parent. | DOCS |
| Absent | No `tools` allowlist, no preload `skills` list, no isolation, memory or permission-mode field. | DOCS (absence) |
| Second mechanism | `[agents.<role>]` in `config.toml` with `config_file`, `description`; plus `[agents]` `max_concurrent_threads_per_session`, `default_subagent_model`. | DOCS |
| User vs project same name | **Not documented.** | UNVERIFIED |
| Loading in 0.169.0 | `codex debug prompt-input` does not list agents, and the project was untrusted. `features list`: `multi_agent` = deprecated/true, `multi_agent_v2` = stable/false; which flag governs custom agent files was not determined. | UNVERIFIED |

### 6.4 Hooks

| Item | Documented | Status |
|---|---|---|
| Feature flag | `hooks` = stable, **true** (`codex features list`). Alias `codex_hooks` accepted. | HELP + DOCS |
| Location | `/etc/codex/hooks.json`, `$CODEX_HOME/hooks.json`, project `.codex/hooks.json` (**trusted projects only**), or inline `[[hooks.<Event>]]` in `config.toml`. Both formats in one layer both load, with a startup warning. | DOCS |
| Events | `PreModelRequest, PostModelRequest, PreToolUse, PermissionRequest, PostToolUse, PreCompact, PostCompact, SessionStart, SessionEnd, SubagentStart, SubagentStop, UserPromptSubmit, Stop, Interrupt` | DOCS |
| Handler | `type` = `command` or `mcp_tool`; `command`, `timeout` (s, default 600), `statusMessage`, `async`. `prompt`/`agent` handlers are parsed but **skipped**. | DOCS |
| Blocking | exit **2** (stderr = reason), or JSON `hookSpecificOutput.permissionDecision: "deny"`. | DOCS |
| Merge | Matching hooks from all layers run, **concurrently**. Identical command in two files can run twice. | DOCS |
| **Trust** | Non-managed hooks must be **reviewed and trusted** (hash of the hook) via `/hooks` before they run; new or changed hooks are skipped until trusted. `--dangerously-bypass-hook-trust` skips it per invocation. | DOCS |
| Project dir variable | **None documented**; use `git rev-parse --show-toplevel`. `PLUGIN_ROOT`/`PLUGIN_DATA` are plugin-only. | DOCS |
| Coverage | `PreToolUse` covers shell, unified exec, `apply_patch`, MCP and local function tools; not hosted tools such as web search. | DOCS |
| Local test | Project untrusted, so `.codex/hooks.json` was not exercised. | UNVERIFIED |

### 6.5 MCP

| Item | Value | Status |
|---|---|---|
| Table | `[mcp_servers.<id>]` | DOCS |
| stdio keys | `command`, `args`, `cwd`, `env` (map, values!), `env_vars` (array of **names** to pass through) | DOCS |
| HTTP keys | `url`, `bearer_token_env_var`, `http_headers`, `env_http_headers`, `auth`, `scopes` | DOCS |
| Lifecycle/filter | `enabled`, `required`, `enabled_tools`, `disabled_tools`, `default_tools_approval_mode`, `startup_timeout_sec`, `tool_timeout_sec` | DOCS |
| Project MCP | Needs project trust (the warning text says so). In the untrusted project, `codex mcp list` showed only the user-level server, not the project's `ctx7`/`pw`. | SMOKE (untrusted) |
| CLI | `codex mcp {list,get,add,remove,login,logout}` with `--json` | HELP |

### 6.6 Config, trust, permissions

| Item | Value | Status |
|---|---|---|
| Project config | `.codex/config.toml` loads only for **trusted** projects. Untrusted projects skip project config, hooks and rules. Some keys (`model_provider`, `notify`, `otel`, auth/profile/telemetry routing) cannot be set at project level. | DOCS |
| Untrusted behaviour | Startup still works: `codex exec` printed a warning and continued, **even with malformed TOML in the untrusted `.codex/config.toml`** (no parse error shown). | SMOKE |
| Trusted behaviour with unknown keys, bad values or bad hook events | Not observable without trust. | UNVERIFIED |
| Approval/sandbox | `approval_policy` ∈ `on-request`, `never`, granular table (`untrusted` unsupported, `on-failure` deprecated); `sandbox_mode`, `sandbox_workspace_write.*`. Observed `exec` banner: `approval: never`, `sandbox: read-only`, `model: gpt-6.1-sol`. | DOCS; banner SMOKE |
| Other permission surfaces | config reference lists **named permission profiles**; `codex exec --ignore-rules` and "project … rules" exist. A rules/profile mechanism likely gives allow/deny, which the spec does not use. Syntax not read. | HELP + DOCS, UNVERIFIED detail |
| Where AGENTS/skills need trust | They do not: both loaded untrusted. | SMOKE |

### 6.7 A wrongly typed `config.toml` key (task-80) — checked 2026-11-23

Codex CLI **0.169.0** (`codex --version`: `codex-cli 0.169.0`), macOS (Darwin 25.6.0). Every run used a temporary `CODEX_HOME` (`<tmp>/home-*`), never `~/.codex*`. No login and no model call: the probe is `codex debug prompt-input hi`. Each user config held `approval_policy = "never"`, `sandbox_mode = "read-only"` and `developer_instructions = "MARKER-DEVINSTR-USER"`, plus the key under test. In a valid config, the output shows `Approval policy is currently never`, the sandbox text `read-only` and the marker, so a missing marker means the file was not applied. This section is not yet reflected in the header's date and tool lines, to merge cleanly with other work on this file.

**Result: Codex refuses the whole file.** One key of the wrong type makes the command fail with exit 1 and no output. No other key in that file applies, including the approval and sandbox settings. Nothing is ignored or skipped key by key.

| Case (`config.toml` line under test) | Result | Status |
|---|---|---|
| Control: `tool_output_token_limit = 4000` | exit 0; marker present; approval and sandbox text present | SMOKE |
| `tool_output_token_limit = "lots"` (integer expected) | exit 1, stdout empty, marker absent. stderr: `Error: <tmp>/home-bad/config.toml:4:27: invalid type: string "lots", expected usize`, then `Caused by:` the same text and ``in `tool_output_token_limit` `` | SMOKE |
| `model_reasoning_effort = 42` (string expected) | exit 1: ``invalid type: integer `42`, expected a string`` | SMOKE |
| `hide_agent_reasoning = "yes"` (boolean expected) | exit 1: `invalid type: string "yes", expected a boolean` | SMOKE |
| `tool_output_token_limit = { a = 1 }` | exit 1: `invalid type: map, expected usize` (a table prints as `map`) | SMOKE |
| `hooks = "not-a-table"` | exit 1: `invalid type: string "not-a-table", expected struct HooksToml` | SMOKE |
| `sandbox_mode = "bogus"` (unknown enum value) | exit 1: ``unknown variant `bogus`, expected one of `read-only`, `workspace-write`, `danger-full-access` `` | SMOKE |
| `model_reasoning_effort = "bogus"` (string, unknown value) | exit 0, marker present: an unknown value of a string key is **not** checked at load | SMOKE |
| `totally_unknown_key = 1` | exit 0, marker present: an unknown key is accepted silently | SMOKE |
| `tool_output_token_limit =` (TOML syntax error) | exit 1: `string values must be quoted, expected literal string` plus the TOML parse error | SMOKE |

Facts the rows show:

- The error names the file, the line and column, and the key. For a string or number it also **prints the offending value**. A mistyped secret-like key would therefore end up in the log. The error never lists the file's other values.
- Wrong type and unknown enum variant are errors; an unknown key and an unknown value of a plain string key are not. So Codex's own check is a type check on keys it knows, not an allowlist.
- This differs from §6.6 and §7, which found a malformed file in an **untrusted** project only warned: that file is never read.

**Project-local `.codex/config.toml` behaves the same when trusted.** Trust was granted without touching `~/.codex`: the temporary `CODEX_HOME/config.toml` held `[projects."<tmp>/proj"]` with `trust_level = "trusted"`, and `<tmp>/proj` held an empty `.git` folder and `.codex/config.toml`. This works, unlike the `-c` form in §0.1, so project-layer behaviour can be tested with a temporary home (the form of the grant, not the `-c` flag, is what matters).

| Case (run from `<tmp>/proj`) | Result | Status |
|---|---|---|
| Trusted, project file valid (`developer_instructions = "MARKER-DEVINSTR-PROJECT"`, `tool_output_token_limit = 4000`) | exit 0; project marker present, user marker absent (the project value replaces the user's for the same key) | SMOKE |
| Trusted, project file with `tool_output_token_limit = "lots"` | exit 1, stdout empty, so **no** marker from either file: `Error: <tmp>/proj/.codex/config.toml:2:27: invalid type: string "lots", expected usize`. The valid user config stops applying too. | SMOKE |
| Not trusted (no `projects` table), same wrong-typed project file | exit 0; user marker present, project marker absent: the project file is not read | SMOKE |

Consequence for alz: a single wrongly typed key written into `config.toml`, at user or project level, switches off every Codex setting for that run, and a trusted project file switches off the user's settings too. That is why TASK-82 asks for an allowlist of recorded keys with a checked type for Codex, mirroring TASK-79. alz must also not echo the value in its own error, because Codex does. Not tested: other commands (`codex exec`, `codex mcp list`) were not run, so the same refusal there is expected but not observed.

Commands (fixtures made with `printf`; `<tmp>` stands for the scratch folder):

```sh
CODEX_HOME=<tmp>/home-ok  codex debug prompt-input hi   # control: exit 0
CODEX_HOME=<tmp>/home-bad codex debug prompt-input hi   # exit 1, see rows above
(cd <tmp>/proj && CODEX_HOME=<tmp>/home-trust codex debug prompt-input hi)
```

## 7. Both — unsupported items must not break startup (spec §12.7)

| Case | Result | Status |
|---|---|---|
| Claude, agent frontmatter: unknown field, `memory: none`, `isolation: none` | loads and runs | SMOKE |
| Claude, skill frontmatter: unknown field | loads | SMOKE |
| Claude, unknown hook event | warning, ignored, run succeeds | SMOKE |
| Claude, unknown *settings* key, malformed `settings.json` | not tested | UNVERIFIED |
| Codex, untrusted project with unknown keys, bad hook event, malformed TOML | warning only, run starts | SMOKE |
| Codex, trusted project, same cases | not testable here | UNVERIFIED |
| Claude project `.mcp.json` | `claude mcp get` help: unapproved `.mcp.json` servers show `⏸ Pending approval` and are not connected, so generated servers stay off until approved (or pre-approved via settings; setting names not checked). | HELP |

## 7b. Claude permissions (spec §7.6, §8.1) — DOCS (permissions page)

- Rule syntax is `Tool(pattern)`: `Read(./.env)`, `Read(*.env)`, `Bash(git push *)`, `Edit(docs/**)`. Not `read:.env*`.
- Order is **deny → ask → allow**; the first match decides; an allow cannot carve out of a deny. "If a tool is denied at any level, no other level can allow it."
- `Read` deny applies best-effort to Grep/Glob/`@file` mentions; Grep/Glob need `Read(...)` patterns, not `Grep(...)`.
- Path anchoring: `//abs`, `~/`, `/x` = relative to the settings file's project root, `./x`/`x` = cwd; bare filenames follow gitignore semantics. `!pattern` carve-outs only work within the same settings source.
- `PreToolUse` hook decisions do not bypass deny/ask rules.

## 7c. Additional directories, path-scoped rules, agent memory (spec §12.8, §12.9) — DOCS

Checked against `code.claude.com/docs/en/{settings,permissions,skills,memory,sub-agents}` on 2026-11-19. Not smoke-tested (a trusted workspace and a model call are needed).

| Item | Spec assumes | Verified now | Status |
|---|---|---|---|
| Setting key | "additional-directory setting" | `permissions.additionalDirectories` (array) in `.claude/settings.json`; `--add-dir` / `/add-dir` at runtime. | DOCS |
| Side-effect loading | "skills, rules or instructions inside that directory get loaded?" | **Depends on how the dir is added.** `permissions.additionalDirectories` "grant[s] file access only and loads none of" skills, commands or agents. `--add-dir`/`/add-dir` (and the Agent SDK option) **do** load the dir's `.claude/skills`, `.claude/commands`, `.claude/agents`; an added-dir skill loses to a project skill of the same name and gets a `<dir>:` prefix. CLAUDE.md from added dirs loads only with `CLAUDE_CODE_ADDITIONAL_DIRECTORIES_CLAUDE_MD=1` (then also its `.claude/rules/*.md` without `paths`). | DOCS |
| Trust | — | `permissions.additionalDirectories` in a project `.claude/settings.json` applies only after the workspace trust dialog is accepted (`-p` runs print a "not trusted" warning). | DOCS |
| Relative paths | `../hub` | Not stated for `additionalDirectories`. Permission rules: `/x` = settings source root, `x` = cwd. The adapter emits the manifest path as written. | UNVERIFIED |
| Rules location | `.claude/rules/*.md` | Yes; `.md` files found **recursively**; rules without `paths` load at launch like `.claude/CLAUDE.md`. User rules `~/.claude/rules/` load before project rules and neither overrides the other. | DOCS |
| Rules frontmatter | `paths: [<globs>]` | `paths` is the **only** field read (YAML list or comma string); other fields ignored. Globs relative to the dir that holds `.claude/`; `**`, `*`, braces; a leading `/` anchors to the base dir. **Frontmatter that fails to parse makes the rule load as if it had no `paths`.** | DOCS |
| Rules loading | — | A path-scoped rule loads when Read/Write/Edit (or a read-like Bash command) touches a matching file. | DOCS |
| Memory dirs | `.claude/agent-memory/` | `memory: project` → `.claude/agent-memory/<agent>/` (meant to be committed); `local` → `.claude/agent-memory-local/<agent>/`; `user` → `~/.claude/agent-memory/<agent>/`. Has no effect when auto memory is off. | DOCS |
| Codex | equivalent setting | Not checked: there is no Codex adapter yet (see §9). | UNVERIFIED |

Consequences in the code:
- The Claude adapter emits `workspace` dirs as `permissions.additionalDirectories`, which does **not** pull the hub's own generated `.claude/` into code repos. `alz doctor` notes when an added dir has `.claude/{skills,agents,commands}`, because a manual `--add-dir` would load them.
- Rules are rendered with generated (always valid) YAML frontmatter, and a base rule must have `paths`.
- `.claude/agent-memory/**` and `.claude/agent-memory-local/**` are protected paths of the Claude adapter.

---

## 8. Required spec changes

Numbered; each names the section and the fact behind it. "Proposed" means a suggestion, not verified.

1. **§7.2 agent schema → native mapping** (§1 here). Adapters must emit camelCase names; omit `memory` when neutral value is `none`; map `isolation` only when `worktree`; map `permission_mode` through a table (`auto`, `dontAsk`, `acceptEdits`, `plan`, …); allow `local` memory; add `disallowedTools`, `maxTurns`, `effort` if wanted; cap preload `skills` at 32. Because unknown fields are silently ignored, a typo is never reported: the adapter must validate against its own list of fields for the verified version.
2. **§7.2 `tools` vocabulary and narrowing rule.** Claude agent `tools` takes tool names only; `write:docs/**` and `bash:mise run *` cannot be expressed per agent. Decide: (a) neutral `tools` become plain tool names, with scoping moved to `permissions.yaml`; or (b) scoped entries compile to a `PreToolUse` hook keyed on `agent_type`. Update the narrowing rule to apply to tool *names*. Codex has no `tools` field at all, so it needs a warning or a prose restriction.
3. **§7.2/§8.2 Codex agent adapter.** No per-agent tools, skills preload, isolation, memory, permission mode, sandbox, approval or MCP. Only `name`, `description`, `developer_instructions`, model and reasoning keys, `skills.config`, `features.*`. Replace "instructions point to a generated body file" with inline `developer_instructions` (no file reference is documented for the standalone TOML; `config_file` belongs to `[agents.<role>]` in `config.toml`). Skill preload becomes text in `developer_instructions`. Pick one of the two agent mechanisms and verify it under a trusted project.
4. **§7.1/§8.1 instructions.** Keep the thin `CLAUDE.md` with `@AGENTS.md` (loads once; **SMOKE**). Note that a `CLAUDE.md` *without* the import makes Claude ignore `AGENTS.md`. Add checks: Codex `AGENTS.override.md` anywhere between repo root and cwd silently shadows the generated `AGENTS.md` in that directory (**SMOKE**), so treat it as a path collision; add a **byte** budget next to `instructions_max_lines` (Codex stops at 32 KiB, `project_doc_max_bytes`); the user-scope `~/.claude/CLAUDE.md` and `~/.codex/AGENTS*.md` are also loaded, so `alz doctor` should list them.
5. **§7.5 hooks.**
   - Event map: `session_start→SessionStart`, `pre_tool→PreToolUse`, `post_tool→PostToolUse`, `stop→Stop`, `subagent_stop→SubagentStop`, `prompt_submit→UserPromptSubmit` (all exist in both tools, per DOCS; Codex not smoke-tested).
   - `on_fail: deny` ⇒ **exit 2** (exit 1 does not block); `warn`/`feedback` need an explicit design.
   - Delete "execution order is stack order": both tools run matching hooks concurrently. Order can only be guaranteed inside one generated wrapper script.
   - `${CLAUDE_PROJECT_DIR}` is right for Claude (**SMOKE**); Codex has no equivalent, so Codex hook scripts must resolve the root themselves (`git rev-parse --show-toplevel`).
   - **Codex hooks need a human trust step** per hook hash; `.codex/hooks.json` in an untrusted project never runs. `locked`/`disabled` are build-time rules only; at runtime `disableAllHooks` can turn Claude hooks off outside managed settings. State that hooks are guardrails, not a security boundary, and that only managed settings enforce one.
   - Codex: use `hooks.json` **or** inline `[hooks]` per layer, not both.
6. **§7.6/§8.1 permissions.** Compile to Claude syntax `Tool(pattern)` (`read:.env*` → `Read(.env*)`, `bash:git push *` → `Bash(git push *)`); document path anchoring. Deny-first and "no level can allow what another denies" match the floor (**DOCS**). **§8.2/§9.3/§13.7:** Codex has no allow/ask/deny list in `config.toml` (only `approval_policy`, `sandbox_mode`, named permission profiles and rules files, not yet read), so the floor test applies to the Claude target; Codex mapping needs a follow-up fact check.
7. **§7.3/§7.4/§13.5 skills.**
   - "Personal beats project" is confirmed for Claude (**SMOKE**) and a losing project skill is dropped without a warning, so `alz doctor` is mandatory.
   - Codex does not merge or rank same-name skills (both appear), so §7.3's "most specific wins" needs doctor for Codex too.
   - Commands: Claude `disable-model-invocation: true`; Codex needs a generated `agents/openai.yaml` with `policy.allow_implicit_invocation: false`. A Claude command-skill cannot be preloaded into an agent, so a slot that points at a command is an error.
   - Both tools stop searching parents at the **git repo root** (**SMOKE**). Doctor check: a client root that is itself a git repo is a leak path.
   - Skills dirs differ: `.claude/skills/` (Claude) and `.agents/skills/` (Codex); neither tool reads the other's.
8. **§7.7 MCP.** Claude project `.mcp.json` servers need approval before they connect; Codex project MCP needs project trust. Document both as manual steps, or decide to also emit approval settings (names not verified). `env_from` maps to Codex `env_vars` (names only). Codex also has `enabled_tools`/`disabled_tools` that the neutral schema could expose.
9. **§7.8 models.** Claude model values are `sonnet|opus|haiku|fable|<full id>|inherit`; Codex takes a model id per agent or default (`gpt-6.1-sol` observed in the exec banner). Keep concrete ids only in `models.yaml` as the spec says; add the Claude alias set as the validation list.
10. **§8.2/§8.3 Codex trust.** The generated `.codex/` (config, hooks, MCP, rules) is inert until the project is trusted, and `-c` cannot grant trust. Add to the build report and to `alz doctor` a "needs trust" notice. `AGENTS.md` and `.agents/skills/` work without trust (**SMOKE**). CI smoke tests (§13.13) need `codex exec --trust` or a pre-seeded `CODEX_HOME`.
11. **§13.12 account independence test.** An empty `CLAUDE_CONFIG_DIR` has no login, so a model-driven listing cannot run there. Re-specify: compare static scans of both account dirs for colliding names (no login needed), and run the init-event listing only where credentials exist (CI with an API key). For Codex use `codex debug prompt-input` (no login) as the listing source.
12. **§13.13 smoke test** can use `codex debug prompt-input` and `codex doctor --json` (no login) for Codex, and the init event for Claude; both need no extra tooling.
13. **§1 / §8.4 wording.** Split the precedence claim: skills (personal > project) are **verified**; agents (project > user) are **documented only** on this machine.
14. **§12 itself.** Replace the checklist with this document's results and add the adapter version ranges: Claude Code verified at **2.1.311**, Codex at **0.169.0**. Field-introduction versions worth recording: `autoCompactWindow` ≥2.1.296, `omitClaudeMd` ≥2.1.271, `onFailure` hook field ≥2.1.295.

## 9. Still to verify before the Codex adapter is written

These need Codex trust (granted by the user, e.g. `codex exec --trust` in a throwaway repo, which edits `~/.codex/config.toml`) or a login. I stopped rather than touch user config.

- Whether `.codex/agents/*.toml` loads, and which of user/project wins on a same name.
- Trusted-project behaviour for unknown keys, bad values and unknown hook events (§7 above).
- `hooks.json` format and trust flow end to end; stdin shape.
- Project `[mcp_servers.*]` actually starting, and `enabled = false` honoured.
- Codex rules / permission profiles syntax (relevant to §7.6).
- Claude: project-vs-user agent precedence and user+project hook merge (need a write to `~/.claude`, or a dedicated logged-in test account).

## 10. Commit attribution (spec §7.12) — checked 2026-11-21

The spec relies on **none** of these: the Co-Authored-By rule is a generated instruction only. They are recorded so nobody builds on them by accident, and because a user setting can hide Claude's trailer.

| Item | Spec assumes | Verified now | Status / how |
|---|---|---|---|
| Claude default trailer | nothing | In a scratch git repo with no settings of its own, Claude Code 2.1.323 reports that its instructions say to end commit messages with `Co-Authored-By: <model name> <noreply@anthropic.com>`. | SMOKE: `claude -p "…quote that trailer line verbatim… otherwise NONE" --max-turns 1` (model-reported; the real user settings were in effect, see §0.1) |
| Claude settings | nothing | `attribution` ("customize the attribution Claude Code adds to commits and pull requests"), `attribution.commit` (change or hide the commit trailer), `attribution.pr`, `attribution.sessionUrl`; `includeCoAuthoredBy` is deprecated in favour of `attribution`. Scope: any settings file (user, project, local, managed). Default values not read. | DOCS (`code.claude.com/docs/en/settings-reference`, settings index) |
| Codex built-in trailer | nothing | `codex_git_commit` feature is **removed** (`false`). `codex debug prompt-input "commit"` in a scratch git repo contains no `Co-authored-by` text, with or without `-c features.codex_git_commit=true`. So Codex 0.169.0 adds no trailer by itself. | HELP (`codex features list`) + SMOKE (prompt-input, untrusted project) |
| Codex `commit_attribution` key | nothing | Vendor docs describe `commit_attribution` in `config.toml` (default `Codex <noreply@openai.com>`, `""` disables), effective only with `[features].codex_git_commit`, which 0.169.0 reports as removed. | DOCS (`developers.openai.com/codex/config-reference`), not effective here |

Consequence: alz writes no attribution setting. With Claude the generated rule and the tool's own trailer agree; with Codex the rule is the only source of the trailer. A layer can still set Claude's `attribution` through `settings.yaml`.

## 11. Git behaviour the commit-identity checks rely on (spec §9.4) — git 2.50.1

Not an agent target, but §9.4 is built on it. Scratch repo with `HOME` pointed at a scratch dir holding a `.gitconfig` with a reserved-domain identity, and `GIT_CONFIG_NOSYSTEM=1`.

| Item | Observed | Status |
|---|---|---|
| Origin and scope | `git config --show-origin --show-scope --get user.email` prints `<scope>\t<origin>\t<value>`; e.g. `global\tfile:<home>/.gitconfig\t…`, `local\tfile:.git/config\t…` (local origin is relative). | SMOKE |
| Command-line config | `git -c user.email=…` and `GIT_CONFIG_COUNT`/`GIT_CONFIG_KEY_0`/`GIT_CONFIG_VALUE_0` both show scope `command`, origin `command line:`. | SMOKE |
| Environment overrides | `GIT_AUTHOR_EMAIL` changes `git var GIT_AUTHOR_IDENT` but is invisible to `git config`. So the effective identity must come from `git var`, and environment variables must be checked separately. | SMOKE |
| Effective identity | `git var GIT_AUTHOR_IDENT` / `GIT_COMMITTER_IDENT` print `Name <email> <epoch> <tz>`. | SMOKE |
| Includes | An `[include] path = extra.inc` in `.git/config` overrides a `user.email` set in `.git/config`; origin shows `file:.git/extra.inc`, scope `local`. So a write must be followed by a re-read. | SMOKE |
| Local scope | Several scopes: `--get` returns the winning value with its origin; `--get-all` lists every scope in order. `--local` outside a repo exits 128; `--get` of an unset key exits 1. | SMOKE |
| Subdirectory | From a subdirectory, `git rev-parse --show-toplevel` gives the parent work tree, so a repo root that is not the top level would write into another repo's config unless checked. | SMOKE |
| Name trimming | `git var GIT_AUTHOR_IDENT` removes these from both ends of a configured name: `,` `:` `;` `"` `'` `\` `<` `>` and whitespace (`,Owner;` gives `Owner`; `.Owner<` gives `.Owner`). A `.` at either end is kept here, and so is a `.` in the local part of an email (`.o.@example.com`), but older git versions may treat `.` as trimmable too, so the spec rejects it at the edges anyway (not checked on older versions). A name with an apostrophe inside (`O'Example`) is kept unchanged. | SMOKE (scratch repo, git 2.50.1) |
| Broken config | A malformed global config makes `git rev-parse --git-dir`, `git var` and `git config` all exit 128, the same status as "not a git repository". Only the message text tells them apart (`fatal: not a git repository`), so alz reads it with `LC_ALL=C`. | SMOKE |
| Clone argument is tried as a local path first | With a bare repo in a folder named `x:y` in the working directory, `git clone -- x:y out` clones that folder (git 2.50.1: local path wins over the `host:path` reading). With `file:///nonexistent-dir/r` and a bare repo at `file:/nonexistent-dir/r` relative to the working directory, git 2.50.1 does not clone it: it fails with `protocol '<cwd>/file' is not supported`, with or without the setting below. Both are treated as local by alz, since other versions may differ. | SMOKE (scratch dirs, git 2.50.1) |
| `protocol.file.allow=never` | `git -c protocol.file.allow=never clone -- x:y out` with that folder present fails with `fatal: transport 'file' not allowed` and clones nothing; a plain local `q` (folder `q.git`) is also refused. A genuinely remote name (`nohost.invalid:z`) fails as before with a connection error. So the setting blocks the local-path fallback, and alz adds it to every clone of a source it classed as remote. | SMOKE (git 2.50.1) |
| `GIT_ALLOW_PROTOCOL` | With `GIT_ALLOW_PROTOCOL=file` in the environment, `git -c protocol.file.allow=never clone -- x:y out` clones the local folder `x:y` anyway: the variable overrides the setting. So alz removes it from the environment of a guarded clone. `GIT_CONFIG_COUNT`/`GIT_CONFIG_KEY_0=protocol.file.allow`/`GIT_CONFIG_VALUE_0=always` does not override `-c`: the clone is still refused. | SMOKE (git 2.50.1) |
| `..` and symlinks | The operating system resolves `..` on disk through symlinks, while `path.resolve` resolves it as text, so `link/../../x.git` can name two different folders. alz therefore refuses `..` in a local `git:` source, and checks and clones the canonical path of the folder found by looking up the string as written. | REASONED from path semantics; covered by an acceptance test |
| Local clones of untrusted repositories | Git's security advisories list CVE-2022-39253 (local clone follows symlinks, fixed in 2.38.1) and CVE-2024-32004 and CVE-2024-32021 (local clone of a crafted repository, fixed in 2.45.1). A `git:` source that is a local path is such a clone, so use git 2.45.1 or later when layers may come from untrusted repositories. | DOCS (git security advisories as cited; not re-verified here) |
| Minimum version | `git config --show-scope` is listed in the git 2.26.0 release notes ("git config" learned to show in which "scope", in addition to in which file, each config setting comes from), and `{author,committer}.{name,email}` in the 2.22.0 notes. So the checks need git 2.26 or later. | DOCS (`Documentation/RelNotes/2.26.0.txt` line 48 and `2.22.0.txt` line 25 in git.git, fetched 2026-11-21); not run on those versions |

## 12. Account selection (spec §12 items 10–21, §7.13) — checked 2026-11-22

Claude Code **2.1.323**, Codex CLI **0.169.0** (SOURCE = Rust code at tag `rust-v0.169.0` of `github.com/openai/codex`), mise **2026.9.1**, macOS (Darwin 25.6.0). direnv is not installed. No Linux machine was available.

**Method.** SMOKE runs used `env -i` with a fake `HOME=<tmp>/home`, the variable pointing at a folder under `<tmp>`, stdin from `/dev/null` and a timeout. There was no login or logout, no credential was read, and no `security` command was run. Claude was never started with `CLAUDE_CONFIG_DIR` unset or empty, because it would then read the real default Keychain entry. Marker files in `<tmp>/home/.claude/settings.json` (a `SessionStart` hook), `<tmp>/home/.claude/skills/`, `<tmp>/home/.claude.json`, `<tmp>/home/.codex/AGENTS.md` and `<tmp>/home/.codex/config.toml` show whether a run reads the default folder. DOCS = `code.claude.com/docs/en/{authentication,env-vars,settings-reference,claude-directory,permissions,errors,skills,plugins/loading,hooks}`, fetched 2026-11-22.

**New no-login probe (corrects §0.1 and §8 item 11).** With an empty `CLAUDE_CONFIG_DIR`, `claude -p "…" --output-format stream-json --verbose --max-turns 1` still emits the `system`/`init` event (`skills`, `agents`, `plugins`, `memory_paths`) and runs `SessionStart` hooks. Only then does it fail with `authentication_failed` and the result `Not logged in · Please run /login` (exit 1). **SMOKE**

| Item | Spec (§7.13) assumes | Verified now | Status / how | Agrees? |
|---|---|---|---|---|
| 10 Claude: folders | unset: `~/.claude` + `~/.claude.json`; set: everything in the folder | "If you set `CLAUDE_CONFIG_DIR`, every `~/.claude` path … lives under that directory instead. The same applies to `.claude.json`." With `CLAUDE_CONFIG_DIR=<tmp>/home/.claude-a`, one `-p` run created `.claude-a/.claude.json` (0600), `plugins/` and `sessions/` (0700), and `backups/`, `projects/<cwd>/<session>.jsonl` and `session-env/` (0755). `<tmp>/home/.claude.json` was unchanged. | DOCS (claude-directory) + SMOKE | yes |
| 10 Claude: `~/.claude` is not the default | default = variable unset | Setting the variable to `~/.claude` is a different account: `.claude.json` moves into the folder and the Keychain entry is keyed to the folder (item 12). | DOCS (inferred from the two statements; not run) | yes; the hook must `unset`, never export `~/.claude` |
| 10 Claude: trust | per account folder | Trust is stored in `.claude.json` as `projects["<path>"].hasTrustDialogAccepted`, keyed on the git repository root (the main checkout for a worktree), or on the start folder outside a repository. Trust given when starting in `$HOME` lasts for that session only. `-p` never shows the dialog, and the `-p` run wrote no `projects` entry. So trust follows the account folder. | DOCS (permissions, errors) + SMOKE (no entry written) | yes. Seeing a second account ask again needs a login: owner-only O1 |
| 10 Claude: read only at start | a running session keeps its account | **UNVERIFIED.** DOCS: `CLAUDE_CONFIG_DIR` may be set in "your shell, user settings, or managed settings". A settings `env` value overrides the shell value and is applied again in a running session when the file changes. The docs do not say whether the running process then changes folder; its subprocesses get the new value. | DOCS (env-vars, settings-reference); owner-only O4 | open; see change C6 |
| 11 Codex: folder | `~/.codex`, or `CODEX_HOME` | `find_codex_home()`: a set, non-empty value must exist and be a directory, and is canonicalized (symlinks resolved). An **empty value counts as unset**. Unset gives `~/.codex`, which is not checked to exist. With an empty `<tmp>` folder, `codex login status` printed `Not logged in` (exit 1) and created `installation_id` (0644), `.sandbox_migration` (0600), `shell_snapshots/`, `skills/.system/` and `tmp/arg0-macos/` (0755). | SOURCE (`utils/home-dir/src/lib.rs`) + SMOKE | yes |
| 11 Codex: logins, config, trust | per folder | `cli_auth_credentials_store` defaults to `file`: `$CODEX_HOME/auth.json`. `keyring` and `auto` use the OS keyring (item 12); `ephemeral` keeps the login in memory. `config.toml` is read from `$CODEX_HOME`: the `developer_instructions` marker in `<tmp>/home/.codex/config.toml` reached `codex debug prompt-input` only with `CODEX_HOME` unset. Project trust is `[projects."<path>"] trust_level` in that `config.toml` (§0.1, §6.6). | SOURCE (`config/src/types.rs`, `login/src/auth/storage.rs`) + SMOKE + DOCS | yes |
| 11 Codex: read only at start | yes | `find_codex_home()` reads the process environment each time it is called; no config key that sets `CODEX_HOME` was found. Not checked exhaustively, and not run in a live session. | SOURCE (partial) | probably |
| 12 Claude, macOS | login keyed by the folder | "keys the macOS Keychain entry to that directory too, so a session with a different `CLAUDE_CONFIG_DIR` reads a different entry". If the Keychain refuses the write, the login goes to `.credentials.json` (0600) in the folder. An empty `<tmp>` folder answered `Not logged in` although this machine has working logins: consistent, but it does not prove the entry's name. The Keychain service name is not documented and was not checked. | DOCS (authentication) + SMOKE (consistent) | yes, for `/login` credentials |
| 12 Claude, Linux | login keyed by the folder | No keyring: `.credentials.json` (0600) under the folder. | DOCS only; **Linux UNVERIFIED**, owner-only O2 | yes |
| 12 Claude: logins outside the folder | — | These are **shared by every folder**: Anthropic profiles in `~/.config/anthropic` (written by `ant auth login` or by a keyless Console sign-in: "Separate directories don't keep two Claude Console sign-ins without an API key apart"); `ANTHROPIC_PROFILE`; the federation variables; `ANTHROPIC_API_KEY`, `ANTHROPIC_AUTH_TOKEN` and `CLAUDE_CODE_OAUTH_TOKEN` in the environment; cloud-provider credentials. Several of these outrank `/login`. | DOCS (authentication, precedence) | **no**: change C3 |
| 12 Codex, macOS and Linux | login keyed by the folder | Default `file`: `auth.json` per folder. `keyring`/`auto`: service `Codex Auth`, account `cli\|<first 16 hex of sha256(canonical CODEX_HOME)>`. The keyring is the macOS Keychain, or on Linux Secret Service plus keyutils. Either way the login is per folder. | SOURCE; Linux UNVERIFIED (O2) | yes |
| 13 Fallback to the default folder | none | **No fallback.** Claude with the variable set: the default `settings.json` hook did not run, `marker-default-skill` was not in `init.skills`, and `<tmp>/home/.claude.json` was unchanged, while the account's own hook ran. Codex with `CODEX_HOME` set: neither default marker (`AGENTS.md`, `config.toml`) appeared in `codex debug prompt-input`; both appeared with it unset. The exception is Claude's profile store in `~/.config/anthropic` (item 12). | SMOKE (both) | yes |
| 14 Claude: symlinked shared items | links work, writes go through | **All six work as links, and writes go through them.** In a `<tmp>` account, `skills`, `agents`, `commands`, `plugins`, `CLAUDE.md` and `settings.json` were absolute links into a separate `<tmp>/shared` folder. The init probe listed the linked skill, agent and command. The `SessionStart` hook from the linked `settings.json` ran. An `InstructionsLoaded` hook reported the linked `CLAUDE.md` as `memory_type: User`, `load_reason: session_start`, with the account path as `file_path`. Writes: the `-p` run wrote `plugins/store/`; `claude plugin marketplace add <tmp>/mkt` and `claude plugin install markerplug@marker-mkt` (scope user) added `extraKnownMarketplaces` and `enabledPlugins` to `settings.json` and wrote `plugins/{cache,marketplaces,installed_plugins.json,known_marketplaces.json}`. All of it landed in `<tmp>/shared`, and `ls -la` still showed all six links. `settings.json` kept mode 0644. A second account linked to the same items loaded the plugin and its skill. `installed_plugins.json` records `installPath` under the **installing** account's path (`<tmp>/home/.claude-a/plugins/cache/…`), not the real path. A per-entry link (`skills/<name>` → a skill folder, inside a real `skills/`) also loads. The claude.ai sync writes `skills/synced/`, `skills/.trash/`, `plugins/synced/` and `plugins/.trash/` (DOCS). These were not run, because the sync needs a login. | SMOKE (2.1.323, 2026-11-22) + DOCS | yes; see C4, C10 |
| 14 Codex: symlinked shared items | links work, writes go through | **Both work as links.** With `AGENTS.md` and `skills` as links in a `<tmp>` `CODEX_HOME`, `codex debug prompt-input` showed the linked `AGENTS.md` marker and the linked skill. Codex then wrote `skills/.system/` (`imagegen`, `openai-docs`, `review-agent`, …) **through the link** into the shared folder, and both links survived. A per-entry link (`skills/<name>` inside a real `skills/`) also loads, and `.system` then stays in the account's own folder. Writes to linked `sessions/` and `memories/` need a login (O7). | SMOKE (0.169.0, 2026-11-22) + SOURCE (`skills/src/lib.rs`) | yes; see C5 |
| 15 Claude: settings change the variable | project settings cannot | Project and local `env` **cannot** set `CLAUDE_CONFIG_DIR`, `HOME`, `XDG_*` or `CLAUDE_CODE_TMPDIR`: each is dropped, with a warning under `claude --debug`. User settings, `--settings` and managed settings **can**, and they override the shell value. | DOCS (settings-reference "Variables Claude Code ignores in `env`") | partly: change C6 |
| 15 Claude: inheritance | hooks, shell, MCP inherit | Hooks, the Bash tool and stdio MCP servers inherit the environment. The hook's environment held `CLAUDE_CONFIG_DIR=<tmp>/home/.claude-a`, and the Bash tool of an interactive session also has it set (only the name was checked). `CLAUDE_CODE_SUBPROCESS_ENV_SCRUB=1` removes it from all three. `CLAUDE_CODE_MCP_ALLOWLIST_ENV=1` gives stdio MCP servers only a baseline environment. | DOCS (hooks, env-vars) + SMOKE (hook, Bash tool) | yes |
| 15 Codex: config changes the variable | project settings cannot | **Not for Codex itself, but yes for its shell commands, from project config too.** Codex finds its folder from the process environment before it reads any `config.toml`, and no key sets `CODEX_HOME` for the process (item 11). `shell_environment_policy` builds the shell tool's environment (`protocol/src/shell_environment.rs` `populate_env`): `inherit` (default `all`; `core` keeps only `PATH`, `SHELL`, `TMPDIR`, `TEMP`, `TMP`, `HOME`, `LANG`, `LC_ALL`, `LC_CTYPE`, `LOGNAME`, `USER`; `none`), then default excludes `*KEY*`/`*SECRET*`/`*TOKEN*`, then `exclude`, then **`set` (any name and value)**, then `include_only`. So `set.CODEX_HOME` or `set.CLAUDE_CONFIG_DIR` changes the value that shell commands see, and `exclude`/`include_only`/`inherit` can drop it. `shell_environment_policy` is **not** in `PROJECT_LOCAL_CONFIG_DENYLIST` (`config/src/loader/mod.rs`, which lists only `openai_base_url`, `chatgpt_base_url`, `model_provider(s)`, `notify`, `profile(s)`, `otel` and a few more), so a trusted project's `.codex/config.toml` may set it. | SOURCE (tag `rust-v0.169.0`); not run (a shell command needs a login, O5) | **no** for shell commands: change C12 |
| 15 Codex: policy patterns | §7.9 refuses a `set` entry that names one of the variables, or an `exclude` pattern that matches one | Three keys hold patterns: the legacy `exclude` and `include_only` lists, and the newer keyed table `filters` (`filters = { "<pattern>" = "include" }` or `"exclude"`), which cannot be mixed with the legacy lists. All three become `WildMatchPattern<'*', '?'>` built with `new_case_insensitive` (`config/src/shell_environment_policy.rs`, `protocol/src/config_types.rs`). `*` matches any run of characters and `?` one character; there are no character classes. wildmatch 2.6.1 (`src/lib.rs` `matches`) folds case by comparing each pattern character's Unicode `to_lowercase()` with the input character's, so `*home*` matches `CODEX_HOME`. A `filters` entry set to `exclude` drops a variable exactly as an `exclude` pattern does. `set` keys are inserted as written on Unix; on Windows, an existing variable whose name equals the key ignoring ASCII case is removed first (`protocol/src/shell_environment.rs` `populate_env`, step 4). | SOURCE (tag `rust-v0.169.0`; wildmatch version from `Cargo.lock`; checked 2026-11-22, Codex 0.169.0 installed); not run | yes: §7.9 names `filters` and the pattern rules (TASK-50.8) |
| 15 Codex: inheritance | hooks, shell, MCP inherit | **Shell tool**: by the policy above, so with the default policy a `CODEX_HOME` set at start reaches it. **Hooks**: start from an empty environment, then replay a snapshot of the Codex process environment taken when the session starts (`hooks/src/registry.rs` `std::env::vars_os()`, `engine/command_runner.rs`). Hooks therefore inherit `CODEX_HOME`, and `shell_environment_policy` does not apply to them. **stdio MCP servers do not inherit it**: they get only `HOME`, `LOGNAME`, `PATH`, `SHELL`, `USER`, `__CF_USER_TEXT_ENCODING`, `LANG`, `LC_ALL`, `TERM`, `TMPDIR`, `TZ`, plus the server's configured `env` and `env_vars` (`rmcp-client/src/utils.rs` `DEFAULT_ENV_VARS`). All three remove `NON_INHERITABLE_ENV_VARS` (federation and internal tokens). | SOURCE (tag `rust-v0.169.0`) | partly: MCP servers do not inherit (C12) |
| 16 Keys that carry or pin a login | per account | Claude `settings.json`: `apiKeyHelper`, `env` (`ANTHROPIC_API_KEY`, `ANTHROPIC_AUTH_TOKEN`; `CLAUDE_CODE_OAUTH_TOKEN` and its refresh token only from user or managed settings), `forceLoginMethod`, `forceLoginOrgUUID`, `otelHeadersHelper`, `sandbox.credentials.*`. A plugin's sensitive `userConfig` values go to the credential store. Codex `config.toml`: `forced_login_method`, `cli_auth_credentials_store`, `mcp_oauth_credentials_store`, `mcp_servers.<id>.{bearer_token_env_var,http_headers,env}`. Not exhaustive: model-provider keys were not checked. | DOCS (Claude) + SOURCE/DOCS (Codex) | yes. `--share settings.json` shares all of these. |
| 17 Claude MCP OAuth | per account (§7.13 table: in `.claude.json`) | Stored in the **credential store**, not `.claude.json`: "Claude Code keeps your sign-ins for OAuth MCP servers in a credential store", which is the Keychain entry keyed to the folder, or `.credentials.json`. `/logout` removes "saved MCP server logins". | DOCS (errors, troubleshoot-install, mcp) | per account yes; location wrong: change C1 |
| 17 Codex MCP OAuth | per account | **Shared across folders by default.** `mcp_oauth_credentials_store` defaults to `auto`: keyring first, else `$CODEX_HOME/.credentials.json`. Keyring service `Codex MCP Credentials`, account `<server name>\|<first 16 hex of sha256 of {type, url, headers}>`. `CODEX_HOME` is part of the key only for `ema-idp:` servers; the source says "The OS keyring is shared across homes". So two accounts with the same server name and URL use one MCP login, on macOS and on Linux with a keyring. Only `file` keeps it per folder. | SOURCE (`rmcp-client/src/oauth.rs`, `config/src/types.rs`); owner-only O3 to observe | **no**: change C2 |
| 18 Claude, missing folder | either way counts as failing closed | **Creates it** with mode 0755 (umask 022), with `.claude.json` 0600, `plugins/` and `sessions/` 0700, and `backups/` and `projects/` 0755. `-p` starts, emits init and answers `Not logged in` (exit 1). An interactive start offers login and setup for the new folder (DOCS). So a blocked folder comes into being at its first blocked start, and a login there would make it usable. | SMOKE + DOCS (authentication) | yes, as §7.13 foresees |
| 18 Codex, missing folder | — | **Refuses to start and creates nothing**: `Error loading configuration: CODEX_HOME points to "<tmp>/…", but that path does not exist` (exit 1); `codex exec` gives `Error finding codex home: …`. A regular file gives `… but that path is not a directory`. Unset with no `~/.codex`: it creates `~/.codex` (0755) and starts. | SMOKE + SOURCE | yes |
| 19 Claude session markers | some variable marks a session | Hook in a `-p` run: `CLAUDECODE=1`, `CLAUDE_CODE_CHILD_SESSION=1`, `CLAUDE_CODE_ENTRYPOINT=sdk-cli`, `CLAUDE_CODE_SESSION_ID`, `CLAUDE_PID`, `CLAUDE_PROJECT_DIR`, `CLAUDE_ENV_FILE`, `CLAUDE_CODE_MESSAGING_SOCKET` and `AI_AGENT=claude-code_2-1-323_harness`. The Bash tool of an interactive session has the same names, with `CLAUDE_CODE_ENTRYPOINT=cli` and `AI_AGENT=claude-code_2-1-323_agent`, plus `CLAUDE_CODE_SESSION_ATTENDED`. DOCS: `CLAUDECODE` is also set in IDE terminals and stdio MCP servers. `CLAUDE_CODE_CHILD_SESSION` (≥2.1.172) is set only for the Bash, PowerShell and Monitor tools, hooks and status line commands, not for MCP servers. `AI_AGENT` is not in the pages read. A child process can unset every one of them, so a marker proves a session but its absence proves nothing. | SMOKE + DOCS (env-vars) | yes, with change C7 |
| 19 Codex session markers | some variable marks a session | **Shell-tool commands**: `CODEX_THREAD_ID`, inserted after `exclude`, `set` and `include_only`, so the policy cannot remove it. `CODEX_SESSION_ID` and `CODEX_VERSION` (`core/src/exec_env.rs` `inject_session_env`). `CODEX_TOOL_CALL_ID`. `CODEX_PERMISSION_PROFILE` when a named profile is active; the source calls it informational, since children can overwrite it. `CODEX_SANDBOX=seatbelt` on macOS when sandboxed (`core/src/sandboxing/mod.rs`). `CODEX_SANDBOX_NETWORK_DISABLED=1` when network is off. Unified-exec processes also get `CODEX_CI=1`, `TERM=dumb` and `PAGER=cat` (`core/src/unified_exec/process_manager.rs`). **Hooks**: the session-start snapshot of the Codex process environment plus per-hook overrides. Which `CODEX_*` names the overrides add was not traced. **stdio MCP servers**: no marker (only the fixed list in item 15). As with Claude, a child can unset each name, and `codex` started from a non-agent shell has none. | SOURCE (tag `rust-v0.169.0`); observing it needs a login (O5) | yes, with change C7 |
| 20 mise | a variable shows what it set | **Yes: `__MISE_DIFF`.** Encoding (`src/env_diff.rs`): base64, standard alphabet without padding, of zlib data, of MessagePack with named fields `{old, new, path}`. `new` holds every variable mise added or changed, with the value it set. `old` holds the earlier value of each one it changed, so a key in `new` but not in `old` was added. `path` lists the folders it added to `PATH`. SMOKE (`env -i`, fake `HOME`, `MISE_TRUSTED_CONFIG_PATHS=<tmp>/m`, so no trust state was written; `<tmp>/m/.mise.toml` with `[env] XDG_CONFIG_HOME = "<tmp>/x"` and a marker): `mise hook-env -s bash` exported both variables and `__MISE_DIFF`, which decoded to `new: {MARKER_MISE, XDG_CONFIG_HOME: <tmp>/x}, old: {}`. With `XDG_CONFIG_HOME=<tmp>/pre` already set, `old` held `<tmp>/pre`. Children of `mise exec` and `mise run <task>` also get `__MISE_DIFF`, which decodes the same. That matters because this repository runs alz through `mise run`. The mise cookbook (`docs/mise-cookbook/shell-tricks.md`, "Inspect what changed after mise hook") decodes it with an external tool. It is a documented recipe, not a stable interface. The value contains every variable mise set **with its value**, including secrets from `[env]` or `_.file`. For alz: Node's `zlib.inflateSync` decodes the zlib layer, but MessagePack needs a small reader of its own, because alz's only runtime dependencies are `yaml` and `diff`. `XDG_CONFIG_HOME` counts as set by mise only when its current value equals `new.XDG_CONFIG_HOME`. A later `export` leaves the diff stale. **NOTE: unverified.** Whether `__MISE_DIFF` is set at all when mise only activates through shims (no shell hook, no `mise exec`, no `mise run`) was not tried; if it is not, alz's mise check (`XDG_*` moved while `__MISE_DIFF` exists) does not fire there. LAYERING-SPEC §7.13 therefore also refuses an `allowed.yaml` whose real path is inside any git work tree, which covers a planted file in a checkout whatever set the variable. | SOURCE (tag `v2026.9.1`) + SMOKE (mise 2026.9.1) + DOCS (cookbook) | new fact: change C11 |
| 20 direnv | — | **Yes: `DIRENV_DIFF`.** Encoding (`gzenv/gzenv.go`): URL-safe base64 **with** padding, of zlib data, of JSON `{"p": {…}, "n": {…}}` (`internal/cmd/env_diff.go`). `n` holds each variable added or changed, with its new value; `p` holds the earlier value of each one changed or removed. Not recorded: `DIRENV_CONFIG`, `DIRENV_BASH`, `DIRENV_IN_ENVRC`, `COMP_WORDBREAKS`, `PS1`, `OLDPWD`, `PWD`, `SHELL`, `__fish*`, `BASH_FUNC_*` and a few more (`IgnoredKeys`). Also set: `DIRENV_DIR` (`-` followed by the `.envrc` folder), `DIRENV_FILE` (the `.envrc` path) and `DIRENV_WATCHES` (gzenv of watched files). These are set in a deferred block "even if load is disallowed or fails" (`internal/cmd/rc.go`), so a set `DIRENV_DIFF` does not prove that an `.envrc` ran. Node built-ins (`Buffer` base64url, `zlib`, `JSON`) decode it. The JSON holds values, including secrets. Not documented in `man/` or the README. | SOURCE only (tag `v2.38.2`; direnv not installed, not run) | new fact: change C11 |
| 21 Claude: deny rules and wrappers | rules can deny; wrappers unknown | **`~` paths: yes.** Permission rules accept `~/` paths (§7b), and sandbox `filesystem.denyRead`/`denyWrite` cover shell commands and their children. A `denyRead` entry does not stop the Read tool, which follows permission rules. **Read/Edit denies** also apply to redirect targets and to `cat`, `head`, `tail`, `sed` and `tee` in a Bash command Claude can analyse. They do not apply to `grep -r` without the file name, or to scripts that open files themselves. **A Bash deny can be bypassed.** Claude strips only `timeout`, `time`, `nice`, `nohup`, `stdbuf`, `command`, `builtin`, `noglob` and bare `xargs`, and a deny rule matches past any leading `VAR=value`. A rule "doesn't match the same program invoked in a different form … isn't a security boundary": `Bash(rm *)` does not stop `/bin/rm …` or `bash -c 'rm …'`, and `mise exec` and `direnv exec` are not stripped. So `Bash(alz account allow*)` stops the usual form only, not `sh -c 'alz account allow …'`, a full path or a script. | DOCS (`permissions.md` "Wrappers", "Read and Edit", `sandboxing.md`, fetched 2026-11-22; 2.1.323) | **no** as a boundary: change C13 |
| 21 Codex: deny rules and wrappers | rules can deny; wrappers unknown | **Partial.** Permission profiles have deny-read patterns, and the pattern parser accepts `~/` (`protocol/src/permissions/target.rs`, `deny_read_validator.rs`). For rules, `shell-command/src/bash.rs` `extract_bash_command` recognises `<shell> -c` and `<shell> -lc <script>`, and `parse_shell_lc_plain_commands` splits a plain script into commands, so a plain `sh -c 'alz account allow …'` is probably matched like the bare command. Not traced: scripts with substitutions or variables, `env`, full paths, nested shells, and whether deny-read reaches the shell tool through the sandbox. | SOURCE (partial, tag `rust-v0.169.0`); behaviour owner-only (O6) | open in detail; C13 applies |
| File names, macOS (§7.9) | a passthrough file name that differs only in case or spelling is a different file | **Not on APFS as set up here.** The volume is case-insensitive and folds more than ASCII case: a file created as `.mcp.jſon` (U+017F, long s) in a scratch folder was opened by `ls .mcp.json`, and `Settings.json` was opened as `settings.json`. So a checked name such as `.mcp.json` or `settings.json` can be reached through a path that does not match it as text. §7.9 compares names with ASCII case folding only and refuses a non-ASCII character in any passthrough path. | SMOKE (macOS APFS, 2026-11-22; Linux file systems not checked) | yes: §7.9 refuses such paths |

### 12.1 Facts that contradict or change §7.13

Applied to LAYERING-SPEC §7.9, §7.13, §8.4, §9.1, §10, §12 and §13 on 2026-11-22. The owner settled C4 and C11 in decision-17: `alz account init` shares nothing by default, Claude `skills/` and `plugins/` are linked entry by entry when shared, and mise and direnv are detected only by the presence of `__MISE_DIFF` and `DIRENV_DIFF`, never decoded. C2 is deferred and recorded in the spec as a known limit; C10 is a documented limit of `--share plugins`. The options below are kept as they were proposed.

- **C1 (item 17, Claude).** The §7.13 table puts "MCP logins" in `.claude.json`. They are in the credential store, which is still per account. Change the table cell to "logins and MCP OAuth logins (Keychain entry keyed to the folder, or `.credentials.json`); `.claude.json` (workspace trust, per-project state, user MCP servers)".
- **C2 (item 17, Codex).** In the default `auto` mode, Codex MCP OAuth logins are shared by every `CODEX_HOME` on the machine. §7.13 must say that account selection does not separate them. Owner decision, either: (a) document it, and have `alz doctor` print a static notice; or (b) also recommend `mcp_oauth_credentials_store = "file"` in each account's `config.toml`. alz never writes that file.
- **C3 (item 12, Claude).** Account selection separates only `/login` credentials. Anthropic profiles (`~/.config/anthropic`, `ANTHROPIC_PROFILE`), keyless Console sign-ins and credentials in environment variables are shared, and several outrank `/login`. Add this as a "Limit" in §7.13, and consider a doctor notice when `ANTHROPIC_API_KEY`, `ANTHROPIC_AUTH_TOKEN`, `CLAUDE_CODE_OAUTH_TOKEN` or `ANTHROPIC_PROFILE` is set (names only).
- **C4 (item 14, Claude).** Sharing `skills/` and `plugins/` as whole-folder links also shares `skills/synced/` and `plugins/synced/`. Each account's claude.ai sync then downloads its organisation's skills and plugins into the shared folder, where the other accounts load them, and removes the other accounts' synced items. Owner decision, either: link each entry and keep `synced` and `.trash` per account; stop sharing these two folders by default; or document `syncClaudeAiSkills: false`. Item 14 SMOKE confirms that writes go through a whole-folder link (plugin install wrote the shared `plugins/` and `settings.json`), and that per-entry skill links load, so the per-entry option works for skills. Per-entry links for `plugins/` were not tested: its contents are files Claude rewrites (`installed_plugins.json`, `known_marketplaces.json`), not a list of entries.
- **C5 (item 14, Codex).** A shared `skills/` link also shares `skills/.system/`, which each Codex version deletes and rewrites at start. SMOKE confirms the rewrite goes through the link into the shared folder. Accounts on different Codex versions would rewrite it at every switch. This is minor: document it, or link each entry (SMOKE: per-entry links load, and `.system` stays per account).
- **C6 (items 10 and 15, Claude).** `env.CLAUDE_CONFIG_DIR` in user settings (inside the chosen folder), `--settings` or managed settings overrides the value alz set, at least for subprocesses. alz cannot see this, because it never reads those files. Add this to §7.13 "Values set outside alz" as a limit. The §7.13 rule that layers cannot set the variables is still right: Claude already drops the variable from project settings, so that rule is defence in depth.
- **C7 (item 19).** Name the markers in §7.13: refuse when any of `CLAUDECODE`, `CLAUDE_CODE_CHILD_SESSION`, `AI_AGENT`, `CODEX_THREAD_ID`, `CODEX_SESSION_ID` or `CODEX_CI` is set. The Codex names are SOURCE only; O5 observes them. Neither tool marks stdio MCP servers in a way that covers both (Codex sets nothing there), so a refusal inside an MCP server is not guaranteed. Also state that `CLAUDECODE` is set in IDE integrated terminals, so `alz account allow` refuses there too, and that markers can be unset, which the "Limit" paragraph already covers.
- **C8 (item 18, record only).** Claude creates a missing folder with mode 0755. Codex refuses to start on one and creates nothing. Write both into §7.13 where it says "§12 item 18 records which". The `alz doctor` warning about an existing blocked folder matters for Claude only.
- **C9 (item 11, record only).** An empty `CODEX_HOME` counts as unset, and a set value is canonicalized. The shell hook should keep using `unset` for the default account; this needs no change.
- **C10 (item 14, Claude, new).** With `plugins/` shared, `installed_plugins.json` records each plugin's `installPath` under the folder of the account that installed it (for example `~/.claude-<name>/plugins/cache/…`), not under the shared real path. That works while that account's link exists. If the user removes the account folder that installed a plugin, other accounts' records may point at a missing path. The SMOKE used a `directory` marketplace, which loads from its source folder, so it did not show whether Claude loads from `installPath`. §7.13 should say: install plugins from the default account when `plugins/` is shared. Having `alz doctor` check `installPath` would mean reading a file in an account folder, which the invariant forbids (account folders are read for names only), so documenting it is the only option that fits the current rules. Owner decision; this is unverified for git and npm marketplaces.
- **C11 (item 20, new).** §7.13 may now name both tools, since §12 item 20 confirms both. Write the rule as: a variable counts as set by mise when the decoded `__MISE_DIFF` `new` map has it with the current value, and as set by direnv when the decoded `DIRENV_DIFF` `n` map does. Three points for the spec. (1) Both values are untrusted input from the environment and hold the values of every variable the tool set, secrets included. alz decodes them in memory with a size limit, reads only the keys it needs and compares one value, and never prints, logs or caches the decoded content. (2) Neither encoding is a stable interface. mise's is an internal MessagePack struct (its fields are `#[serde(default)]`), and direnv's is undocumented, so a decode failure means "unknown source", never an error, and the adapter's verified-version rule covers it. (3) Decoding mise's value needs a small MessagePack reader inside alz, because a new runtime dependency would break the `yaml` and `diff` rule. Owner decision: write that reader, or name only direnv and print a generic "set outside alz" warning for mise.
- **C12 (item 15, Codex, new).** (a) `shell_environment_policy.set`, `exclude`, `include_only` and `inherit` change or drop `CODEX_HOME`, `CLAUDE_CONFIG_DIR`, `HOME` and `XDG_*` for Codex shell commands. Unlike Claude's project `env`, this is allowed in a trusted project's `.codex/config.toml`. The §7.13 rule that layers cannot set these variables must name this key explicitly: the Codex adapter refuses a generated `shell_environment_policy` that names them. A hand-written project `.codex/config.toml` stays outside alz's control; add that as a limit. (b) Codex stdio MCP servers do not inherit `CODEX_HOME` or `CLAUDE_CONFIG_DIR` unless `env_vars` lists them, so an MCP server that runs `codex` or `claude` uses the default account. Add this to §7.13 next to the Claude inheritance rule.
- **C13 (item 21, new).** A Bash deny rule such as `bash:alz account allow*` is not a boundary in Claude: the docs say so, and `sh -c`, a full path or a script bypass it. §7.13 should treat that rule as a speed bump. The real guards remain the session-marker refusal (C7, which a child can unset) and keeping `allowed.yaml` outside the repository. Say so in the "Limit" paragraph. A `~/.config/agent-layerz/**` Read/Edit deny helps only together with the sandbox's `denyRead`/`denyWrite`, because scripts bypass the permission rules.

### 12.2 Owner-only checks (need a real login, a second account or Linux)

Run these in a throwaway folder; none of them reads a credential.

- **O1 (item 10, trust per account).** In `/tmp/alz-trust`, run `git init -q`, then `CLAUDE_CONFIG_DIR=~/.claude-<name> claude`, accept the trust dialog and `/exit`. Then run `claude` with the variable unset. Look for the trust dialog appearing again. `claude auth status --text`, run with and without the variable, should name two different accounts.
- **O2 (item 12, Linux).** On a Linux machine, `CLAUDE_CONFIG_DIR=$(mktemp -d) claude auth status --text` should report no login, and after a login in `~/.claude-<name>`, `ls -la ~/.claude-<name>/.credentials.json` should show `-rw-------`. For Codex, set `cli_auth_credentials_store = "keyring"` in two homes' `config.toml` and log in to only one. `CODEX_HOME=<the other> codex login status` should print `Not logged in`.
- **O3 (item 17, Codex shared MCP login).** Configure an OAuth MCP server with the same name and URL in `~/.codex/config.toml` and `~/.codex-<name>/config.toml`. Run `codex mcp login <server>` with the variable unset, then `CODEX_HOME=~/.codex-<name> codex mcp list`. If the second account shows the server as logged in, the login is shared, as SOURCE says.
- **O4 (item 10, read only at start).** Start `CLAUDE_CONFIG_DIR=~/.claude-<name> claude` in a scratch repo. From another terminal, add `"env": {"CLAUDE_CONFIG_DIR": "<tmp>/x"}` to `~/.claude-<name>/settings.json`. In the session run `! echo $CLAUDE_CONFIG_DIR`, send one prompt, and check whether new transcript lines still go to `~/.claude-<name>/projects/`. Remove the key afterwards.
- **O5 (items 15 and 19, Codex).** In a scratch repo: `codex exec "Run this shell command and print its output verbatim: env | cut -d= -f1 | sort"`. Look for `CODEX_*` names, and whether `CODEX_HOME` reaches the shell when it was set.
- **O6 (item 21).** In a scratch repo, write `.claude/settings.json` as `{"permissions":{"deny":["Bash(echo alz-deny-probe*)","Read(~/alz-deny-probe.txt)"]}}`. Ask Claude to run `echo alz-deny-probe`, `sh -c 'echo alz-deny-probe'`, `env echo alz-deny-probe` and `cat ~/alz-deny-probe.txt` (after creating that file). Note which are blocked. Repeat with the Codex rules mechanism once its syntax is recorded.
- **O7 (item 14, `sessions/` and `memories/`).** With `~/.codex-<name>/sessions` a link to the default folder's copy, run one `codex exec`, then `ls -la ~/.codex-<name>`: is the link kept, or replaced by a folder?
- **O8 (§7.13 'Limit').** Inside a Claude Code session with its sandbox on, and inside a Codex session with its sandbox, try to start a program outside the sandbox that could run `alz account allow` in a terminal the agent does not own: `tmux new-session -d 'alz account allow'` (or `tmux send-keys` to an existing tmux or screen socket), `open -a Terminal` and `open -a iTerm` (LaunchServices) and `osascript -e 'tell application "Terminal" to do script "alz account allow"'`. Note which are blocked. A launcher that works means an agent can open a real terminal, so the terminal and marker checks stop only a direct run (§7.13 'Limit').


## 13. Security-relevant native settings keys (TASK-76) — checked 2026-11-23

**Tools:** Claude Code **2.1.323**, Codex CLI **0.169.0** (source tag `rust-v0.169.0`). These are the keys of the one table in `crates/alz-core/src/item/native_keys.rs` (98 path rows: 60 Claude, 38 Codex), which R4.16 uses for floors and reserved keys. Adapters and `settings.yaml` rules may rely on a key only as recorded here.

### 13.0 Sources and method

- Claude, DOCS: `code.claude.com/docs/en/settings-reference.md` (every key has a Scope, Type and Default line) and `code.claude.com/docs/en/settings.md`, fetched 2026-11-23. **The page describes a Claude Code newer than 2.1.323**: it says the default of `sandbox.autoAllowBashIfSandboxed` changed in v2.1.324. Where the page and the installed tool differ, the row says so.
- Codex, DOCS: `codex-rs/core/config.schema.json` and `codex-rs/config/src/config_toml.rs` at tag `rust-v0.169.0` (`raw.githubusercontent.com/openai/codex/rust-v0.169.0/...`), and `learn.chatgpt.com/docs/config-file/config-reference`.
- Claude, SMOKE (type only): a settings file with a wrongly typed value does not load, and `claude -p hi` prints, before the login check, `These settings files did not load … (<key path>): Expected boolean, but received string`. The command needs no login. I wrote a project `.claude/settings.json` in a throwaway folder and ran `CLAUDE_CONFIG_DIR=<empty folder> claude -p hi --max-turns 1`, once with the number `12345` and once with the string `"x"` per key. A key that gives no error for the wrong value is not type-checked by 2.1.323 in a project file. An unknown key (`bogusKey`) gives no error.
- Codex, SMOKE (type, enum values, load errors): `CODEX_HOME=<empty folder>` with a `config.toml` holding one key, then `codex debug prompt-input hi`. A wrong type exits with `invalid type: integer …, expected a boolean` (or `expected struct …`); a wrong enum value lists the accepted variants. An unknown key (`bogus_key = 1`, and an unknown key inside `[sandbox_workspace_write]`) is **accepted without error**.
- SMOKE shows that the installed tool enforces the name and the type or values. It does not show the effect. Every **effect**, **default** and **scope** below is DOCS, because it needs a login, a model call or a sandbox run that I did not make. Nothing was written under `~/.claude*` or `~/.codex*`.

Level column: `S` = name and type or values SMOKE, effect DOCS; `D` = DOCS only (the tool does not check the key where I could run it).

### 13.1 Claude — sandbox switches (booleans)

| Key (under `claude:`) | Default | Scope | What it does | Stricter value | Level |
|---|---|---|---|---|---|
| `sandbox.enabled` | `false` | any file, with limits on project and local files | Sandboxes Bash commands. Without it commands run unsandboxed. | `true` | S |
| `sandbox.failIfUnavailable` | `false` | any file, with limits | `true`: Claude exits at startup when `enabled` is `true` but the sandbox cannot start. `false`: commands run unsandboxed. | `true` | S |
| `sandbox.autoAllowBashIfSandboxed` | `false` from v2.1.324; **`true` before that, so `true` in the installed 2.1.323** | any file | `true`: sandboxed Bash commands run without a permission prompt. `deny` rules and content-scoped `ask` rules still apply. | `false` | S |
| `sandbox.allowUnsandboxedCommands` | `true` | any file; a managed or `--settings` `false` makes the sandbox "admin-required" | `true`: Claude may retry a blocked command outside the sandbox with `dangerouslyDisableSandbox`. `false`: the parameter is ignored. | `false` | S |
| `sandbox.enableWeakerNestedSandbox` | `false` | any file, with limits | Linux in an unprivileged container: bind-mounts the existing `/proc`. The page says it reduces security. | `false` | S |
| `sandbox.enableWeakerNetworkIsolation` | `false` | any file, with limits | macOS: lets sandboxed commands reach `com.apple.trustd.agent`. The page says it opens a possible exfiltration path. | `false` | S |
| `sandbox.network.allowAllUnixSockets` | `false` | any file, with limits | `true`: every Unix socket (and `AF_VSOCK` on Linux) is reachable. | `false` | S |
| `sandbox.network.allowLocalBinding` | `false` | any file, with limits | macOS: listen on ports and connect to any localhost port. No effect on Linux or WSL2. | `false` | S |

### 13.2 Claude — sandbox lists and maps

The type of every list is an array. SMOKE: a string or a number is refused (`Expected array`). Claude merges list keys across the settings files it loads, so a later file can append.

| Key | Entries | Default | Scope | What it does | Level |
|---|---|---|---|---|---|
| `sandbox.excludedCommands` | command patterns (`Bash(...)` rule syntax) | unset | any file, with limits | Commands that run outside the sandbox. "Not a security boundary" (the page). | S |
| `sandbox.filesystem.allowWrite` | path strings | unset | any file, with limits | Extra writable paths. | S |
| `sandbox.filesystem.allowRead` | path strings | unset | any file, with limits | Re-opens reads inside a `denyRead` region. | S |
| `sandbox.filesystem.denyWrite` | path strings | unset | any file | Blocks writes, also inside writable folders. | S |
| `sandbox.filesystem.denyRead` | path strings | unset | any file | Blocks reads, for example of credential files. | S |
| `sandbox.ignoreViolations` | **an object** (a command substring maps to a list of violation substrings; `*` is every command), not a list | unset | any file, with limits | Silences violation reports. The sandbox still blocks the access. SMOKE: `Expected record`. | S |
| `sandbox.network.allowedDomains` | domains, `*.` wildcards, IP literals, optional `:port` | unset | any file, with limits; managed only when `allowManagedDomainsOnly` is set | Pre-allows outbound hosts. | S |
| `sandbox.network.deniedDomains` | same syntax | unset | any file | Blocks hosts; wins over `allowedDomains`. | S |
| `sandbox.network.allowUnixSockets` | socket paths | unset | any file, with limits | macOS only; ignored on Linux and WSL2. | S |
| `sandbox.network.allowMachLookup` | XPC or Mach service names; a trailing `*` is a prefix, `"*"` alone is all | unset | any file, with limits | macOS: services the sandbox may look up; the sandbox does not confine what they do. | S |

Order notes for the floors: `allowWrite`, `allowRead`, `excludedCommands`, `allowedDomains`, `allowUnixSockets`, `allowMachLookup` and `ignoreViolations` widen what a command may do or hide, so more entries is looser. `denyWrite`, `denyRead` and `deniedDomains` narrow it, so more entries is stricter. This matches the table.

### 13.3 Claude — keys for a user or managed scope, and other reserved keys

SMOKE for the type. The scope is DOCS; I could not observe it, except for `managedMcpServers`, where the tool says: `"managedMcpServers" is only honored from managed settings and was ignored here`.

| Key | Type | Default | Scope (DOCS) | Level |
|---|---|---|---|---|
| `sandbox.filesystem.disabled` | boolean; `true` skips filesystem isolation and keeps network isolation | `false` | user or managed | S |
| `sandbox.allowAppleEvents` | boolean; removes code-execution isolation on macOS | `false` | user or managed | S |
| `sandbox.allowNestedUserNamespaces` | boolean; Linux and WSL2; needs v2.1.305 or later; "turns off a security boundary" | `false` | user or managed; a `true` in a project file is ignored | S |
| `sandbox.network.strictAllowlist` | boolean; deny hosts outside the allowlist instead of prompting | `false` | user or managed | S |
| `sandbox.network.allowManagedDomainsOnly` | boolean; only managed domains count | `false` | managed | S |
| `sandbox.network.httpProxyPort`, `sandbox.network.socksProxyPort` | **a number** (a local TCP port). Your proxy then filters, and Claude's domain lists no longer apply to that traffic. SMOKE: a string is `Expected number` | unset | any file, unless other sandbox settings limit it | S |
| `sandbox.credentials` | object (`files`, `envVars`, `allowPlaintextInject`, `awsPairs`, `sigv4`); see also §12 item 16 | unset | any file, parts of it user or managed only | S |
| `apiKeyHelper` | string, a shell command line that prints the credential; run through `/bin/sh` | unset | any file | S |
| `autoMode` | object (`environment`, `allow`, `soft_deny`, `hard_deny`, `classifyAllShell`) | unset | user or managed | S |
| `skipAutoPermissionPrompt` | boolean | unset | user or managed | S |
| `skipDangerousModePermissionPrompt` | boolean; skips the confirmation before `bypassPermissions` | unset | user, local or managed | S |
| `allowManagedHooksOnly` | boolean | unset | managed | S |
| `allowManagedMcpServersOnly` | boolean | `false` | managed | S |
| `managedMcpServers` | object keyed by server name (`https://` `url` required) | unset | managed; dropped with a warning elsewhere | S (scope also SMOKE) |
| `allowAllClaudeAiMcps` | boolean; loads claude.ai connectors next to a deployed `managed-mcp.json` | `false` | **managed** | S |
| `prependPlugins`, `appendPlugins` | arrays of `plugin-name@marketplace-name`: managed plugins whose mods run before or after a user's | unset | user or managed; ignored in project and local files | D (2.1.323 gave no error for a wrong type) |
| `pluginConfigs` | object: plugin ID to `{ options, mcpServers }` | unset | user or managed | S |
| `strictKnownMarketplaces`, `blockedMarketplaces` | arrays of marketplace source objects | unset | managed | S |
| `strictPluginOnlyCustomization` | `true`, or an array from `"skills"`, `"agents"`, `"hooks"`, `"mcp"` | unset | managed | D (2.1.323 gave no error for a wrong type) |
| `syncClaudeAiPlugins` | boolean; `false` stops the download of plugins synced from the claude.ai account | unset | user, local or managed | S |

### 13.4 Claude — commands, hooks, skills, MCP, plugins, permissions

| Key | Type | Default | Scope | What it does | Level |
|---|---|---|---|---|---|
| `statusLine` | object: `type: "command"`, `command`, optional `padding`, `refreshInterval`, `hideVimModeIndicator` | unset | any file | Runs a command to draw the status line. Only the managed value runs when `allowManagedHooksOnly` is on or `disableAllHooks` is set outside managed settings. | S |
| `fileSuggestion` | object: `type: "command"`, `command` | unset | any file | Runs a command for `@` path completion. | S |
| `disableAllHooks` | boolean; also turns off the custom status line and file suggestion | unset | any file | See §4. | S |
| `hooks` | object keyed by hook event; the handler `type` is `command`, `prompt`, `agent`, `http` or `mcp_tool`. SMOKE: a non-object is ignored with a message | unset | any file | Hooks merge across files. | S |
| `allowedHttpHookUrls` | array of URL patterns, `*` wildcard | unset (any URL allowed) | any file; arrays merge | A covered HTTP hook runs only if its URL matches the merged list. | S |
| `httpHookAllowedEnvVars` | array of variable names | unset (each hook's own list applies) | any file; arrays merge | Limits which variables an HTTP hook may put into headers. | S |
| `disableSkillShellExecution` | boolean | unset | any file; a managed `true` cannot be undone | Replaces inline shell blocks in skills and commands with a notice. | S |
| `allowedMcpServers` | array of objects with **exactly one** key each: `serverName`, `serverCommand` (an argument array, matched exactly) or `serverUrl` (a URL pattern). SMOKE: a non-array is ignored with a message | unset (all allowed); an empty merged list blocks every server users add | any file; entries merge unless `allowManagedMcpServersOnly` | Allowlist of MCP servers. | S |
| `deniedMcpServers` | same entry shape | unset | any file; always merged, even under `allowManagedMcpServersOnly` | Blocks matching servers wherever they are defined. | S |
| `disableClaudeAiConnectors` | boolean; a `true` in any file applies, and a project `false` cannot undo a user or managed `true` | `false` | any file | Stops fetching and connecting the claude.ai connectors. | S |
| `enableAllProjectMcpServers` | boolean | unset | any file; in an untrusted folder the shared project file is ignored | Approves every server of the project `.mcp.json`. | S |
| `enabledMcpjsonServers`, `disabledMcpjsonServers` | array of server names of `.mcp.json` (the tool also accepted a bare string) | unset | any file; the same trust rule | Approve or reject named servers. | S |
| `mcpServers` | **not a documented settings key.** The settings reference has no `mcpServers` entry; servers live in `.mcp.json` and `~/.claude.json`. 2.1.323 accepts any value here without a message | n/a | n/a | Whether it has an effect in `settings.json` is unknown. | D |
| `enabledPlugins` | object: `plugin-name@marketplace-name` to boolean | unset (each plugin follows its `defaultEnabled`) | any file | Turns plugins on or off. SMOKE: `Expected record`. | S |
| `extraKnownMarketplaces` | object: marketplace name to `{ source, autoUpdate? }` | unset | any file; a project entry is honoured only after the workspace trust dialog | Registers marketplaces for everyone who opens the repository. | S |
| `model` | string, a model alias or a full model ID | unset | any file | The model every new session uses. | S |
| `disableAutoMode` | the string `"disable"` only; SMOKE: any other value is `Invalid value. Expected one of: "disable"`. Also accepted as `permissions.disableAutoMode` | unset | any file | Removes auto mode from the cycle; a session that would start in it starts in `default`. | S |

Keys of the `permissions` block. `claude.permissions` is refused as a whole (§7.9); these are recorded because the owner made three of them first-class floor fields of `permissions.yaml`.

| Key | Type | Scope | Level |
|---|---|---|---|
| `permissions.disableBypassPermissionsMode` | **the string `"disable"`, not a boolean** (SMOKE). Claude then rejects `--dangerously-skip-permissions` and ignores an agent's `permissionMode: bypassPermissions` | any file | S |
| `permissions.blockReadsOutsideWorkingDirectories` | boolean. Read, Grep, Glob and LSP refuse paths outside the working directories in every mode. A `true` in any file applies; `false` equals unset | any file | S |
| `permissions.blockWritesOutsideWorkingDirectories` | boolean. Write, Edit and NotebookEdit refuse such paths; shell commands, MCP tools and hooks are not limited. Same `true` rule | any file | S |
| `permissions.denyInsteadOfPrompting` | boolean; needs v2.1.319 or later | user, managed or `--settings` | S |
| `permissions.defaultMode` | one of `acceptEdits`, `auto`, `bypassPermissions`, `default`, `dontAsk`, `plan` (SMOKE; the page adds `manual` as an alias of `default`). `auto` and `bypassPermissions` have no effect from project or local files | any file | S |
| `permissions.additionalDirectories` | array of directory paths | any file | S |

### 13.5 Codex — keys that duplicate first-class items

Name and type are SMOKE (`CODEX_HOME=<empty folder>`); the effect is DOCS. §6.4 to §6.6 hold the rest.

| Key | Type and values | Notes |
|---|---|---|
| `model` (top, `profiles.<p>.model`) | string | |
| `mcp_servers.<id>` | table (`command`, `args`, `url`, `enabled_tools`, `disabled_tools`, `default_tools_approval_mode`, …) | §6.5 |
| `hooks` | table keyed by event (`PreToolUse`, `PostToolUse`, `SessionStart`, `Stop`, …) plus `state` | §6.4 |
| `features.hooks`, `features.codex_hooks` (also under `profiles.<p>.features`) | boolean | |
| `approval_policy` (top and `profiles.<p>`) | The schema lists `on-request`, `never` and a `granular` table. The parser names also `untrusted` and `on-failure`. **`approval_policy = "untrusted"` fails at load: `is no longer supported; remove this setting`** (SMOKE); `on-failure` loads | §6.6 |
| `sandbox_mode` (top and `profiles.<p>`) | `read-only`, `workspace-write`, `danger-full-access` (SMOKE) | |
| `profile` | string. **0.169.0 refuses it at load: ``legacy `profile = "p"` config is no longer supported; use `--profile p` with `p.config.toml` instead``** (SMOKE). The `profiles` table is still parsed (accepted, SMOKE); whether `profiles.<p>.*` has an effect is UNVERIFIED | the docs say a project-local `.codex/config.toml` ignores `profile` and `profiles` |

### 13.6 Codex — floors

Name and type or values are SMOKE; the effect is DOCS. A wrong type is refused at load.

| Key | Type and values | Default | Stricter | What it does |
|---|---|---|---|---|
| `sandbox_workspace_write.network_access` | boolean | `false` | `false` | Outbound network inside the `workspace-write` sandbox. |
| `sandbox_workspace_write.writable_roots` | array of absolute paths | `[]` | fewer | Extra writable roots when `sandbox_mode = "workspace-write"`. |
| `sandbox_workspace_write.exclude_tmpdir_env_var` | boolean | `false` | `true` | Removes `$TMPDIR` from the writable roots. |
| `sandbox_workspace_write.exclude_slash_tmp` | boolean | `false` | `true` | Removes `/tmp` from the writable roots. |
| `approvals_reviewer` (top, `profiles.<p>`, `apps.<id>`) | `user`, `auto_review`, `guardian_subagent` (a legacy alias; SMOKE lists all three). The reference page lists `user \| auto_review` | `user` | `user` | Who reviews eligible approval prompts; `auto_review` uses a reviewer subagent. It does not change sandboxing. |
| `allow_login_shell` | boolean | `true` | `false` | `false`: a request for a login shell is rejected. |
| `web_search` (top, `profiles.<p>`) | `disabled`, `cached`, `indexed`, `live` (SMOKE) | `cached`; `live` with `--yolo` or a full-access sandbox | `disabled`, then `cached`, `indexed`, `live` | `cached`: an OpenAI-maintained index, no external web access. `indexed`: external access only when gated by the index. `live`: unrestricted. |

### 13.7 Codex — reserved

| Key | Type and values | Level |
|---|---|---|
| `auto_review` | table (`policy`, `timeout_ms`, `circuit_break_action`, …); `policy` replaces the reviewer's default policy text | S |
| `default_permissions` | string. `:read-only`, `:workspace` and `:danger-full-access` are built in; other names need a `[permissions.<name>]` table. SMOKE: **it fails at load with `requires a [permissions] table` without one.** Do not combine it with `sandbox_mode` or `[sandbox_workspace_write]` (DOCS) | S |
| `permissions` | table of named permission profiles | S |
| `projects.<path>.trust_level` | `trusted` or `untrusted` (SMOKE). An untrusted project skips project-scoped `.codex/` layers (DOCS) | S |
| `plugins.<plugin>@<marketplace>`, `plugins._default` | table: `enabled`, `mcp_servers`, `apps`, … | S |
| `marketplaces.<name>` | table: `source` (a git repository or an absolute local path), `source_type` (`git` or `local`), `ref`, `sparse_paths` | S |
| `apps.<id>`, `apps._default` | table; `default_tools_approval_mode` is `auto`, `prompt`, `writes` or `approve`; `approvals_reviewer` is also allowed | S |
| `windows.sandbox` | `elevated`, `unelevated`, `mxc` (SMOKE). The schema also has `windows.allow_mxc` (boolean; `false` blocks both explicit and automatic MXC selection) | S |
| `notify` | array of strings, a command to run | S |
| `model_provider` (top, `profiles.<p>`) | string | S |
| `model_providers.<id>` | table (`name`, `base_url`, `env_key`, `auth`, …) | S |
| `openai_base_url`; `chatgpt_base_url` (also in `profiles.<p>`) | string | S |
| `otel` | table (`exporter`, `trace_exporter`, `metrics_exporter`, `log_user_prompt`, …) | S |

DOCS: Codex ignores `openai_base_url`, `chatgpt_base_url`, `apps_mcp_product_sku`, `model_provider`, `model_providers`, `notify`, `profile`, `profiles`, `experimental_realtime_ws_base_url` and `otel` when they stand in a project-local `.codex/config.toml`. So refusing them in `settings.yaml` costs nothing: they would do nothing there.

### 13.8 Corrections the code table needs

Found by TASK-76. Items 1 to 7 are applied in `crates/alz-core/src/item/native_keys.rs` and §7.9 of the spec (TASK-76, AC #2); a Codex `profiles` table is reserved whole, so the rows below it (`profiles.*.…`) are folded into that one row.

1. **`codex.tools.web_search` is not a mode.** It is a boolean or a table `{ context_size = "low|medium|high", allowed_domains = [string], location = {…} }` (SMOKE: an integer gives `data did not match any variant of untagged enum WebSearchToolConfigInput`; the source ignores a boolean). The row `codex("tools.web_search", WEB_SEARCH)` makes every table a `Kind` error against a text ladder. Remove it. Replace it with `tools.web_search.allowed_domains` as `NO_ADDITIONS` and `profiles.*.tools.web_search.allowed_domains` likewise. The page says these domain filters are separate from the sandbox's network rules and do not restrict connectors or MCP servers. `context_size` and `location` need no row. That an unset `allowed_domains` means "unrestricted" is implied by the docs, not observed.
2. **`claude.allowAllClaudeAiMcps` is managed only**, so give it `OUTSIDE_THE_PROJECT`, not the reason "it widens the connector servers past the allowlist of mcp.yaml". The refusal stays.
3. **`claude.mcpServers` is not a settings key** in the reference. Keep the refusal (the key is accepted without a message), but the spec and the row should not call it a key that Claude reads from settings. For agent frontmatter `mcpServers` is real (§1), so the `AGENT_KEYS` row stands.
4. **`codex.profile`**: 0.169.0 refuses the key at load, and project config ignores `profile` and `profiles`. Keep the refusal. The row names `permissions.yaml` as the place that sets it; the reason is "removed by Codex", and the profile rows (`profiles.*.approval_policy`, `profiles.*.sandbox_mode`, `profiles.*.features.*`) carry the same caveat.
5. **Keys of a profile that the table does not cover**: `profiles.*.model_provider`, `profiles.*.chatgpt_base_url` and `profiles.*.windows.sandbox` exist in the schema (SMOKE: accepted) and are not refused. Either add them as reserved rows or reserve `profiles` as a whole (the docs say project config ignores it). Recommended: reserve `profiles` whole; it covers the `profiles.*.…` rows too.
6. **Keys that run a command and are not in the table**, all "any file" scope, type string (SMOKE): `awsAuthRefresh`, `awsCredentialExport`, `gcpAuthRefresh`, `otelHeadersHelper` (an executable path or a shell command line). Recommended: `RUNS_A_COMMAND`, like `statusLine`.
7. **Managed- or user-only keys that are not in the table** (DOCS, from the settings index; type not probed): `sandbox.filesystem.allowManagedReadPathsOnly`, `sandbox.ripgrep`, `sandbox.bwrapPath`, `sandbox.socatPath`, `sandbox.network.tlsTerminate`, `processWrapper`, `policyHelper`, `sshHostCommands`, `forceLoginGatewayUrl`, `allowManagedPermissionRulesOnly`, `disableSideloadFlags`. Recommended: `OUTSIDE_THE_PROJECT`. For Codex: `windows` as a whole.
8. **`permissions.yaml` fields (owner answer 2)**: `disableBypassPermissionsMode` must be written as the string `"disable"`, never `true`; `blockReadsOutsideWorkingDirectories` and `blockWritesOutsideWorkingDirectories` are booleans, and `false` equals unset. `disableAutoMode` is the string `"disable"` too.
9. **A wrongly typed value costs the file.** For most keys Claude drops the **whole settings file** (`nothing in them is in effect, including hooks and permission rules`). For `hooks`, `allowedMcpServers`, `deniedMcpServers`, `extraKnownMarketplaces` and `managedMcpServers` it ignores that field only and says so. A value that alz writes into `.claude/settings.json` must have the exact type above.
10. **Defaults that matter for the "stricter" order**: `sandbox.autoAllowBashIfSandboxed` is `true` by default in the installed 2.1.323, so a layer that leaves it unset leaves the prompts off. `sandbox.allowUnsandboxedCommands` is `true` by default.

Counts. Of the 98 rows, all 98 were checked against DOCS; 94 also have a SMOKE type check; 4 are DOCS only (`claude.mcpServers`, `prependPlugins`, `appendPlugins`, `strictPluginOnlyCustomization`, which 2.1.323 does not type-check in a project file). One row is wrong in kind (correction 1), one carries a wrong reason (2), and two need a changed note (3 and 4); corrections 5 to 7 are keys the table lacks. No key is absent from the sources. Every other row matches in name, type and direction of the order.

## 14. Claude settings keys and subagent frontmatter fields with a type (TASK-79) — checked 2026-11-24

**Tools:** Claude Code **2.1.323** (`claude --version`). These are the keys of the typed table in `crates/alz-core/src/item/native_types/claude.rs`, which spec §7.9 and §7.2 use: under `claude:`, `settings.yaml` and an agent's native fields accept only these keys, and check the type of each value. A key may be accepted only if it is recorded here with its name and type; a unit test reads this section and fails for a table key that is not recorded in it.

### 14.0 Sources and method

- DOCS: `code.claude.com/docs/en/settings-reference.md` (every key has a Scope and a Type line), fetched 2026-11-24, and `code.claude.com/docs/en/sub-agents.md` (the frontmatter table), fetched 2026-11-24. The settings page describes a Claude Code newer than 2.1.323; where a key is new, the installed tool does not check it (level D).
- SMOKE (type, and enum values where it checks them): a project `.claude/settings.json` in a throwaway folder, with `CLAUDE_CONFIG_DIR` and `HOME` set to empty throwaway folders, then `claude -p hi --max-turns 1`, which needs no login for this check. For each key, one value of the documented type must load, and a number and a string (or a list, an object or a wrong enum value, whichever the key's type is not) must give `These settings files did not load … Expected …`, or, for the keys that Claude ignores field by field, a message that says the entry was ignored. Nothing was written under `~/.claude*`.
- What SMOKE found beyond the types: an object key that Claude does not know (`sandbox.bogusKey`, `spinnerTipsOverride.bogus`, `sandbox.network.bogus`) and an unknown top-level key load without a message. So Claude does not enforce the closed shape that alz asks for; the owner decision of 2026-11-23 (TASK-79) does. A `null` value (`"verbose": null`) does not load, which matches the rule that `null` is a value of the wrong type. `spinnerVerbs` needs both `verbs` and `mode` (an object with one, or none, does not load).
- Level: `S` = name and type SMOKE (the installed tool refuses a wrong type); `D` = DOCS only, because 2.1.323 loads a wrong type or wrong value without a message (the key is newer than the tool, or the tool does not check it in a project file). The effect and the default of every key are DOCS and are not needed by alz, which only passes the value on.

### 14.1 What the table checks

A type is a boolean, text, a whole number (a YAML integer), a number, one of a list of texts, a list of one type, a mapping with any text keys and one value type (`*` in a path), a mapping with listed keys (each optional unless the note says required; any other key is an error), a mapping with exactly one of the listed keys, or one of several types. A path has `*` for a key of a mapping with any keys and `[]` for a list item. Ranges ("minimum 1", "at most 1"), patterns and the syntax of a path or a URL are not checked.

A key that the security table refuses (§13) is not listed here, even when it has a type; a floor key of §13 is listed and typed.

### 14.2 Claude settings keys that a project file may carry

Keys with a floor (§13) first.

| Key (under `claude:`) | Type | Level | Note |
|---|---|---|---|
| `disableAutoMode` | one of disable | S |  |
| `sandbox` | a mapping | S |  |
| `sandbox.enabled` | a boolean | S |  |
| `sandbox.failIfUnavailable` | a boolean | S |  |
| `sandbox.autoAllowBashIfSandboxed` | a boolean | S |  |
| `sandbox.excludedCommands` | a list whose items are text | S |  |
| `sandbox.allowUnsandboxedCommands` | a boolean | S |  |
| `sandbox.filesystem` | a mapping | S |  |
| `sandbox.filesystem.allowWrite` | a list whose items are text | S |  |
| `sandbox.filesystem.denyWrite` | a list whose items are text | S |  |
| `sandbox.filesystem.denyRead` | a list whose items are text | S |  |
| `sandbox.filesystem.allowRead` | a list whose items are text | S |  |
| `sandbox.ignoreViolations` | a mapping whose values are a list whose items are text | S | a command substring maps to a list of violation substrings |
| `sandbox.enableWeakerNestedSandbox` | a boolean | S |  |
| `sandbox.enableWeakerNetworkIsolation` | a boolean | S |  |
| `sandbox.network` | a mapping | S |  |
| `sandbox.network.allowUnixSockets` | a list whose items are text | S |  |
| `sandbox.network.allowAllUnixSockets` | a boolean | S |  |
| `sandbox.network.allowLocalBinding` | a boolean | S |  |
| `sandbox.network.allowMachLookup` | a list whose items are text | S |  |
| `sandbox.network.allowedDomains` | a list whose items are text | S |  |
| `sandbox.network.deniedDomains` | a list whose items are text | S |  |
| `allowedHttpHookUrls` | a list whose items are text | S |  |
| `httpHookAllowedEnvVars` | a list whose items are text | S |  |
| `disableSkillShellExecution` | a boolean | S |  |
| `allowedMcpServers` | a list whose items are a mapping with exactly one of the keys serverName, serverCommand, serverUrl | S |  |
| `allowedMcpServers[].serverName` | text | D |  |
| `allowedMcpServers[].serverCommand` | a list whose items are text | D |  |
| `allowedMcpServers[].serverUrl` | text | D |  |
| `deniedMcpServers` | a list whose items are a mapping with exactly one of the keys serverName, serverCommand, serverUrl | S |  |
| `deniedMcpServers[].serverName` | text | D |  |
| `deniedMcpServers[].serverCommand` | a list whose items are text | D |  |
| `deniedMcpServers[].serverUrl` | text | D |  |
| `disableClaudeAiConnectors` | a boolean | S |  |

All other accepted keys.

| Key (under `claude:`) | Type | Level | Note |
|---|---|---|---|
| `$schema` | text | S |  |
| `advisorModel` | text | S |  |
| `alwaysThinkingEnabled` | a boolean | S |  |
| `availableModels` | a list whose items are text | S |  |
| `effortLevel` | one of low, medium, high, xhigh | D |  |
| `enforceAvailableModels` | a boolean | S |  |
| `fallbackModel` | a list whose items are text | S |  |
| `fastMode` | a boolean | S |  |
| `fastModePerSessionOptIn` | a boolean | S |  |
| `language` | text | S |  |
| `maxEffortLevel` | one of low, medium, high, xhigh, max | D |  |
| `modelOverrides` | a mapping whose values are text | S |  |
| `modelSettings` | a mapping whose values are a mapping | D |  |
| `modelSettings.*.effortLevel` | one of low, medium, high, xhigh | D |  |
| `modelSettings.*.maxEffortLevel` | one of low, medium, high, xhigh, max | D |  |
| `modelSettings.*.autoCompactWindow` | a whole number, or one of auto | D |  |
| `outputStyle` | text | S |  |
| `promptCacheTtl` | one of 5m, 1h | D |  |
| `showThinkingSummaries` | a boolean | S |  |
| `subagentPromptCacheTtl` | one of 5m, 1h | D |  |
| `switchModelsOnFlag` | a boolean | S |  |
| `ultracode` | a boolean | D |  |
| `autoCompactEnabled` | a boolean | S |  |
| `autoCompactWindow` | a whole number | D | 100000 or more (DOCS); the range is not checked |
| `autoMemoryDirectory` | text | S |  |
| `autoMemoryEnabled` | a boolean | S |  |
| `bashOutputMaxChars` | a whole number | D | positive; Claude clamps it to 4000 to 128000 (DOCS) |
| `claudeMdExcludes` | a list whose items are text | S |  |
| `env` | a mapping whose values are text | D | SMOKE: a number as a value is accepted by 2.1.323; alz requires text |
| `fileCheckpointingEnabled` | a boolean | S |  |
| `idleCompaction` | a boolean | S |  |
| `plansDirectory` | text | S |  |
| `skillListingBudgetFraction` | a number | S | above 0 and at most 1 (DOCS); the range is not checked |
| `skillListingMaxDescChars` | a whole number | S |  |
| `autoScrollEnabled` | a boolean | S |  |
| `companyAnnouncements` | a list whose items are text | S |  |
| `defaultShell` | one of bash, powershell | S |  |
| `editorMode` | one of normal, vim | D |  |
| `emojiCompletionEnabled` | a boolean | S |  |
| `keybindingFlavor` | one of classic, readline | D |  |
| `maxProseWidth` | a whole number | D | minimum 40 (DOCS); the range is not checked |
| `prefersReducedMotion` | a boolean | S |  |
| `promptSuggestionEnabled` | a boolean | S |  |
| `respectGitignore` | a boolean | S |  |
| `respondToBashCommands` | a boolean | S |  |
| `showClearContextOnPlanAccept` | a boolean | S |  |
| `showTurnDuration` | a boolean | S |  |
| `spinnerTipsEnabled` | a boolean | S |  |
| `spinnerTipsOverride` | a mapping | D |  |
| `spinnerTipsOverride.tips` | a list whose items are text, or a mapping | D |  |
| `spinnerTipsOverride.tips[].id` | text | D |  |
| `spinnerTipsOverride.tips[].text` | text | D |  |
| `spinnerTipsOverride.tips[].cooldownSessions` | a whole number | D |  |
| `spinnerTipsOverride.tips[].priority` | a whole number | D |  |
| `spinnerTipsOverride.tipsFile` | text | D |  |
| `spinnerTipsOverride.label` | text | D |  |
| `spinnerTipsOverride.excludeDefault` | a boolean | D |  |
| `spinnerVerbs` | a mapping | S | `verbs` and `mode` are both required (SMOKE: a file with only one does not load) |
| `spinnerVerbs.verbs` | a list whose items are text | S |  |
| `spinnerVerbs.mode` | one of append, replace | S |  |
| `syntaxHighlightingDisabled` | a boolean | S |  |
| `terminalProgressBarEnabled` | a boolean | S |  |
| `terminalTitleFromRename` | a boolean | S |  |
| `theme` | text | D | the documented values and `custom:<slug>`; only the type is checked |
| `timeFormat` | one of auto, 12-hour, 24-hour, 24-hour-utc | S (values D) |  |
| `timeZone` | text | S | an IANA name; only the type is checked |
| `tui` | one of default, fullscreen | S |  |
| `verbose` | a boolean | S |  |
| `viewMode` | one of default, verbose, focus | D |  |
| `voice` | a mapping | S |  |
| `voice.enabled` | a boolean | S |  |
| `voice.autoSubmit` | a boolean | S |  |
| `voice.mode` | one of hold, tap | S |  |
| `voiceEnabled` | a boolean | S |  |
| `wheelScrollAccelerationEnabled` | a boolean | S |  |
| `attribution` | a mapping, or the boolean false | S | an object, or `false` (needs v2.1.281 or later, DOCS) |
| `attribution.commit` | text | S |  |
| `attribution.pr` | text | S |  |
| `attribution.sessionUrl` | a boolean | D |  |
| `includeCoAuthoredBy` | a boolean | S |  |
| `includeGitInstructions` | a boolean | S |  |
| `prUrlTemplate` | text | S |  |
| `disableWorkflows` | a boolean | S |  |
| `enableWorkflows` | a boolean | S |  |
| `workflowKeywordTriggerEnabled` | a boolean | S |  |
| `workflowSizeGuideline` | one of unrestricted, small, medium, large | S |  |
| `disableBundledSkills` | a boolean | S |  |
| `skillOverrides` | a mapping whose values are one of on, name-only, user-invocable-only, off | S |  |
| `agent` | text | S |  |
| `crossSessionInbound` | one of accept, hold, refuse | S |  |
| `disableAgentView` | a boolean | S |  |
| `isolatePeerMachines` | a boolean | S |  |
| `teammateMode` | one of in-process, auto, tmux, iterm2 | D |  |
| `worktree` | a mapping | S |  |
| `worktree.baseRef` | one of fresh, head | S |  |
| `worktree.symlinkDirectories` | a list whose items are text | S |  |
| `worktree.sparsePaths` | a list whose items are text | S |  |
| `worktree.bgIsolation` | one of worktree, none | D |  |
| `agentPushNotifEnabled` | a boolean | S |  |
| `awaySummaryEnabled` | a boolean | S |  |
| `disableArtifact` | a boolean | S |  |
| `disableDeepLinkRegistration` | one of disable | S |  |
| `disableRemoteControl` | a boolean | S |  |
| `enableArtifact` | a boolean | S |  |
| `inputNeededNotifEnabled` | a boolean | S |  |
| `preferredNotifChannel` | one of auto, terminal_bell, iterm2, iterm2_with_bell, kitty, ghostty, notifications_disabled | D |  |
| `remote` | a mapping | S |  |
| `remote.defaultEnvironmentId` | text | S |  |
| `remoteControlAtStartup` | a boolean | S |  |
| `autoUpdatesChannel` | one of latest, stable | S |  |
| `minimumVersion` | text | S |  |
| `cleanupPeriodDays` | a whole number | S | minimum 1 (DOCS); the range is not checked |
| `feedbackSurveyRate` | a number | S | between 0 and 1 (DOCS); the range is not checked |

### 14.3 Keys that are not accepted

Not accepted means a `settings` error `is not a Claude Code settings key that alz accepts`.

- **Read from a user or managed scope only** (Scope column of the settings reference, DOCS): `askUserQuestionTimeout`, `availableModelsMatch`, `deniedModels`, `modelPicker`, `modelPricing`, `useAutoModeDuringPlan`, `autoMode.classifyAllShell`, `dialogExpiry`, `footerLinksRegexes`, `spellcheck`, `vimInsertModeRemaps`, `axScreenReader`, `bashEditDiffEnabled`, `autoContinueAtUsageLimit`, `claudeMd`, `worktree.location`, `sshConfigs`, `sshHostAllowlist`, `allowedProviders`, `gatewayInternalNetworks`, `enforceAnthropicSignIn`, `requiredMaximumVersion`, `requiredMinimumVersion`, `browserExternalPageTools`, `disableBrowserExternalNavigation`, `disableMobileSimulatorTools`, `desktopSessionCleanupPeriodDays`, `feedbackDrafts`, `forceRemoteSettingsRefresh`, `managedSourcesBehavior`, `parentSettingsBehavior`, `wslInheritsWindowsSettings`, `allowedChannelPlugins`, `channelsEnabled`, `disableCommandPluginSources`, `pluginSuggestionMarketplaces`, `pluginTrustMessage`, `allowClaudeInChromeWithManagedMcp`, `disableDesktopLocalSessions`, `disableRemotePreviewScripts` and `syncClaudeAiSkills` (user, local or managed). The security table also lists the keys of §13.3 and §13.8 item 7.
- **Global config keys**, which Claude Code keeps in `~/.claude.json` and not in a settings file: `autoConnectIde`, `autoInstallIdeExtension`, `claudeInChromeDefaultEnabled`, `copyFullResponse`, `copyOnSelect`, `defaultToAgentsView`, `diffTool`, `externalEditorContext`, `leftArrowOpensAgents`, `permissionExplainerEnabled`, `prStatusFooterEnabled`, `teammateDefaultModel`.
- **Any file, left out until the owner decides** (they lower a safety check or steer the sign-in, so they may need a floor or a reason in the security table first): `skipWebFetchPreflight` (skips the WebFetch domain safety check), `forceLoginMethod` and `forceLoginOrgUUID`.
- **Refused by the security table**: the first-class, reserved and floor-by-reason keys of §13 (`model`, `permissions`, `hooks`, `statusLine`, `enabledPlugins`, and the others).
- **Not a documented key**: any key that the settings reference does not list.

### 14.4 Subagent frontmatter fields under `claude:` in an agent file

DOCS only: `code.claude.com/docs/en/sub-agents.md`, "Supported frontmatter fields", fetched 2026-11-24. A subagent cannot be loaded without a login, so no SMOKE was possible; §1 records that Claude Code ignores an unknown field without a message. `name`, `description`, `tools`, `model`, `permissionMode`, `skills`, `isolation` and `memory` have a neutral field, and `mcpServers` and `hooks` come only through `mcp.yaml` and `hooks.yaml` (§1, spec §7.2), so none of them is in this table.

| Field (under `claude:`) | Type | Level | Note |
|---|---|---|---|
| `disallowedTools` | text, or a list whose items are text | D | a comma-separated string or a YAML list, "same format as `tools`" |
| `maxTurns` | a whole number | D | maximum number of agentic turns |
| `autoCompactWindow` | a whole number | D | 100000 or more; needs v2.1.296 or later; the range is not checked |
| `background` | a boolean | D |  |
| `omitClaudeMd` | a boolean | D | needs v2.1.271 or later |
| `effort` | one of low, medium, high, xhigh, max | D | the levels available depend on the model |
| `color` | one of red, blue, green, yellow, purple, orange, pink, cyan | D |  |
| `initialPrompt` | text | D | auto-submitted as the first turn when the agent runs as the main session agent |
| `experimental` | a mapping | D | a mapping of experimental options; docs name only `cacheTtl` |
| `experimental.cacheTtl` | one of 5m, 1h | D | `5m` or `1h`; needs v2.1.248 or later |

### 14.5 Counts

Settings: 147 rows (every key, and every key below an object, a map or a list), checked against the settings reference, of which 111 have a SMOKE type check and 36 are DOCS only (the installed 2.1.323 loads a wrong type without a message, or the row is a key below a map or list that the probe did not reach). Agent frontmatter: 10 rows, all DOCS. The security table of §13 keeps the keys it refuses.
