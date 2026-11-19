# LAYERING-SPEC.md — Layered agent configuration for any client and project

**Status:** Draft v1 · 2026-11-19
**Audience:** a coding agent (Claude Code / Codex) implementing the tool, and humans who use it
**Name:** `agent-layerz`, with the CLI command `alz`

This spec describes a **tool-agnostic layering system** for coding-agent configuration:
instructions, agents, skills, commands, hooks, permissions, MCP servers and model routing.
Layers are written once and composed into each repository. It works with any number of
clients and projects, any account setup, and both Claude Code and Codex. Other targets can
be added later.

> **Implementer:** read the whole spec before writing code. Section 12 lists facts to check
> against the *installed* versions of Claude Code and Codex before implementing the
> adapters. Never hardcode a frontmatter field or config key from this document without
> checking it first.

---

## 1. Problem

- Agent configuration is spread over many places: `~/.claude`, `~/.codex`, per-account
  config dirs, `CLAUDE.md`, `AGENTS.md`, `.claude/`, `.codex/`, `.agents/`, plugins.
- Each agent tool resolves name conflicts differently. In Claude Code, for example, a
  **project** subagent beats a user one, but a **personal** skill beats a project one. So a
  layered setup that relies on native precedence behaves inconsistently.
- Personal standards, client context and repo specifics are duplicated and drift apart.
- Per-account config dirs (one per client or subscription) fragment the user-level config,
  so switching accounts changes agent behaviour.

## 2. Goals and non-goals

**Goals**
1. Write each piece of configuration **once**, in the narrowest layer where it applies.
2. Use **deterministic, documented merge rules** that are the same for every target tool.
3. Make agent behaviour **independent of which account is active**.
4. Work for **any client/project shape**: single repo, multi-repo, monorepo, nested groups.
5. Keep **public-safe layers** (e.g. personal, open-source) free of private content, and
   enforce that by checks.
6. Make results **reproducible** (pinned layer versions) and **reviewable** (generated output
   shows up in diffs).
7. Support **multiple targets**: Claude Code and Codex in v1, more through adapters.

**Non-goals (v1)**
- Running agents or orchestrating them. That is the orchestrator's job.
- Managing credentials or accounts. That is the account tool's job (e.g. a per-folder
  account switcher). This tool must never read them.
- Replacing a tool's native plugin system. It can coexist with one (see §11).

---

## 3. Concepts

| Term | Meaning |
|---|---|
| **Layer** | A versioned directory of config sources, usually its own git repo. Has a name, a visibility and a position in a stack. |
| **Stack** | An ordered list of layers applied to one repo, from most general to most specific. |
| **Target** | An agent tool whose native files we generate (`claude`, `codex`, later `gemini`, `opencode`, …). |
| **Item** | One unit of config: instruction block, agent, skill, hook, permission rule, MCP server, model route, setting. |
| **Overlay** | A partial item in a more specific layer that extends or changes an item from a more general layer. |
| **Slot** | A skill name that a general layer *references* and that more specific layers are expected to *fill* (e.g. `stack-conventions`). |
| **Build** | Resolving a stack and writing target-native files into the repo. |
| **Lock** | A file that pins the exact revision of every layer used in the last build. |

### 3.1 Layer kinds (convention, not hard-coded)

The tool supports **any number** of layers. The usual shape is:

```
personal   (public)   — your universal standards, generic agent personas, generic skills
org/client (private)  — client domain, constraints, compliance, shared stack choices
group      (private)  — optional: a product line or set of related repos within a client
repo       (private)  — repo-specific stack, commands, paths, contract ownership
```

Each layer declares a `visibility`: `public`, `private` or `confidential`. A layer may only
`extend` layers whose visibility is the **same or more public** (a public layer can't
extend a private one). Build output inherits the **most private** visibility in the stack.

---

## 4. File layout

### 4.1 Layer source layout (identical for every layer)

