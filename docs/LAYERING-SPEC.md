# LAYERING-SPEC.md — Layered agent configuration for any client and project

**Status:** Draft v1 · 2026-11-22 (§7.12 commit identity added; needs agent-layerz 0.2.0.
§7.13 account selection specified, planned in TASK-50 and not built yet)
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
8. Optionally **choose the account folder** each repository's agent tools start with, from
   the layers and a machine-local user config, so a client's repositories start on that
   client's account (§7.13, planned).

**Non-goals (v1)**
- Running agents or orchestrating them. That is the orchestrator's job. (`alz run`, §7.13,
  only starts one tool with its account variable set.)
- Managing credentials and logins. Logging in stays with each tool. alz may choose which
  account folder a tool starts with (§7.13), but it never reads, copies or prints a login
  and never opens a file inside an account folder.
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

`kind` is a free-form label, with one exception: only a layer with `kind: client` may
override the commit identity (§7.12) and, once built, the account (§7.13, planned).

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
commit_identity:               # optional; first layer or one client layer only (§7.12)
  name: Your Name
  email: you@example.com
```

Unknown fields are a schema error, so a layer that declares `commit_identity` needs
agent-layerz 0.2.0 or later.

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

*Planned (TASK-50.2):* no layer may set `CLAUDE_CONFIG_DIR`, `CODEX_HOME`, `HOME`,
`XDG_CONFIG_HOME`, `XDG_STATE_HOME` or `XDG_CACHE_HOME`, which choose the account folder or
locate alz's own files and the layer cache (§7.13). That covers environment settings in
`settings.yaml` (such as Claude's `env` or Codex's `shell_environment_policy`), an MCP
server's environment, and a passthrough settings or config file (§7.11). Each one is a build
error that names the layer. Account resolution reads layers from the cache and relies on its
integrity, which TASK-51 fixes.

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

### 7.12 Commit identity
The person who owns the stack declares the identity that every commit is authored and
committed under, in the `layer.yaml` of the stack's first layer (§5.1). A client layer may
override it for that client's repos, for example with a client-issued email:

```yaml
commit_identity:
  name: Your Name
  email: you@example.com
