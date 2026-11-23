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
- `protected_paths` and `mcp.enable` are lists of text.
- `budgets.instructions_max_lines`, `budgets.instructions_max_bytes` (32768 when absent, Codex's
  `project_doc_max_bytes`) and `budgets.always_loaded_skills_max` are whole numbers, 0 or more. A
  float without a fractional part, such as `2.0` or `1e3`, counts as a whole number (decision-28).
- `options.commit_generated` and `options.strict_slots` are `true` or `false`, and `true` when
  absent.
- `mcp`, `workspace`, `budgets` and `options` are mappings with only the fields above.

**Credentials in a source** (TASK-65). A `git:` URL must not hold credentials:
- an `http://` or `https://` URL with a user name or password (`https://user@…`,
  `https://token@…`, `https://user:token@…`) is refused;
- any other URL, or a scp-like address (`user@host:path`), whose user part holds a `:` (a
  password) is refused.

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
UTF-8 and parsed as one JSON document nested at most 128 levels deep. A duplicate key in any object
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

The order of the checks is fixed, so that the reason never depends on the order in which the
file system lists names. Account resolution walks the layer, stopping at entry 2001 (§7.13 "No
fetch on resolve"). It then checks, and the first that fails decides the miss: too many entries
(`unreadable`), a name that is not valid Unicode (`layer-invalid`), reading each file within 16
MiB in all (`unreadable`), the content hash (`content-hash`), a `.env` file (`layer-invalid`),
then `layer.yaml`. The entry limit comes first so that the walk never goes past entry 2001. In
the build the name check comes first, before the content hash is computed or compared (below),
`layer.yaml` is read or any `.env` file is reported.

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
is set. The message never contains file content. Nothing is compared when the lock has no entry
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
   `$XDG_CACHE_HOME/agent-layerz` (default `~/.cache/agent-layerz`). A `github:` or `git:` layer
   is the directory `git-<sha256 of what is cloned, in hex>@<commit sha>`, where what is cloned
   is the remote URL exactly as it is passed to git (for `github:owner/repo`,
   `https://github.com/owner/repo.git`), or for a local repository its canonical absolute path.
   The URL is not rewritten, so two different URLs never share an entry and the same text always
   maps to the same one. An entry from an older version with a different name is ignored and
   fetched again, and is never removed. A cached git entry that already exists is not trusted over
   a fresh clone: when the lock has no entry to check it against, the build clones the commit,
   compares the two content hashes, discards the clone and stops with the "delete that directory"
   error on a difference. A cache entry must be a real directory, never a link.

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
   prompt: alz runs them so that they cannot reach the terminal, because alz could neither see
   nor sanitise what they write there. A key with a passphrase therefore has to be in an agent,
   and a host whose key is not yet known fails the fetch with the message above (TASK-70; the
   TypeScript `alz` leaves `ssh` able to prompt, reference drift).

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

   A pinned `path:` layer is `tree2@<git tree id>`, where the tree is the layer's directory at the
   commit. The prefix is the format version: entries named `tree@<id>`, written by older versions
   with `git checkout-index`, may differ from the tree by attribute conversions, so they are
   ignored and written again (and never removed). Reading the tree:

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
     refused.
   - A tree with two paths that are equal after Unicode NFC normalisation and case folding, or a
     file and a folder with such names, is refused before anything is written, because one file
     system would merge them.

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

No layer may set `CLAUDE_CONFIG_DIR`, `CODEX_HOME`, `HOME`, `XDG_CONFIG_HOME`,
`XDG_STATE_HOME` or `XDG_CACHE_HOME`, which choose the account folder or locate alz's own
files and the layer cache (§7.13). Each of these is a build error (code `protected-variable`)
that names the layer, the file, a key path chosen by alz and, for a protected name, the
variable, and never the text the layer wrote. A variable name here is a plain name,
`^[A-Za-z_][A-Za-z0-9_]*$`; a key or entry that is anything else (`NAME=x`, a NUL, a space) is
refused, because a spawn would read `CODEX_HOME=/x` as the name `CODEX_HOME` with a value.
Names compare ignoring ASCII case. An `env`, `set` or pattern list of another type cannot be
checked and is refused.

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
  (`layer-directory-missing`), never a layer to skip. It reads from:
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
  Each file is read once, never through a link, and the bytes hashed are the bytes parsed. A
  layer with more than 2000 entries, or more than 16 MiB in all, is a cache miss
  (`unreadable`); `alz build` has no such limit. The entries are the names the walk lists in
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
  invalid YAML. The YAML alias limit is four budgets:
  - at most 100 aliases (`*name`) in a document, counted as they are written in the text, not
    as they expand;
  - at most 64 mappings and sequences nested inside one another: a document that is one scalar
    has none, and `a: [x]` has two. Depth is counted in the tree as aliases expand it, so an
    alias at depth d of a node h levels deep reaches d + h;
  - at most 20000 nodes (scalars, aliases, mappings and sequences) in a document;
  - at most 10000 parse events replayed through aliases. Each scalar, each alias and the start
    and end of each mapping or sequence is one event, so each use of an alias of a large node
    costs that node's size. An alias inside that node costs one event plus the size of its own
    anchor's node, each time the outer node is replayed.

  decision-28 names the serde-saphyr options, not how they count. R2 (TASK-55.3) sets them so
  that these four hold and tests each boundary.

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
resolve") lives in the layer cache, so it is no exception. Account resolution treats a
repository root, cache or layer directory that lies inside an account folder, or contains
one, as a cache miss and opens nothing in it.

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

- `<level>` is `error` or `warning`.
- `<code>` is a short kebab-case name that the spec gives, for example `layer`, `manifest`,
  `no-manifest`, `lock`, `yaml`, `yaml-tag`, `source` or `cache`. The code is part of the
  contract. The message is part of it only where the spec quotes it.
- `<where>` locates the problem:
  - a file of a layer is `<layer>:<path>`, where `<layer>` is the layer's name, or, while the name
    is not known (its `layer.yaml` did not parse), the layer directory or stack entry as it was
    given;
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
the linted layers' names joined by `, `. A layer whose name is not known appears as its directory
as it was given, stripped.

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