```
<layer-root>/
├── layer.yaml                 # layer metadata (§5.1)
├── instructions/
│   ├── 00-core.md             # concatenated in filename order
│   └── 50-<topic>.md
├── agents/
│   ├── <name>.md              # full agent definition (base)
│   └── <name>.overlay.md      # overlay on an agent from a more general layer
├── skills/
│   └── <name>/SKILL.md        # Agent Skills standard; may include scripts/, references/
├── commands/                  # optional; compiled to skills (see §7.4)
│   └── <name>.md
├── rules/                     # path-scoped rules (§7.10)
│   └── <name>.md
├── hooks/
│   ├── hooks.yaml             # declarative hook list (§7.5)
│   └── scripts/…              # hook scripts, referenced relatively
├── permissions.yaml           # allow / ask / deny (§7.6)
├── mcp.yaml                   # MCP server definitions (§7.7)
├── models.yaml                # role → model routing (§7.8)
├── settings.yaml              # other target settings, namespaced by target (§7.9)
└── targets/                   # optional raw passthrough per target (escape hatch)
    ├── claude/…
    └── codex/…
```

Every directory is optional. A layer may contain only `instructions/`.

### 4.2 Where layers live

- **Personal / public layers:** a git repo, e.g. `github.com/<you>/agent-layer-personal`.
  Cloned to a cache (`~/.cache/agent-layerz/<name>@<rev>`) or used from a local checkout
  during development.
- **Client layers:** live in the client's **hub** repo at `<client-root>/hub/layer/`
  (see §4.4). The client root folder itself **need not and should not** be a git repo.
- **Group layers:** a private repo, or a subdirectory of the hub (`hub/layers/<group>/`).
- **Repo layer:** inside the repo at `.layer/` (same layout as §4.1, no separate repo).

### 4.3 Repo-level files

```
<repo>/
├── layer.yaml            # the stack manifest (§5.2)
├── layer.lock            # generated pin file (§5.3) — committed
├── .layer/               # repo layer sources
├── AGENTS.md             # GENERATED
├── CLAUDE.md             # GENERATED (thin: imports AGENTS.md + Claude-only block)
├── .claude/…             # GENERATED (agents, skills, settings.json, hooks)
├── .agents/skills/…      # GENERATED (shared Agent Skills dir for Codex & others)
├── .codex/…              # GENERATED (agents/*.toml, config.toml)
└── .mcp.json             # GENERATED if the target uses it
```