```

- **Who may declare it.** A layer is *outside the repository* when its source is `github:`
  or a `git:` URL of a remote, or a `path:` (or a `git:` local path or `file://` URL, read
  against the repo root) that resolves outside the repo root both as written and after
  following symlinks, using the file system's own spelling of each name, so a case variant of
  a path inside the repo, and a link outside the repo that points into it, are inside. A
  `git:` argument counts as a local path, whatever it looks like (`x:y`, `file:///dir`), when
  a path of that name exists relative to the repo root, as given or with git's own suffixes
  `/.git`, `.git/.git`, `.git` or `.bundle`, because `git clone` looks for such a path before
  it reads the argument as a URL. Only then is a `file://` URL its absolute path, and a
  `scheme://` or `host:path` argument a remote. The path is looked up from the string as
  written, so the operating system resolves any `..` through symlinks exactly as git will,
  then canonicalised; the inside check and `git clone` both use that canonical path, never the
  URL as written. A local `git:` source (including `file://`) with a `..` segment is a source
  error that names the source, because text and disk can disagree about where it leads. A
  `git:` or `github:` source classed as a remote is cloned with `-c protocol.file.allow=never`
  and without `GIT_ALLOW_PROTOCOL`, which would override that setting, so git cannot fall back
  to a local path (`docs/target-facts.md` §11); a `url.<base>.insteadOf` that rewrites such a
  URL to a local mirror therefore fails to clone. Only two layers may declare `commit_identity`:
  1. the **first stack entry** (normally `personal`), if it is outside the repository;
  2. one **client layer**: a later layer with `kind: client`, `visibility: private` or
     `confidential`, outside the repository (normally the hub's `layer/`, §4.4).

  `kind` stays a free-form label everywhere else (§3.1); `client` has this one meaning. Any
  other declaration is a build error that names the layer: the repo layer (inside the
  repository, whatever its `kind` says), group layers, any other kind, and a public layer
  that calls itself `client`. In the hub repo itself (§4.4) `layer/` is inside the
  repository, so the hub's own commits use the first layer's identity. There is no overlay,
  `~` or remove form. `alz lint` of a single layer checks the shape only, because it cannot
  know the layer's position or source.
- **Which one wins.** The client layer's declaration replaces the first layer's whole
  identity, name and email together; the two are never mixed. Equal values are fine. A
  declaration by a second client layer is a build error naming all the client layers that
  declare, so no further layer can override silently. If only one of the two declares, that one applies.
- **Where it came from.** The winning layer is named in the generated Commits section, in
  the `AGENTS.md` header sources, and in every `init`, `build` and `check` report (§9.4).
- **No declaration, no feature.** Nothing is added to `AGENTS.md`, and `init`, `build` and
  `check` run no git command for it and print nothing about it. (`init` still resolves the
  stack to look for a declaration, which fetches remote layers like `build` does.)
- **Validation.** The identity is untrusted layer content and reaches a git subprocess
  (§9.4), so `layer.yaml` parsing rejects it as a schema error unless:
  - the map has exactly the keys `name` and `email`, both strings;
  - `name` is 1 to 128 characters, has no leading or trailing whitespace, does not start with
    `-`, and contains no control character (U+0000–U+001F, U+007F–U+009F, U+2028, U+2029),
    no Unicode format character (category Cf: zero-width, bidi controls, BOM, soft hyphen),
    and no `<`, `>`, backtick or backslash. It does not start or end with `.` `,` `:` `;` `"`
    `'` `\` `<` or `>`: git trims most of these from both ends of a name
    (`docs/target-facts.md` §11), so the declared name could never match what git reports.
    An apostrophe inside a name is allowed;
  - `email` is at most 254 ASCII characters, does not start with `-` and neither starts nor
    ends with `.`, has no format character, and matches
    `^[A-Za-z0-9._%+-]+@[A-Za-z0-9-]+(\.[A-Za-z0-9-]+)+$`.
- **Generated Commits rule.** With a declaration, the build appends this section as the last
  section of `AGENTS.md`, with the winning values in place of `<name>` and `<email>` and the
  winning layer's name in place of `<layer>`, and adds `<layer>@<rev>:layer.yaml` (winning
  layer only) to the header's sources:

  ```markdown
  ## Commits

  - Author and committer of every commit made in a local clone: `<name> <<email>>` (declared by layer `<layer>`). This repository's local git config sets it; never change `user.name` or `user.email`, and never commit locally under a CI, bot or shared identity.
  - A commit made by an agent ends with a `Co-Authored-By:` trailer that names the agent. Use the trailer your tool adds; if it adds none, write one.
  - If `git var GIT_AUTHOR_IDENT` shows another identity, do not commit; report it.
  - Bots that commit on the hosting service, such as release or CI bots, are exempt from this rule.
  ```

  The section is derived from layer content only, so builds stay byte-identical. It counts
  towards the instruction budgets (§5.2). It has no instruction filename, so no layer can
  `mode: replace` or `mode: remove` it. Changing this text changes generated output and needs
  a release note.
- **Attribution is instruction-only.** alz sets no Claude Code or Codex attribution setting
  and relies on no tool behaviour for the trailer (`docs/target-facts.md` §10). A layer may
  still set a target's own attribution keys through `settings.yaml` (§7.9).
- **Setting it up and checking it** is described in §9.4: `alz init` and `alz build --adopt`
  set the repo-local git identity, `alz build` warns about a mismatch, `alz check` fails on one.
- **Scope: people and agents committing in a local clone.** Bots that commit on the hosting
  service (a release bot, a CI workflow) are exempt, and CI runs `alz check --skip-identity`
  (§14). alz never configures a bot identity.
- **Trust.** The repo manifest is repo content and chooses the stack, so a cloned repo could
  name an external layer it controls as the first layer, or as a private `kind: client`
  layer. The rule above stops the repository's own files from declaring an identity, but it
  cannot tell your personal layer from someone else's. Three things limit the damage: only
  the setup commands write (§9.4); they only add missing local keys and never replace one;
  and every write and report names the declaring layer and its source.
- The identity belongs to one person. In a repository that several people build, everyone
  but the owner sees the `build` warning and fails `check`; they pass `--skip-identity` (§10).

### 7.13 Account selection
*Planned: TASK-50, decision-15 as amended by decision-16 (the owner's decisions recorded in
TASK-50.1). Nothing in this section is built yet; each part names the task that builds it.*

Claude Code and Codex keep logins, workspace trust and per-user state in an account folder
that an environment variable chooses. alz can choose that folder per repository, so a
client's repositories start on the account that client expects. It is **off by default**: with
no layer declaration and no user config, alz sets no variable, prints nothing about accounts,
and every tool starts on its default account as it does today. alz never reads, copies or
prints a login and never opens a file inside an account folder; §9.1 lists the only
operations it performs there.

A layer names the account for every target, or per target:

```yaml
account: client-a              # every target
```

```yaml
account:                       # per target; a target left out is not decided by this layer
  claude: client-a
  codex: client-a-api
```

- **Account names** (TASK-50.2). A name matches `^[a-z0-9][a-z0-9-]*$` and has at most 64
  characters, and the map's keys are target names (§8). `default` always means the default
  account. Anything else in the field, such as a path or a secret, is a schema error from
  `layer.yaml` parsing and `alz lint`, because the name becomes part of a folder path and of
  the shell hook's output. An account name that alz reads from the environment or from
  `allowed.yaml` must match the same rule, or it is ignored with a warning.
- **Folders per target** (TASK-50.2, TASK-50.3). Each target adapter (§8) defines its variable
  and its folders, and alz sets only these variables. Account selection for Codex only sets
  `CODEX_HOME`, so it needs none of the rest of the Codex adapter (§8.2). The facts behind
  this table are §12 items 10 to 18:

  | Target | Variable | Default account | Named account `<name>` |
  |---|---|---|---|
  | `claude` | `CLAUDE_CONFIG_DIR` | variable unset: `~/.claude` plus `~/.claude.json` | `~/.claude-<name>` |
  | `codex` | `CODEX_HOME` | variable unset: `~/.codex` | `~/.codex-<name>` |

  The default account is always the variable unset, and it never fails, even when its
  folder does not exist yet.
- **Who may declare it** (TASK-50.2). The same two layers as `commit_identity`, with the same
  test for "outside the repository" and the same build errors (§7.12): the first stack entry,
  or one private or confidential `kind: client` layer outside the repository. The client
  layer's value replaces the first layer's whole value, a per-target map included; the two
  are never mixed. A declaration by any other layer, or by a second client layer, is a build
  error that names the layers, and `alz run`, the shell hook and `alz account which` ignore it
  and print one warning. `alz lint` of a single layer checks the shape only. In the hub repo
  itself (§4.4) `layer/` is inside the repository and cannot declare, so hub sessions get
  their account from the user config's `folders`.
- **Layers cannot set the variables** (TASK-50.2). No layer may set an account variable, or
  the variables that locate the account folders, alz's own files and the layer cache, through
  any other item (§7.9).
- **User config** (TASK-50.2). The owner's override lives in
  `${XDG_CONFIG_HOME:-~/.config}/agent-layerz/accounts.yaml`. It is written by hand: alz reads
  it and never writes or rewrites it. It is machine-local, outside every repository, never
  committed and never generated. Account names in it follow the same rule:

  ```yaml
  default_name: personal           # what the default account is called on this machine
  folders:                         # a folder and everything below it → an account
    ~/work/client-a: client-a
    ~/work/client-b:
      claude: client-b
      codex: default
  ```

  `default_name` is the name the default account goes by on this machine: a layer or
  `--account` that names it gets the default account, as `default` always does. alz warns
  when `~/.claude-<default_name>` or `~/.codex-<default_name>` exists, because that folder
  can then never be chosen. Paths are absolute or start with `~/`.
- **Allow state** (TASK-50.2, decision-16). Approvals live in
  `${XDG_STATE_HOME:-~/.local/state}/agent-layerz/allowed.yaml`, a file only
  `alz account allow` writes. It holds one entry per repository root, sorted by root:

  ```yaml
  # written by alz account allow; deleting the file withdraws every approval
  - root: /home/you/work/client-c/api    # real path of the repository root
    account:                             # the accepted account per target
      claude: client-c
      codex: client-c
    source: /home/you/work/client-c/hub/layer  # local layer's real path, or URL without ref
  ```

  On read, an entry whose `root` is not absolute, or whose account names break the name
  rule, is ignored with a warning.
- **Both files** (TASK-50.2). alz ignores `accounts.yaml` or `allowed.yaml`, with a warning,
  when its real path is inside the current repository, when the user does not own it, when
  group or others can write it or its directory or any parent directory below `$HOME`, or
  when the `XDG_*` variable that locates it was set by mise or direnv (where §12 item 20 shows
  how to detect that). It warns when the file lies inside a git work tree.
  `alz account which` prints the paths of both files. `alz account allow` holds an exclusive
  lock file next to `allowed.yaml` while it reads and writes it, replaces any entry for the
  same root, and writes the file at its real path through a temporary file in the same
  directory and a rename, with mode `0600` in a directory with mode `0700`.
- **Trust and `alz account allow`** (TASK-50.2, decision-16). The repo manifest is repository
  content and chooses the stack, so a cloned repository can name an external layer that names
  one of your accounts: the weakness §7.12 "Trust" describes for the commit identity. The
  session would then run with that account's logins and MCP connections. So an account named
  by a layer applies only after `alz account allow` in that repository:
  - `allow` reads the stack the way resolution does for an allowed repository (below), so it
    accepts exactly what resolution will apply. On a cache miss it changes nothing and says
    to run `alz build`.
  - It prints the account per target, the declaring layer, its source and its revision, then
    asks the person to confirm by typing the account name (not `y` or `n`). It refuses and
    changes nothing when standard input is not a terminal or when it runs inside an agent
    session (the session markers are §12 item 19). There is no `--yes`.
  - It records the repository's real root, the accepted per-target map (with the targets it
    leaves out) and the declaring layer's source: the remote URL without its ref, or the real
    path of a local layer. Step 4 of the resolution applies only while the resolved map and
    the source both equal the recorded ones; any difference, including a target added to or
    removed from the map, skips step 4 for every target. A ref bump needs no new approval,
    because the account is what is approved, not the layer's revision.
  - `alz account allow --remove` removes this repository root's entry, under the same lock
    file. It has the same terminal and agent-session checks, because without an entry the
    repository goes back to the default account. Deleting `allowed.yaml` removes every entry.
  - `alz build` and `alz check`, which read the stack anyway, print the `alz account allow`
    command when a layer names an account in a repository that is not allowed, or when an
    allowed entry has changed (then with the old and new values). They print it only when
    standard input is a terminal and no agent-session marker is set. `alz account which`
    prints the command only when an allowed entry has changed; for a repository that is not
    allowed yet, it reports only that step 4 was skipped. The shell hook and `alz run` never
    print it.
  - Accounts set in the user config need no approval, because the owner wrote them.
  - **Limit.** alz runs as the user, so an agent that can write outside the repository can
    edit `accounts.yaml` or `allowed.yaml` itself; the terminal and session checks only stop
    it from running `alz account allow`. The real protection is a sandbox that limits the
    agent's writes to the working directory. The personal layer should also hold deny rules,
    which no more specific layer can remove (§7.6), for `read:`, `write:` and `edit:` on
    `~/.config/agent-layerz/**` and `~/.local/state/agent-layerz/**`, and for
    `bash:alz account allow*`. Whether the tools enforce such rules is §12 item 21.
- **Resolution** (TASK-50.2), for each target, first match wins:
  1. `alz run <tool> --account <name>`, for that run only;
  2. `alz use <tool> --account <name>`, for the current terminal until
     `alz use <tool> --reset`;
  3. the user config's `folders` entry for the working directory: the longest path that is
     the directory itself or one of its parents;
  4. the layer declaration, if allowed (above) and readable without a fetch (below);
  5. the default account.

  A step that names no account for this target is skipped. `alz account which` prints the
  account per target and its source: `use`, `environment` (a value set outside alz, see
  below), `user config`, `layer` or `default`, with the reason for every skipped step.
- **Paths** (TASK-50.2). The repository root is the nearest directory at or above the working
  directory whose `layer.yaml` has a `stack` key (§5.2); a `layer.yaml` without one, such as
  a layer's own manifest (§5.1), is skipped and the walk goes on. Each `layer.yaml` read on
  the way is parsed only up to 1 MiB and with a limit on YAML aliases. Paths are resolved to
  real paths using the file system's own spelling of each name, as in §7.12. A `folders`
  entry matches whole path segments, so `~/work/client-a` does not match `~/work/client-ab`.
  An `allowed` entry matches only the exact real root of the repository whose manifest is
  being resolved, so a nested repository needs its own entry. An entry whose path does not
  resolve matches nothing.
- **No fetch on resolve** (TASK-50.2). Resolution (the shell hook, `alz run`,
  `alz account which`) never fetches, clones or runs git, and writes nothing. If the
  repository root is not in `allowed`, step 4 is skipped without reading the stack. If it is,
  resolution reads each layer's `layer.yaml` with the same validation as `build`, from:
  - a `github:` or `git:` layer: its cache entry for the commit the lock pins;
  - a `path:` layer with a `ref`: the cache entry named by the cache index. Next to each
    `tree@<tree>` entry, `build` writes an index entry keyed by the layer directory's real path
    and the commit SHA, for example `pathref@<sha256 of the real path>-<sha>`, that names its
    tree. `build` and resolution compute the real path with the same function, and
    resolution computes it from the manifest without git and reads only the index entry for
    that path, so a repository cannot select another source's entry. The entry is used only
    when `lstat` shows a regular file whose whole content is 40 or 64 hexadecimal
    characters;
  - a `path:` layer without a `ref`: the layer in place, using only its `layer.yaml`, with the
    inside/outside test of §7.12. Such a layer is never missing from the cache, but its
    content hash is still checked (below), so an edit since the last `alz build` is a miss.

  A lock `sha` is used only when it is 40 or 64 hexadecimal characters. Resolution checks each
  layer it reads against the lock's content hash (§5.3), which covers every file of the
  layer, and a mismatch counts as a cache miss. Anything else above is a cache miss too, and
  a cache miss skips step 4 and fails closed (below). The lock does not change. Resolution
  relies on the integrity of the cache, which TASK-51 fixes (cache entries checked against
  the lock's content hash, git cache keys that no two URLs share).
- **A named account that cannot apply fails closed** (TASK-50.2, TASK-50.3, TASK-50.6,
  decision-16). A named account cannot apply when its folder does not exist, explicit (steps
  1 and 2) or implicit (steps 3 and 4), or when, in an allowed repository, step 4 is skipped
  because of a cache miss, a changed declaration, or a recorded source that is no longer in
  the stack. Then alz never uses the default account, and never a newly declared account:
  - `alz run` and `alz use` fail before anything starts, exit non-zero and name the fix. For
    a missing folder they list the account folders that exist by name only; on a cache miss
    they say to run `alz build`; on a changed declaration or source, `alz account allow`.
  - The shell hook warns and always sets the variable of each target that the recorded entry
    or the declaration names:
    - to the chosen account's folder when that folder is missing;
    - to the recorded account's folder on a cache miss while the recorded source is still
      in the stack;
    - to the blocked folder, `~/.claude-alz-blocked` or `~/.codex-alz-blocked`, when the
      declaration changed or the recorded source is no longer in the stack.

    alz never creates the blocked folders, and `alz account init` refuses the name
    `alz-blocked`. `alz doctor` does not count them as accounts, but warns when one exists,
    because a tool may create it at a blocked start and a login there would make it usable.
    The tool then starts logged out or refuses to start, and both count as failing closed
    (§12 item 18 records which).
  - `alz account which` reports `layer step skipped (<reason>); fails closed`, what the hook
    applies (the missing folder, the recorded account or the blocked folder), and the fix
    command.

  This amends decision-15's "falls back to the default account" (decision-16).
- **Values set outside alz** (TASK-50.2, TASK-50.3, TASK-50.6). The shell hook exports, next to
  each variable it sets, a marker that holds the exact value it set:
  `ALZ_SET_CLAUDE_CONFIG_DIR` and `ALZ_SET_CODEX_HOME`. A current value that differs from its
  marker, or that has none, is foreign: alz leaves it alone and warns once that it was set
  outside alz, and it takes the place of steps 3 to 5. An explicit `--account` (steps 1 and
  2) replaces it. The warning names mise or direnv only where §12 item 20 confirms how to
  detect them. The marker also gives `alz account which` its `environment` source.
- **`alz use` stays in the shell** (TASK-50.6). `alz use` keeps its choice in unexported shell
  variables held by the `alz` shell function, which passes the choice to `alz run` as
  `--account`, and to the hook's resolution and to `alz account which` as an argument; that
  is the `use` source. An exported `ALZ_USE_<TARGET>` or any other environment variable is
  never read as an `alz use` choice, so a forged one changes nothing.
- **Shell hook** (TASK-50.6). `alz run` (TASK-50.3) starts one tool with the variable set and
  needs no hook. A Node CLI cannot change its parent shell's environment, so, as with mise,
  `alz use` and switching by folder need the hook from `eval "$(alz activate zsh)"` (or
  `bash`). The hook defines the `alz` shell function that `alz use` needs, and runs when it
  is loaded and at each prompt where the working directory changed. It then sets each
  target's variable and marker to the resolved account, or unsets them where the default
  account applies. Only `alz use` changes a foreign value. Because the output is evaluated:
  - the output contains only lines `export NAME='value'` and `unset NAME`, for the account
    variables and the `ALZ_SET_*` markers;
  - each value is built from `$HOME` and a validated account name, never from other text,
    and is single-quoted with every `'` written as `'\''`;
  - every message goes to standard error, sanitised as in §9.4 (home shown as `~`, control
    characters removed);
  - the activate script quotes every expansion.
- **Shared and per-account items** (TASK-50.5 links them, TASK-50.4 reports missing links;
  decision-16). `alz account init <name>` creates the named account folders and links shared
  items to the default account folder's copy, so instructions, agents and skills are the same
  in every account (§8.4). By default it shares configuration only. Settings and history stay
  per account unless the user opts in with `--share <item>`, which prints a warning.
  `--share` accepts only `settings.json`, `projects`, `sessions` and `memories`; anything else
  is rejected before planning:

  | Target | Shared by default | Per account; shared only with `--share` | Always per account, never linked |
  |---|---|---|---|
  | `claude` | `agents/`, `commands/`, `skills/`, `plugins/`, `CLAUDE.md` | `settings.json`, `projects/` | logins; `.claude.json` (workspace trust, per-project state, MCP logins); prompt history; file checkpoints |
  | `codex` | `AGENTS.md`, `skills/` | `sessions/`, `memories/` | logins; `config.toml` |

- **`alz account init` safety** (TASK-50.5).
  - It refuses the names `default` and `alz-blocked` (the blocked folders, above) and the
    configured `default_name`.
  - It uses `lstat` throughout. It refuses when an account folder, or the parent of a shared
    item, is a symbolic link, or when an account folder's real path is the default folder.
  - It plans every step before it acts, and changes nothing when the plan has any problem.
    `--dry-run` prints the plan and stops.
  - It creates folders with mode `0700`, and links only items that exist in the default
    folder.
  - It moves an existing item aside only by renaming it in the same folder to
    `<item>.alz-backup-<UTC timestamp>` (for example `skills.alz-backup-20261122T101500Z`),
    never by copying and never over an existing backup. A backup name never matches
    `.claude-*` or `.codex-*`, so account discovery (§8.4) never takes a backup for an
    account folder.
  - Running it again changes nothing.
- **Trust is per account** (TASK-50.4). Workspace trust lives in per-account state
  (`.claude.json`, Codex `config.toml`; to verify, §12 items 10 and 11), which alz never
  opens, so alz never determines whether a repository is trusted. `alz doctor` prints a
  static notice that trust is per account, so a repository trusted on one account needs
  accepting again on another.
- **No generated output** (TASK-50.2). The build writes no account value into any generated
  file or the lock and sets no variable, so the output stays byte-identical whatever account
  is chosen.
- **Layer format.** `account` is a new `layer.yaml` field. Unknown fields are a schema error
  (§5.1), so a layer that declares it needs the minor release that ships TASK-50.2, and §5.1
  lists the field from that release on. The new build error for layers that set the account
  variables (§7.9) can fail a build that passes today, so it needs a release note too. The
  user config, the allow state and the commands in §10 are new user-facing surface;
  generated output does not change, and the cache index lives in the cache.

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

Account selection (§7.13, planned) builds on this rule: because the build output does not
depend on the account, alz can choose each repository's account folder without changing what
its agents see. `alz account init` links the shared user-level configuration (instructions,
agents, commands, skills, plugins) so it is the same in every account folder; settings and
history stay per account unless the user shares them on purpose.

---

## 9. Safety rules

### 9.1 Never touch
Credentials, auth/token files, `.env*` contents, account config dirs and anything outside the
repo and the layer cache.

**Account folders are touched by name only.** `~/.claude`, `~/.claude.json`, `~/.codex` and
every `~/.claude-*` and `~/.codex-*` are read by `alz doctor` and, once built, by account
selection (§7.13). The only read operations allowed there are:
- listing `$HOME` for names that start with `.claude-` or `.codex-`;
- `lstat` and `readlink` of the fixed shared-item names (§7.13);
- listing `agents/`, `commands/` and `skills/` for the names they hold.

alz never opens or reads any other file there, so it never learns a login or whether a
repository is trusted (§7.13 "Trust is per account"). The only writes there are the planned
ones of `alz account init`, listed below.

**Planned writes outside the repo and the cache** (§7.13), each made only by an explicit
command, never by `build`, `check`, `alz run` or the shell hook:
- `alz account allow` writes `${XDG_STATE_HOME:-~/.local/state}/agent-layerz/allowed.yaml`
  (TASK-50.2) under an exclusive lock file next to it, through a temporary file in the same
  directory and a rename, with mode `0600` in a directory with mode `0700`.
- `alz account init <name>` creates the account folders for `<name>` with mode `0700`, links
  the shared items, and moves an item it replaces aside by renaming it in the same folder
  (TASK-50.5). It has a `--dry-run` that changes nothing, and it never copies a file.

alz also reads the hand-written user config
`${XDG_CONFIG_HOME:-~/.config}/agent-layerz/accounts.yaml`, which it never writes. Neither
command opens a file inside an account folder or touches a login. The cache index that
`alz build` writes for pinned `path:` layers (§7.13 "No fetch on resolve") lives in the layer
cache, so it is no exception.

### 9.2 Leak guard
For layers with `visibility: public`:
- Before `alz publish` or a commit on the layer repo, scan every file against the layer's
  `forbid_patterns`, a built-in secret scanner (gitleaks or equivalent) and a heuristic for
  private hostnames and emails.
- A public layer that `extends` a private one is an error (§3.1).

- A `commit_identity` (§7.12) in a public layer is public. Prefer a no-reply address (for
  GitHub, `<id>+<user>@users.noreply.github.com`); any other address raises the usual
  `leak-email` warning, and `forbid_patterns` apply to it like any other line.
- An `account` (§7.13, planned) in a public layer is public too: a public first layer that
  names a client's account publishes that client's name. Declare client accounts in the
  client layer; `forbid_patterns` apply to the field like any other line.

### 9.3 Security floor
Locked hooks (§7.5) and deny rules (§7.6) can't be removed by more specific layers. The
build fails, with a message naming the layer that tried.

### 9.4 Repository git identity
With a declared `commit_identity` (§7.12):

- **Effective identity** is what git would use for a commit in the repo root:
  `git var GIT_AUTHOR_IDENT` and `git var GIT_COMMITTER_IDENT`. Name and email are compared
  exactly with the declaration. An error from `git var` (no identity and
  `user.useConfigOnly`) counts as a mismatch.
- **Source of a mismatch**, found in git's own order: the environment variables
  `GIT_AUTHOR_NAME`, `GIT_AUTHOR_EMAIL`, `GIT_COMMITTER_NAME`, `GIT_COMMITTER_EMAIL`; then the
  config keys `author.*` or `committer.*`, then `user.*`, each located with
  `git config --show-scope --show-origin --get <key>`; then the `EMAIL` variable; otherwise
  "not set, git guessed it from the host".
- **Reports never echo the wrong value** (name or email). They name the layer that declared
  the identity, the key, the scope
  (`system`, `global`, `local`, `worktree`, `command`, or `environment`) and the source file,
  with the home directory shown as `~` and control characters removed. They may show the
  declared value, e.g. in the fix command `git config --local user.email '<email>'`, but only
  when the value is made of Unicode letters, marks, digits, space and `. - _ @ +`, so that the
  command means the same in POSIX shells, fish and PowerShell. For any other value (an
  apostrophe, say) the report prints no command and says to set the key to the value declared
  by the layer, which `alz build --adopt` adds when the key is unset. An identity that comes
  from the command scope (`GIT_CONFIG_COUNT`, `GIT_CONFIG_PARAMETERS`) is fixed by unsetting
  those variables, and one from an environment variable by unsetting that variable. Findings
  are printed one line per distinct source key in a fixed order (name, then email; author,
  then committer), so the output of a given setup is always the same.
- **Writing.** Only `alz init` (repo kind) and `alz build --adopt`, the setup commands, write.
  Plain `alz build` reports a mismatch as a warning and still succeeds; `alz check` reports it
  as an error and fails unless `--skip-identity` is passed. Writes:
  - use `git config --file <repo>/.git/config <key> <value>`, with the path checked by
    `insideRoot`; never `--global` or `--system`, never `~/.gitconfig` or any file outside
    the repo;
  - happen only when the repo root is the top level of its work tree and `.git` is a real
    directory (not a symlink, and not a gitfile as in linked worktrees and submodules).
    Also `.git/config` must be absent or a regular file, not a symlink, whose real path is
    inside the repository. Otherwise nothing is written and the report prints the commands to
    run by hand, followed by the findings of the effective identity (layer, scope, file). When
    the repo root is not the top level of its work tree, those commands would write to the
    outer repository, so the report says to run `git init` here first, or to run alz from the
    repository's top level, and prints findings without fix commands;
  - add `user.name` or `user.email` only when `.git/config` has no value for that key. An
    equal value is left alone. A different value is **never replaced**: it is reported as
    above. `.git/config` is not a generated file, and adding a missing key overwrites nothing
    (§4.3). A wrong global or system value needs no change, because the local key wins;
  - are followed by a fresh read of the effective identity, since an `[include]` in the local
    config or an environment variable can still override it; any remaining mismatch is
    reported. Each key written is printed with the declared value and the declaring layer.
- **Outside a git repository** (git itself says so) nothing is read or written, and init, build
  and check print a note instead. If git is missing or fails for another reason (a broken
  config file), the identity cannot be checked: `alz check` fails with an error, `build` and
  `init` warn. A `git config --get` failure other than "not set" reports the source as unknown.
- Identity subprocesses run through the existing git helper (no shell, `GIT_TERMINAL_PROMPT=0`)
  without the inherited `GIT_DIR`, `GIT_WORK_TREE`, `GIT_COMMON_DIR`, `GIT_CONFIG` and
  `GIT_INDEX_FILE`, so the environment cannot point them at another repository or file.

---

## 10. CLI

| Command | Does |
|---|---|
| `alz init [--stack <source>]… [--hub <dir>]` | Scaffold a repo manifest with these layers, then `.layer/`. `--hub` also adds the hub's `layer/`, pinned to its newest `layer-v*` tag. Then resolves the stack and, with a declared commit identity, sets the repo-local git identity (§9.4). A source that cannot be resolved yet only gets a note that `alz build --adopt` will do it. Any other problem with the stack or with a declaration (a schema error, a layer that may not declare) is printed as a warning, nothing is written, and `init` still exits 0 because the scaffold succeeded; `alz build` fails on the same problem. `--kind layer` scaffolds a new layer. |
| `alz build [--adopt]` | Resolve, merge, render, write, update the lock. Warns when the effective git identity differs from the declared commit identity (§9.4). `--adopt` also adopts hand-written config (§4.3) and sets the repo-local git identity. |
| `alz check [--skip-identity]` | Rebuild in memory and fail if the repo differs from it or from the lock (for CI and the Stop hook), or if the effective git identity differs from the declared commit identity (§9.4). `--skip-identity` skips the identity check, for CI runners and for people other than the owner. |
| `alz update [<layer>]` | Re-resolve refs to the newest matching versions and rebuild. |
| `alz ls [--live]` | List what the stack builds and the layer that decides each item. `--live` also asks Claude what it loaded (one short model call) and fails on any built item it did not load. |
| `alz explain <type> <name>` | Show which layers contributed to an item, in order, with the merged result. |
| `alz diff` | Show the effective change a pending build would make. |
| `alz doctor` | Check account dirs for collisions, target versions vs. adapter expectations, unfilled slots, budgets. *Planned (TASK-50.4):* per repository, a named account folder that does not exist, an account variable set outside alz, and shared-by-default items not linked in an account folder, by name only; items kept per account on purpose (`settings.json`, `projects/`, `sessions/`, `memories/`) are not flagged, and the blocked folders (`~/.claude-alz-blocked`, `~/.codex-alz-blocked`) are ignored. Prints a static notice that workspace trust is per account (§7.13, §9.1). |
| `alz lint [--layer <path>]` | Validate a layer: schema, leak guard, slot declarations. |
| `alz publish` | Lint, then tag `v<version>` on a layer repo (no push without `--push`). |
| `alz run <tool> [--account <name>] [args…]` | *Planned (TASK-50.3).* Start `claude` or `codex` on the account resolved for the working directory (§7.13), or with `--account <name>` for this run only; `--account default` uses the default account, and the `alz` shell function passes an `alz use` choice as `--account`. Arguments and exit code pass through unchanged. It fails closed before anything starts, exits non-zero and never uses the default account or a newly declared one when the tool or account is unknown, when the account's folder does not exist (it lists the folders that exist by name only), or when in an allowed repository a cache miss (it says to run `alz build`) or a changed declaration or source (it says to run `alz account allow`) skips the layer step. An account variable set outside alz is kept with a warning unless `--account` is given. Needs no shell hook and never fetches. |
| `alz use <tool> --account <name>`, `alz use <tool> --reset` | *Planned (TASK-50.6).* Set that tool's account for the current terminal until `--reset`; a `cd` does not undo it. The choice lives in unexported variables of the `alz` shell function, never in the environment. It fails closed as `alz run` does. Needs the hook from `alz activate`; without it, changes nothing, prints the `eval` line to add, and exits non-zero. |
| `alz activate zsh\|bash` | *Planned (TASK-50.6).* Print the shell hook for `eval "$(alz activate zsh)"`. It defines the `alz` shell function that `alz use` needs and, when loaded and at each prompt where the working directory changed, sets or unsets each target's account variable and its `ALZ_SET_*` marker (§7.13). When a named account cannot apply it warns and always sets the variable to the missing named folder, the recorded account (cache miss, same source) or the blocked folder, never to a newly declared account or the default account. Its evaluated output holds only `export NAME='value'` and `unset NAME` lines; messages go to standard error. |
| `alz account which` | *Planned (TASK-50.2).* Print the resolved account per target and its source (`use`, `environment`, `user config`, `layer`, `default`), the reason for each skipped step, and the paths of `accounts.yaml` and `allowed.yaml` (§7.13). When a named account fails closed it prints `layer step skipped (<reason>); fails closed`, what the hook applies (the missing folder, the recorded account or the blocked folder) and the fix command. Prints the `alz account allow` command only when an allowed entry has changed. Never fetches and writes nothing. |
| `alz account allow [--remove]` | *Planned (TASK-50.2).* Show the account this repository's layers name per target, the declaring layer, its source and its revision, and ask the person to confirm by typing the account name; then, under a lock file, record the repository's real root, the per-target map and the source in `allowed.yaml`, replacing any entry for that root (§7.13, §9.1). `--remove` removes this root's entry under the same lock file. Both refuse and change nothing without a terminal or inside an agent session; there is no `--yes`. On a cache miss it changes nothing and says to run `alz build`. Until it runs, and after the map or the source changes, a layer-named account does not apply. |
| `alz account init <name> [--share <item>]… [--dry-run]` | *Planned (TASK-50.5).* Create the account folders for `<name>` and link the shared-by-default items that exist in the default folder; `--share` also links `settings.json`, `projects`, `sessions` or `memories`, with a warning, and rejects any other item before planning (§7.13). Refuses the names `default`, `alz-blocked` and the configured `default_name`. `--dry-run` lists every folder it would create, every item it would link and every existing item it would move aside (rename in the same folder to `<item>.alz-backup-<UTC timestamp>`), and changes nothing. Otherwise it plans everything first, stops on any problem, and prints each backup's path. Rerunning it changes nothing. |

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
Items 10 to 21 are for account selection (§7.13) and must be recorded before TASK-50.2.
Account resolution also relies on the integrity of the layer cache, which is not a tool fact:
TASK-51 fixes it (cache entries checked against the lock, unambiguous git cache keys).

10. Claude Code: `CLAUDE_CONFIG_DIR`. That it is read only at start, so a running session
    keeps its account; where logins, workspace trust and per-project state live with it unset
    (`~/.claude`, `~/.claude.json`) and set (expected: `.claude.json` inside the folder); and
    that trust is per account folder. `docs/target-facts.md` §0.1 already shows that logins
    follow the variable (an empty folder answers `Not logged in`).
11. Codex: `CODEX_HOME`. That it is read only at start, and where logins, project trust and
    `config.toml` live.
12. Both, on macOS and on Linux separately: whether a login kept in the macOS Keychain or a
    Linux keyring is stored under an item keyed by the variable, so two account folders
    never share one login.
13. Both: whether a tool started with the variable set still reads the default folder
    (`~/.claude`, `~/.claude.json`, `~/.codex`) as a fallback.
14. Both: which shared items (§7.13) work as symbolic links, and whether the tools write
    through a linked item or replace the link with a file or folder.
15. Both: whether project settings (Claude `env`, Codex `shell_environment_policy`) can
    change the variable, and whether hooks, the shell tool and MCP servers inherit it.
16. Both: which settings or config keys carry credentials or pin the login, so they stay per
    account.
17. Both: where MCP OAuth tokens live.
18. Both: what each tool does when its variable points at a folder that does not exist
    (creates it, starts logged out, or refuses to start), and which file modes it creates
    there. This records the behaviour; it does not change §7.13, which sets the variable
    either way. If a tool creates the folder, a blocked folder (§7.13) can come to exist.
19. Both: which environment variables mark an agent session (in the tool's shell commands,
    hooks and MCP servers), so `alz account allow` can refuse to run inside one.
20. mise and direnv: whether each leaves a variable that shows which variables it set, so a
    warning about a value set outside alz can name it and alz can tell when one of them set
    an `XDG_*` variable.
21. Both: whether Claude Code and Codex permission or sandbox rules can deny reads and writes
    to `~`-relative paths (such as `~/.config/agent-layerz/**`), and whether a Bash deny rule
    such as `bash:alz account allow*` can be bypassed through `sh -c` or another wrapper.

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
18. **Commit identity** (§7.12, §9.4): in a git repo whose global config holds a wrong
    identity (reserved domains only, e.g. `ci@example.invalid`), `alz build --adopt` adds the
    declared identity to `.git/config` and leaves the global file byte-identical; a second
    build changes nothing and the output stays byte-identical. A private `kind: client`
    layer outside the repo overrides the personal identity (e.g. with `owner@client.example`),
    and the Commits section and reports name that layer. A declaration in the repo layer
    (even with `kind: client`), a group layer, a public `client` layer, a `path:` symlink
    inside the repo that points outside it, or a second client layer fails the build and
    names the layer. `alz check` fails on a wrong global, local or environment identity,
    naming only the layer, the scope and the source; no output contains the wrong value. A
    local value that differs is never replaced. An identity with a newline, control character
    or leading `-`, a name that git would trim (edge `.` `,` `;` and the like), a backslash or
    an invisible format character fails before any git command. A `git:` local path or
    `file://` layer, a case variant of an in-repo path and a link into the repo count as inside
    it. A `.git/config` that is a symlink, a gitfile `.git`, or a repo root inside another work
    tree gets no write; the report still names the source of a wrong value, and a name with an
    apostrophe gets no shell command. Without a declaration, `AGENTS.md` has no Commits
    section.

Tests 19 to 24 are planned with account selection (§7.13); each names its task. The numbers
in this list are not file numbers: 19 to 23 become new test files with the next free `NN`,
and 24 adds cases to the existing `test/acceptance/24-untrusted-repo.test.ts`. All use a fake
`HOME`, `XDG_CONFIG_HOME`, `XDG_STATE_HOME` and `XDG_CACHE_HOME`, never a real account folder.

19. **Account resolution** (TASK-50.2): `alz account which` reports each source per target
    (`use`, `environment`, `user config`, `layer`, `default`) and the paths of both files. A
    `folders` entry matches whole path segments and the longest one wins. A symlinked path, a
    case variant on a case-insensitive file system and a nested stack manifest each resolve
    to the right root, and a `layer.yaml` without `stack` is skipped. A missing named account
    folder fails closed; a missing default folder never fails. A user config inside the
    repository, writable by group or others, or under a parent directory below `$HOME` that
    group or others can write is ignored with a warning, and so is an `allowed.yaml` entry
    with a relative root or an account name that breaks the rule. Build output is
    byte-identical with and without account settings.
20. **Account allow** (TASK-50.2): in a fresh clone whose stack names an account, tools start
    on the default account; `alz build` on a terminal prints the allow command, and prints
    nothing about it with an agent-session marker set. `alz account allow` without a
    terminal, or with an agent-session marker set, refuses and writes nothing; on a terminal
    it accepts only the typed account name. After `allow` the named account applies, and a
    ref bump of the declaring layer keeps it. A changed per-target map (a target added or
    removed included) or declaring source skips step 4 for every target, and
    `alz account which` warns with the old and new values. When a named account fails
    closed, `alz account which` prints `layer step skipped (<reason>); fails closed`, what
    the hook applies and the fix command. A pinned `path:` layer resolves through the cache
    index; a lock `sha` that is not 40 or 64 hex characters, an index entry for another path,
    a malformed index entry, a symlinked index entry, and a layer whose content hash differs
    from the lock are each a cache miss, and the hint says only to run `alz build`. In a
    repository that is not allowed, resolution reads no file of its stack. No resolution
    command runs git. Two concurrent `allow` runs leave one sorted entry per root, and
    `alz account allow --remove` removes only this root's entry.
21. **`alz run`** (TASK-50.3): for `claude` and `codex`, with and without `--account`,
    arguments and the exit code pass through. An unknown tool or account, a missing named
    folder whether named or resolved, and a cache miss, a changed declaration or a recorded
    source no longer in the stack in an allowed repository each fail before anything starts,
    never on the default account, and name the fix. A variable set outside alz is kept with a
    warning unless `--account` is given. A forged `ALZ_USE_CLAUDE` in the environment changes
    nothing.
22. **Shell hook** (TASK-50.6): in zsh and bash, the hook sets the variables and markers after
    a `cd` into a configured repository and unsets them on leaving; `alz use` survives a `cd`
    until `--reset` and exports no variable; a forged `ALZ_USE_CLAUDE` changes nothing; a
    foreign value is kept with one warning. When a named account cannot apply, the hook warns
    and sets the missing named folder, the recorded account (cache miss, same source) or the
    blocked folder; a newly declared account or the default account is never set. The
    evaluated output holds only `export NAME='…'` and `unset NAME` lines. A directory named
    `$(touch pwned)` and a layer name containing ESC create no file and put no control
    character on the terminal.
23. **Account folders by name only** (TASK-50.4, TASK-50.5): a dry run of
    `alz account init <name>` changes nothing. A real run creates `0700` folders, links only
    shared-by-default items that exist in the default folder, and backs up by renaming. It
    refuses `default`, `alz-blocked`, the configured `default_name`, a symlinked account
    folder or parent, and a `--share` item other than `settings.json`, `projects`, `sessions`
    or `memories` (before planning), and a rerun changes nothing. `alz doctor` reports
    missing links but not items kept per account, and ignores the blocked folders. A
    file-access trace shows that neither command opens a file inside an account folder.
24. **Untrusted repository**, account cases (TASK-50.2): a repo layer, a group layer or a
    public `client` layer that declares `account` fails the build, and `alz run`, the hook
    and `alz account which` ignore it with one warning. A layer that sets `CLAUDE_CONFIG_DIR`,
    `CODEX_HOME`, `HOME`, `XDG_CONFIG_HOME`, `XDG_STATE_HOME` or `XDG_CACHE_HOME` through
    `settings.yaml`, an MCP environment or a passthrough file fails the build and names the
    layer. A stack that names an account does not apply before `alz account allow`.

---

## 14. Rollout (generic, for any client)

1. **Personal layer:** extract universal standards, generic agent personas (architect,
   designer, domain-expert, backend-engineer, frontend-engineer, reviewer, …), generic
   skills, security-floor hooks and denies, and model aliases. Declare your
   `commit_identity` (§7.12). Mark it `public` and run `lint`.
2. **Client hub + layer:** for each client, create the private hub repo at `<client-root>/hub/`
   (§4.4). Put the domain context, constraints, persona overlays and client-wide slot fills
   in `hub/layer/` (`kind: client`, private), plus a `commit_identity` if the client issues
   you an email, and tag it `layer-v0.1.0`. Put the docs, specs and Backlog.md board in the
   rest of the hub.
3. **Repo layer:** in each repo, run `alz init`, add the stack-specific slot fills
   (`stack-conventions`, …), set the MCP allowlist and budgets, then `alz build`.
4. **Clean the account dirs:** move agents and skills out of `~/.claude*` and `~/.codex*`
   into the right layer until `alz doctor` is clean.
5. **CI:** add `alz check --skip-identity` to each repo's pipeline (a CI runner has no owner
   identity), and plain `alz check` to the agent's Stop hook if wanted.
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
