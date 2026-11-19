# target-facts.md — verified facts about Claude Code and Codex

**Date:** 2026-11-19
**Spec checked:** `docs/LAYERING-SPEC.md` Draft v1, §12 only (items 8–9 added later the same day: §7c)
**Tools:** Claude Code **2.1.311** (`/opt/homebrew/bin/claude`), Codex CLI **0.169.0** (`/opt/homebrew/bin/codex`)
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
- Scratchpad fixtures: `/private/tmp/claude-501/-Users-gautamr-dev-projects-agent-layers/da2fcdc2-d9c9-4d3f-9b05-272ea677fb99/scratchpad/{repo2,repo3a,repo3b,repo3c,repo4,parent,cx}`.

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