Every generated file starts with a header (or the format's equivalent) saying where it
came from:

```
<!-- GENERATED by agent-layerz — DO NOT EDIT. Source: <layer>@<rev>:<path> [+ overlays…]. Edit the source and run `alz build`. -->
```

Files the tool did not generate are **never overwritten**. A path collision is a build
error unless `--adopt` is passed, which turns the repo's hand-written agent config into repo-layer
sources first, so an existing repo needs only `alz init` and `alz build --adopt`:

| Hand-written | Becomes |
|---|---|
| `AGENTS.md` | `.layer/instructions/00-adopted.md` |
| `CLAUDE.md` | `.layer/targets/claude/CLAUDE.extra.md`, without `@AGENTS.md` and without paragraphs `AGENTS.md` already holds |
| `.claude/agents/*.md` | neutral agents; native-only fields go under `claude:` (§7.2), concrete models become `models.yaml` aliases; an agent a general layer defines becomes a `mode: replace` overlay |
| `.claude/skills/**`, `.claude/rules/*.md` | moved unchanged |
| `.claude/hooks/**` | moved to `.layer/hooks/`; hooks that run them use `run:` |
| `.claude/settings.json` | `permissions.yaml`, `hooks/hooks.yaml`, `settings.yaml`, `models.yaml` default, `workspace.add_dirs` |
| `.mcp.json` | `mcp.yaml` and `mcp.enable` |

`settings.json` and `.mcp.json` are adopted even when the build does not collide with them.
Adoption is planned in full before anything is written. If any part cannot be adopted safely
(an unknown permission tool, a literal env value in `.mcp.json`, a hook type other than
`command`), nothing changes and every problem is listed.

### 4.4 The client hub (multi-repo clients)

A multi-repo client gets one private **hub** repo, cloned next to the code repos. The hub
holds two separate things:

1. **The client layer**: agent *configuration* in `hub/layer/`. It is versioned with tags
   prefixed `layer-v*` so that doc commits don't force config releases.
2. **Shared project truth**: product, brand, domain, architecture decisions, cross-repo
   specs and the task board. These are *content*, not configuration. They are never
   compiled into repos. Agents read and write them in place.

```
<client-root>/                 (not git)
├── hub/                       private repo
│   ├── layer.yaml             the hub's own stack (it is also a "repo" for doc-role agents)
│   ├── .layer/                hub repo layer (e.g. doc-only permissions)
│   ├── layer/                 CLIENT LAYER sources (§4.1 layout), tagged layer-v*
│   ├── docs/                  product/, brand/, domain/, architecture/, adr/
│   ├── specs/NNN-slice/       spec.md, plan.md, review.md (cross-repo slices)
│   └── backlog/               Backlog.md board for the whole client (tasks labelled repo:*, slice:*)
├── <repo-a>/                  stack: personal → path:../hub/layer@layer-v* → ./.layer
└── <repo-b>/
```

Rules:
- Each code repo's manifest declares the hub in `workspace` (§5.2). The build generates the
  target's "additional directory" settings so agents can read the hub and write to it.
- Planning roles (designer, domain expert, architect) usually work **with the hub as their
  working directory**. Implementation roles work in code repos and use the hub only through
  the additional-directory setting.
- Contract ownership stays with the producing code repo (e.g. the backend owns the OpenAPI
  file). The hub links to it rather than copying it.
- Single-repo clients may skip the hub. The client layer then lives in its own repo, or in
  the repo's `.layer/`.
- **No `CLAUDE.md` / `AGENTS.md` at the client root.** Tools load ancestor instruction files
  natively, so content there would be loaded twice. `alz doctor` warns about this.

### 4.5 Protected paths (never generated, never overwritten)

The build must leave these alone, even when a layer has a file at the same path:

- `.claude/agent-memory/**` and any target's per-agent memory dirs. These are repo-owned
  lessons, committed to git, and they survive account switches by design.
- `backlog/**` (task boards), `docs/**`, `specs/**`.
- `.claude/settings.local.json`, any `*.local.*` file.

The list is extendable through `protected_paths:` in the repo manifest.

---

## 5. Manifests

### 5.1 `layer.yaml` (in every layer)

```yaml
name: personal                 # unique layer id
kind: personal                 # free-form label: personal | org | client | group | repo | …
visibility: public             # public | private | confidential
version: 1.4.0                 # semver; tag the repo with v<version>
description: Universal standards and generic agent personas
provides_slots:                # skills this layer fills (for lint/coverage)
  - arch-standards
requires_slots:                # skills this layer references but expects others to fill
  - stack-conventions
  - domain-glossary
forbid_patterns:               # leak guard for public layers (§9.2), regex
  - "(?i)\\b(acme|clientname)\\b"
```

### 5.2 `layer.yaml` (repo stack manifest)

```yaml
stack:
  - source: github:<you>/agent-layer-personal@v1.4.0
  - source: path:../hub/layer                  # client layer inside the client hub (§4.4)
    ref: layer-v0.3.0                          # optional pin for path sources inside a git repo
  - source: path:./.layer                      # repo layer — always last
targets: [claude, codex]
workspace:
  hub: ../hub                                  # optional; enables hub access (§4.4)
  add_dirs: [../hub]                           # emitted as each target's additional-directory setting
protected_paths: []                            # extra paths the build must never touch (§4.5)
mcp:
  enable: [context7, playwright]               # allowlist; others defined in layers stay off
budgets:
  instructions_max_lines: 400                  # build fails if composed AGENTS.md exceeds
  always_loaded_skills_max: 0                  # skills preloaded into the main session
options:
  commit_generated: true                       # default true (§8.3)
  strict_slots: true                           # missing slot = error (else warning)
```

Supported `source` schemes: `path:`, `github:owner/repo@ref`, `git:<url>@ref`.

### 5.3 `layer.lock`

Generated by `alz build` / `alz update`. Records each layer's resolved revision
(commit SHA), a content hash, and the hash of every generated file. Used by `alz check`.

---

## 6. Resolution algorithm

1. Read `layer.yaml` from the repo and resolve each `source` to a local directory at the
   pinned revision (using the lock unless `alz update` is run).
2. Validate the stack: visibility ordering (§3.1), layer schema, no duplicate layer names.
3. For each item type, collect items from all layers **in stack order** (general → specific).
4. Apply the merge rule for that type (§7).
5. Check: slots filled, budgets met, leak guard on public layers, permission floor kept.
6. Render each target with its adapter (§8) into a temp dir.
7. Diff against the repo. Write only generated paths. Update `layer.lock`.

The build is **pure and deterministic**: the same inputs give byte-identical outputs
(sorted keys, stable ordering, no timestamps in file bodies).

---

## 7. Merge rules

General rule: **the more specific layer wins on conflicts, collections are combined, and
security can only get stricter.**

### 7.1 Instructions
- Concatenate all `instructions/*.md` in stack order, then by filename within a layer.
- Each layer's block is wrapped in `## <Layer kind>: <layer name>`.
- A layer may **suppress** a block from a more general layer: `instructions/<same-filename>`
  with frontmatter `mode: replace` replaces it, and `mode: remove` drops it.
- The composed result becomes `AGENTS.md`. `CLAUDE.md` is generated as an import of
  `AGENTS.md` plus a `## Claude-specific` block built from any `targets/claude/CLAUDE.extra.md`.
- The `instructions_max_lines` budget is enforced. Long standards should be **skills**, not
  instructions.

### 7.2 Agents
An agent source is Markdown with YAML frontmatter, written in a **neutral schema**:

```yaml
---
name: architect                      # required, kebab-case, unique after merge
description: When to use this agent  # required
role: planning                       # used for model routing (§7.8)
model: inherit                       # or a logical model id from models.yaml
tools: [read, grep, glob, write:docs/**, bash:mise run *]   # neutral tool vocabulary
skills: [arch-standards, adr, stack-conventions]           # preload list; slots allowed
isolation: worktree                  # optional: none | worktree
permission_mode: default             # optional
memory: project                      # optional: none | project | user
targets: [claude, codex]             # optional restriction
---
<system prompt body>
```

Overlay (`<name>.overlay.md` in a more specific layer):

```yaml
---
extends: architect
mode: extend          # extend (default) | replace
description: ~        # ~ = keep inherited; a value replaces it
skills: +[domain-glossary, cost-guardrails]   # "+[…]" append, "-[…]" remove, "[…]" replace
tools: -[write:docs/**] +[write:docs/adr/**]  # see narrowing rule below
model: ~
---
<this text is appended to the body under "## <layer kind> context: <layer name>">
```

Rules:
- **Scalars:** the most specific value wins.
- **Lists (`skills`):** apply `+`, `-` and replace in stack order. The result is de-duplicated
  and order is kept.
- **`tools` (narrowing rule):** an overlay may only remove tools or narrow a tool's scope.
  It may not add a tool the base didn't have, unless the overlay sets `allow_widen: true`
  **and** the layer is not public. Violations are build errors.
- **Body:** `extend` appends a section per layer. `replace` replaces the whole body (the
  build warns about it).
- `mode: replace` with no base means "define new". An overlay whose `extends` target can't
  be found is an error.
- Two layers defining a **base** (non-overlay) agent with the same name is an error. Use an
  overlay or `mode: replace` instead.
- **Native fields:** a `claude:` (or `codex:`) map holds target fields the neutral schema does
  not model, such as `effort`, `maxTurns`, `color` or per-agent `hooks`. They are passed through
  to that target only, and overlays deep-merge them. Keys with a neutral field (`tools`,
  `model`, `permissionMode`, …) are rejected inside the map.

### 7.3 Skills
- Identity is the skill directory name (it must match `name:` in the SKILL.md).
- The **most specific layer wins** on a name collision, and the whole skill directory is
  replaced. This intentionally overrides each tool's native precedence, because all skills
  are emitted at project scope (§8.4).
- **Slots:** a skill name listed in any agent's `skills` or in a layer's `requires_slots`
  must exist in the final set. If `strict_slots` is true, a missing slot is an error;
  otherwise the build emits a stub skill that says "not defined for this repo" and warns.
- Skill **extension** is not supported (skills are atomic). To specialise a skill, the more
  general skill should reference a slot skill, e.g. "also apply `stack-conventions`".

### 7.4 Commands
- `commands/<name>.md` compiles into a skill `<name>` that can only be invoked explicitly
  (no automatic model invocation), using the target's equivalent flag.
- Same collision rule as skills. A command and a skill with the same name in one layer is an error.

### 7.5 Hooks
`hooks.yaml`:

```yaml
- id: block-secrets              # unique id across the stack
  event: pre_tool                # neutral events: session_start | pre_tool | post_tool | stop | subagent_stop | prompt_submit
  match: { tools: [read, write, edit], paths: [".env*", "**/credentials*"] }
  run: scripts/block-secrets.sh  # relative to the layer; copied into the repo output
  on_fail: deny                  # deny | warn | feedback
  targets: [claude, codex]
```

- A hook has exactly one of `run:` (a script inside the layer, copied into the repo) or
  `command:` (a command line used as written). In a command, `${PROJECT_DIR}` is the repo root;
  each adapter replaces it with its own variable. `timeout` is in seconds.
- `event` may also be a target's own event, written `<target>:<Event>` (e.g.
  `claude:Notification`). It runs on that target only and takes the target's `matcher` string
  instead of `match`.
- **All hooks from all layers are combined.** Execution order is stack order, then file order.
- A more specific layer may **disable** a hook by `id` with `- id: x\n  disabled: true`,
  **except** hooks marked `locked: true`. Personal or client layers use `locked` for
  security floors.
- Hook scripts are copied to `.claude/hooks/<layer>/…` (or the target's equivalent) and
  referenced through the project-dir variable, never by absolute path.
- A hook event a target doesn't support becomes a build **warning** for that target. The
  report lists which guardrails fall back to pre-commit/CI.

### 7.6 Permissions
`permissions.yaml`:

```yaml
allow: [ "bash:mise run *", "bash:git status" ]
ask:   [ "bash:git push *" ]
deny:  [ "read:.env*", "bash:rm -rf *", "bash:git push --force*" ]
default_mode: default
```

- `allow`, `ask` and `deny` are each **combined** across layers.
- **Deny always wins.** A more specific layer cannot remove a deny from a more general layer
  (the **permission floor**). Removing one is a build error.
- `default_mode`: most specific wins, but it can never be looser than the strictest mode
  declared with `floor: true` in a more general layer.

### 7.7 MCP servers
`mcp.yaml` defines servers by id:

```yaml
context7:
  transport: http
  url: https://mcp.context7.com/mcp
playwright:
  transport: stdio
  command: npx
  args: ["@playwright/mcp@latest"]
  env_from: [ ]            # names of env vars to pass through; values are NEVER stored here
```

- Same id → the most specific definition wins, and the whole entry is replaced.
- Only ids listed in the repo manifest's `mcp.enable` are emitted (allowlist). The rest are
  defined but inactive, which keeps the token budget under control.
- Secrets: definitions may name env vars but must not contain values. The leak guard rejects
  anything that looks like a token.

### 7.8 Model routing
`models.yaml`:

```yaml
aliases:                       # logical → per-target concrete model ids
  strong:  { claude: opus,   codex: <codex-strong-model> }
  fast:    { claude: sonnet, codex: <codex-fast-model> }
roles:
  planning: strong
  review:   strong
  implementation: fast
  research: fast
```

- Aliases and roles are deep-merged, and the most specific value wins.
- An agent's `model: inherit` resolves through its `role`. An explicit alias overrides the role.
- Concrete model ids live **only** here, so swapping models is a one-line change in one layer.

### 7.9 Other settings
`settings.yaml`, namespaced by target:

```yaml
claude: { <native settings.json keys> }
codex:  { <native config.toml keys> }
```

Deep merge. Scalars: most specific wins. Arrays: concatenated and de-duplicated. These are
passed through to the target **without validation** beyond a schema check where one exists.
Keys that duplicate a first-class item (permissions, hooks, MCP, model) are rejected, so
there is only one way to set each thing.

### 7.10 Path-scoped rules
`rules/<name>.md` holds Markdown with frontmatter `paths: [<globs>]`. It's loaded only when
the agent works on matching files (e.g. `backend.md` → `src/**`).
- Identity is the filename. The most specific layer wins, and the whole file is replaced.
- `mode: extend` in a more specific layer appends its body under a layer heading and
  combines `paths` (union).
- Output: the target's native path-scoped rules (`.claude/rules/` for Claude). For a target
  without them, the adapter emits a nested `AGENTS.md` per directory where the globs resolve
  to one directory. Otherwise it warns.
- Rules are how area conventions stay out of the always-loaded instructions budget.

### 7.11 Passthrough (`targets/<target>/…`)
This is the escape hatch for target features the neutral schema doesn't cover. Files are
copied into place, and the most specific layer wins per path. The build warns on each use
so passthroughs get turned into first-class items over time.

---

## 8. Target adapters

Each adapter maps the resolved neutral model onto native files. Adapters are plugins of the
tool (one module each), so new targets need no core changes.

### 8.1 `claude` adapter
| Neutral item | Output |
|---|---|
| Instructions | `CLAUDE.md` = `@AGENTS.md` + Claude-only block |
| Agents | `.claude/agents/<name>.md` (map neutral tools/skills/model/isolation/memory to native frontmatter) |
| Skills / commands | `.claude/skills/<name>/` |
| Hooks | `.claude/settings.json` → `hooks`, scripts in `.claude/hooks/` |
| Permissions | `.claude/settings.json` → `permissions` |
| MCP | `.mcp.json` (only enabled ids) |
| Models | resolved into agent frontmatter; default in `settings.json` |

### 8.2 `codex` adapter
| Neutral item | Output |
|---|---|
| Instructions | `AGENTS.md` (shared with all targets) |
| Agents | `.codex/agents/<name>.toml` (instructions point to a generated body file) |
| Skills / commands | `.agents/skills/<name>/` |
| Hooks | `.codex/config.toml` / hooks config where supported; otherwise a warning |
| Permissions | sandbox / approval settings in `.codex/config.toml` (best-effort mapping; anything that can't be mapped becomes a warning) |
| MCP | `.codex/config.toml` → MCP servers (only enabled ids) |
| Models | per-agent and default model in `.codex/config.toml` |

### 8.3 Committing generated output
- Default `commit_generated: true`. Generated files are committed so CI, cloud agents,
  collaborators and other accounts all see the same setup, and so changes appear in PRs.
- With `false`, generated paths are added to `.gitignore` and `alz build` must run in
  CI and on checkout (via a mise hook or task).

### 8.4 Account independence (key rule)
The build writes **only to project scope**. User-level dirs (`~/.claude`, `~/.codex`, and
every per-account variant such as `~/.claude-<client>`) should hold **no agents or skills**
that might collide by name. Check: `alz doctor` scans every configured account dir and
reports collisions with the built names. The result is that switching accounts never
changes agent behaviour.

---

## 9. Safety rules

### 9.1 Never touch
Credentials, auth/token files, `.env*` contents, account config dirs (they are read only by
`alz doctor`, and only to list names, never contents) and anything outside the repo and
the layer cache.

### 9.2 Leak guard
For layers with `visibility: public`:
- Before `alz publish` or a commit on the layer repo, scan every file against the layer's
  `forbid_patterns`, a built-in secret scanner (gitleaks or equivalent) and a heuristic for
  private hostnames and emails.
- A public layer that `extends` a private one is an error (§3.1).

### 9.3 Security floor
Locked hooks (§7.5) and deny rules (§7.6) can't be removed by more specific layers. The
build fails, with a message naming the layer that tried.

---

## 10. CLI

| Command | Does |
|---|---|
| `alz init [--stack <source>]… [--hub <dir>]` | Scaffold a repo manifest with these layers, then `.layer/`. `--hub` also adds the hub's `layer/`, pinned to its newest `layer-v*` tag. `--kind layer` scaffolds a new layer. |
| `alz build` | Resolve, merge, render, write, update the lock. |
| `alz check` | Rebuild in memory and fail if the repo differs from it or from the lock (for CI and the Stop hook). |
| `alz update [<layer>]` | Re-resolve refs to the newest matching versions and rebuild. |
| `alz ls [--live]` | List what the stack builds and the layer that decides each item. `--live` also asks Claude what it loaded (one short model call) and fails on any built item it did not load. |
| `alz explain <type> <name>` | Show which layers contributed to an item, in order, with the merged result. |
| `alz diff` | Show the effective change a pending build would make. |
| `alz doctor` | Check account dirs for collisions, target versions vs. adapter expectations, unfilled slots, budgets. |
| `alz lint [--layer <path>]` | Validate a layer: schema, leak guard, slot declarations. |
| `alz publish` | Lint, then tag `v<version>` on a layer repo (no push without `--push`). |

Expose these as mise tasks in each repo: `mise run alz:build`, `alz:check`, etc.

**Implementation:** TypeScript on Node ≥ 22 (the oldest Node line still supported), so the CLI
runs under whatever Node a project pins. Runtimes pinned by mise. No `.js` extensions on
relative imports. Single package, distributed through npm and runnable with `npx`.
Dependencies kept minimal (YAML parser, glob, a TOML writer, a diff library).

---

## 11. Coexisting with native plugins

- Native plugin systems (Claude Code plugins/marketplaces, Codex plugins) stay usable.
  Plugin items are namespaced natively, so they can't take part in overrides. Treat them as
  **external dependencies**, declared in `settings.yaml` under the target, and keep layered
  items out of plugins.
- Optional later: `alz export-plugin <layer>` packages a layer as a native plugin, for
  people who want just one public layer without this tool.

---

## 12. Facts to verify before implementing (do not trust this spec blindly)

Check each against the installed version's `--help`, docs and a smoke test, and record the
findings in `docs/target-facts.md` with the version and date:

1. Claude Code: subagent frontmatter fields (`tools`, `skills` format, `model`, isolation
   field name, memory, permission mode, hooks in frontmatter).
2. Claude Code: same-name precedence for agents (expected: project > user > plugin) and for
   skills (expected: personal > project). This is what §8.4 exists to neutralise.
3. Claude Code: does `CLAUDE.md` `@import` work for `AGENTS.md`, and is `AGENTS.md` now read
   natively? If it is, still generate the thin `CLAUDE.md`, but avoid loading it twice.
4. Claude Code: hook event names, JSON shape, how hook arrays merge across scopes, and the
   project-dir environment variable.
5. Claude Code: whether skills load from parent directories and from nested `.claude/skills`,
   so that an ancestor folder (e.g. a client root) doesn't inject unexpected skills.
6. Codex: custom agent TOML schema and location, skills lookup order, `AGENTS.md` merge order,
   `AGENTS.override.md` behaviour, hooks support, MCP config keys.
7. Both: how an unsupported item is handled (it must not break startup).
8. Both: the setting that grants access to an additional directory (Claude Code
   `additionalDirectories` / `--add-dir`, the Codex equivalent), and whether skills, rules
   or instructions inside that directory get loaded as a side effect. If they do, the hub
   must not contain `.claude/` content that conflicts with the code repos.
9. Claude Code: the path-scoped rules format (`.claude/rules/*.md`, `paths:` frontmatter)
   and the subagent `memory` field / memory dir location (to protect it, §4.5).

Each adapter declares the version range it was verified against, and `alz doctor` warns
when the installed tool is outside that range.

---

## 13. Acceptance tests

The tool is done when all of these pass in CI (fixtures under `test/fixtures/`):

1. **Determinism:** building twice gives byte-identical output, and `alz check` passes.
2. **Instruction stacking:** personal + client + repo produce one `AGENTS.md` with three
   headed sections in order. `mode: remove` drops a block.
3. **Agent overlay:** a base `architect` from layer 1, plus a skills overlay from layer 2,
   plus a body overlay from layer 3, gives one agent with the merged skills and three body
   sections. `alz explain agent architect` shows all three sources.
4. **Tool narrowing:** an overlay that adds a tool fails, and one that removes a tool passes.
5. **Skill override:** a repo `stack-conventions` replaces the personal one, even when a
   same-name skill exists in a user dir (the doctor reports the collision).
6. **Slots:** a missing slot fails with `strict_slots: true` and produces a stub plus warning
   with `false`.
7. **Permission floor:** a repo layer removing a personal deny fails. Allow lists combine.
8. **Locked hook:** disabling a locked hook fails, and disabling an unlocked one works.
9. **MCP allowlist:** a server defined but not enabled doesn't appear in any output.
10. **Leak guard:** a public layer containing a forbidden pattern or a fake token fails `lint`.
11. **Visibility:** a public layer extending a private one fails.
12. **Account independence:** with `CLAUDE_CONFIG_DIR` (or equivalent) pointed at two
    different empty account dirs, the effective agents and skills listed by each tool are
    identical.
13. **Two targets:** the same fixtures produce valid output for both `claude` and `codex`.
    Both tools start in the fixture repo without config errors (smoke test, skipped if the
    tool isn't installed).
14. **Non-destructive:** an existing hand-written `.claude/agents/foo.md` makes the build fail
    with a clear message, and `--adopt` moves it into `.layer/agents/`.
15. **Protected paths:** files under `.claude/agent-memory/`, `backlog/`, `docs/` and `specs/`
    are byte-identical before and after a build, even when a layer ships a file at the same path.
16. **Hub:** a two-repo fixture with a hub resolves `path:../hub/layer@layer-v0.1.0`. Each repo's
    output grants the hub as an additional directory. A doc commit in the hub after that tag
    doesn't change either repo's build. `doctor` warns about a `CLAUDE.md` at the client root.
17. **Path-scoped rules:** a repo `rules/backend.md` overrides the personal one, and an
    `extend` rule combines `paths`.

---

## 14. Rollout (generic, for any client)

1. **Personal layer:** extract universal standards, generic agent personas (architect,
   designer, domain-expert, backend-engineer, frontend-engineer, reviewer, …), generic
   skills, security-floor hooks and denies, and model aliases. Mark it `public` and run `lint`.
2. **Client hub + layer:** for each client, create the private hub repo at `<client-root>/hub/`
   (§4.4). Put the domain context, constraints, persona overlays and client-wide slot fills
   in `hub/layer/` and tag it `layer-v0.1.0`. Put the docs, specs and Backlog.md board in the
   rest of the hub.
3. **Repo layer:** in each repo, run `alz init`, add the stack-specific slot fills
   (`stack-conventions`, …), set the MCP allowlist and budgets, then `alz build`.
4. **Clean the account dirs:** move agents and skills out of `~/.claude*` and `~/.codex*`
   into the right layer until `alz doctor` is clean.
5. **CI:** add `alz check` to each repo's pipeline, and to the agent's Stop hook if wanted.
6. **Iterate:** each lesson learned (e.g. from a `/learn` command) goes into the narrowest
   layer it applies to, as a PR on that layer's repo, followed by `alz update` in consumers.

---

## 15. Open questions (decide before v1.0)

- Should `group` layers be first-class (e.g. auto-detected from a folder convention), or
  only declared explicitly in each repo's stack? Leaning towards explicit.
- Should `commit_generated` default to `true` for every client, or be configurable per
  client layer? Leaning towards a client-layer default that repos can override.
- Format for publishing a layer as a native Claude/Codex plugin (§11): later.
- A third target (Gemini CLI / OpenCode) once v1 is stable.
