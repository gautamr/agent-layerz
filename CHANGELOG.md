# Changelog

All notable changes to agent-layerz are recorded here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and versions follow
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed

- `alz` is one Rust binary with no runtime. Until the first Rust release is published, build it
  from a clone with `mise run install:local` (README, Install). 0.1.0 was installed from a
  package registry and leaves a symlink named `alz` on the `PATH`, which `mise run install:local`
  will not replace: remove that install first (README, "Moving from 0.1.0").
- The Rust `alz` checks layer files more strictly than 0.1.0, as
  `docs/LAYERING-SPEC.md` §5.2, §7.2, §7.7, §7.8, §7.9, §9.2 and §10.1 say (TASK-55.5). A layer
  that 0.1.0 accepts can fail when the repository moves to the Rust `alz`:
  - The top level of `settings.yaml` takes only `claude` and `codex`. Any other key is an error
    and the file's settings are not merged.
  - Keys of `mcp.yaml` and entries of `mcp.enable` must be server ids: an ASCII letter or digit,
    then up to 63 letters, digits, `_` or `-`. Each alias in `models.yaml` maps only `claude`
    and `codex`.
  - An agent overlay narrows a tool only when the base covers it segment by segment
    (`docs/**` covers `docs/adr/**`, while `docs*` covers only `docs*`), and a narrowed `bash` scope
    may not add a shell operator such as `;`, `&&`, `|` or `$(`.
  - `forbid_patterns` use the `regex-lite` syntax. `\d`, `\w` and `\b` are ASCII only, and
    look-around, backreferences and `\p{…}` are refused. Each layer has at most 64 entries of
    at most 512 characters each.
  - Array items in `settings.yaml` are de-duplicated by value, so two mappings that differ only
    in key order become one.
  - Markdown frontmatter closes only at a line that is exactly `---`, and a Markdown file
    without frontmatter must be valid UTF-8.
  - A folder in `targets/` must be named after a known target (`claude`, `codex`).
  - Codex keys that duplicate a first-class item are refused at any depth. That covers `hooks`
    and `codex_hooks` in any `features` table. `profile` (removed by Codex 0.169.0) and
    `profiles` as a whole are refused too, because a project's Codex config ignores them.
  - An agent overlay may only make an agent stricter. Its `permission_mode` may not be looser
    than that of the agent it refines or than the permissions floor, and it may not take an
    entry out of `claude.disallowedTools`.
  - A tool path scope that starts with `~` or `/`, holds a backslash or starts with a drive
    letter is covered only by an equal scope.
  - Security-relevant native settings (`docs/LAYERING-SPEC.md` §7.9) may only get stricter down
    the stack: sandbox switches, allow lists that may gain no entries, and deny lists that only
    grow. Reserved keys are refused in `settings.yaml`: plugin and marketplace sources, keys that
    run a command (such as `awsAuthRefresh` and `otelHeadersHelper`) or route traffic, keys meant
    for user or managed settings (including Codex `windows`), and `mcpServers`, which Claude Code
    does not document as a settings key. Codex `tools.web_search` is a boolean or a table, so it
    is no longer an ordered mode; its `allowed_domains` may not gain entries.
  - An agent's native fields may not hold `claude.mcpServers`, `codex.mcp_servers` or `hooks`.
  - Under `claude:`, `settings.yaml` accepts only the keys that `docs/target-facts.md` §14 records
    with a type, and each value must have that type. An unknown key at any depth, a key meant for
    a user or managed scope, and a value of the wrong type are errors that name the file and the
    key path and print no value (TASK-79). An agent's `claude:` fields get the same check for the
    subagent frontmatter keys: `disallowedTools`, `maxTurns`, `autoCompactWindow`, `background`,
    `omitClaudeMd`, `effort`, `color`, `initialPrompt` and `experimental.cacheTtl`.
  - `permissions.yaml` has the floor fields `disable_bypass_mode`, `block_reads_outside` and
    `block_writes_outside`. Once a layer sets one to `true`, a more specific layer cannot set it
    back to `false`.
  - The leak guard decodes UTF-16 files (with a byte order mark) and removes NULs before it
    checks. `^` and `$` in `forbid_patterns` match at every line, and the secret scan reads the
    whole `layer.yaml`.

  These are breaking changes for some layers. The cutover to the Rust `alz` is a minor version
  bump (0.x).

- The Rust `alz` resolves layer sources more strictly than 0.1.0, as
  `docs/LAYERING-SPEC.md` §6 ("The layer cache", "Layers in or around an account folder", "Git
  subprocesses") and §9.1 say (TASK-52, TASK-53, TASK-63, TASK-70). What changes when a
  repository moves to the Rust `alz`:
  - Git runs without a terminal, so `ssh` cannot ask for a key's passphrase or confirm a new
    host key. Add the key to an `ssh` agent and the host to `known_hosts` first. A graphical
    askpass program still works.
  - Git 2.30 or later is required.
  - A clone that runs longer than 10 minutes fails, and so does any other git call that runs
    longer than 2 minutes.
  - For its cache and clone calls, alz run from a git hook ignores `GIT_DIR`, `GIT_INDEX_FILE`
    and the other variables that point git at a repository, and the `-c` options of the git
    that started the hook.
  - An empty or relative `XDG_CACHE_HOME` is ignored, and an absolute one that puts the cache
    inside the repository is refused. Missing cache folders are created with mode `0700`.
  - A layer folder or cache folder that is the home folder or above it, lies inside an account
    folder or holds one is refused.

  The layer format, the lock format and the generated files do not change.
- The Rust `alz` writes generated files and adopts native files more carefully than
  0.1.0, as `docs/LAYERING-SPEC.md` §4.3, §4.5, §5.3, §7.11, §8 and §10.3 say
  (TASK-55.6). What changes when a repository moves to the Rust `alz`:
  - A file counts as generated only when it carries the generated mark, or when `layer.lock` lists
    it and its bytes still match the hash recorded there. alz never overwrites or deletes anything
    else. A generated file without a header that was edited by hand, such as
    `.claude/settings.json`, is a collision at the next build; `alz build --adopt` takes it into
    the repo layer.
  - alz never writes through a link. A generated path that is a link, or lies under one, is a
    collision.
  - Paths are compared ignoring ASCII case, accents included (`É` and `é` match), as on macOS
    and Windows. A passthrough file such as
    `targets/claude/.claude/Settings.json` collides with the generated `.claude/settings.json`,
    and `protected_paths` match in any case.
  - A layer file or folder name with a control or format character is refused, and so is a
    source of a generated header (a layer version, ref or path) that holds such a character or
    `-->`. In `protected_paths`, `?` matches one character, also one above U+FFFF.
  - `alz build --adopt` refuses text the layer reader would refuse before it writes anything. It
    reads JSON numbers exactly, and it changes `layer.yaml` only where it adds entries, keeping
    every other byte.
  - `alz diff` prints its own unified diff.
  - The messages of `build`, `check`, `ls`, `explain`, `doctor` and `init` follow §10.3.

  Generated files stay byte-identical to 0.1.0's for the same layers, except for
  the changes listed in this section. These are breaking changes for some repositories, part of the
  minor version bump at the cutover.
- The Rust `alz` sets and checks the commit identity and has `alz account which`, `alz account
  allow` and `alz account allow --remove`, as `docs/LAYERING-SPEC.md` §7.13, §9.4, §10.1 and
  §10.3 say (TASK-55.7). No generated file changes. Where it differs from 0.1.0:
  - `allowed.yaml` is written so that it reads back for every repository root, also one whose
    path holds a C1 control character, U+2028 or U+2029.
  - The git calls of the identity step ignore every variable that points git at another
    repository (`GIT_OBJECT_DIRECTORY`, `GIT_NAMESPACE`, `GIT_CEILING_DIRECTORIES` and the
    others §9.4 lists), not only five of them. alz checks `.git` and `.git/config` right before
    and right after git writes the identity there, and fails with an error when it finds a link
    there afterwards.
  - Account resolution never reads a name inside an account folder through a link planted in
    the repository. With `HOME` unset it uses the home folder of the real user id, and when the
    user database has none it fails closed instead of crashing.
  - Ctrl-C at the `alz account allow` prompt answers no: `not confirmed; nothing changed`, exit 1.
  - `alz doctor` does not list the blocked folders as accounts and warns when one exists. It no
    longer reads `ALZ_ACCOUNT_DIRS`.

  These are part of the minor version bump at the cutover.
- The Rust `alz` sorts names, paths and keys by Unicode code point everywhere, as
  `docs/LAYERING-SPEC.md` (§6, name order) says (TASK-64). 0.1.0 sorts by UTF-16
  unit, which puts a character above U+FFFF, such as most emoji, before one in U+E000–U+FFFF:
  private use, compatibility ideographs, presentation forms, variation selectors, half-width and
  full-width forms, and U+FFFD. Its JSON output also writes array-index keys (whole numbers
  without leading zeros below 2^32 − 1, such as `9` and `10`) before all other keys and in
  numeric order. What changes when a repository moves to the Rust `alz`:
  - A layer with two file paths whose first difference is such a pair gets a new content hash.
    If alz then reports that a cached layer changed, delete the cache directory it names and run
    `alz build`. For a `path:` layer, run `alz build`.
  - Generated files can change order: `AGENTS.md` blocks from instruction files with such
    names, and keys in `.claude/settings.json`, `.mcp.json` and `layer.lock` that contain such
    characters or are array-index keys (`10` now sorts before `9`). `alz check` reports this
    until you run `alz build`.
  - `alz account allow` writes entries of `allowed.yaml` whose roots differ this way in the new
    order the next time it runs.
  - A name or key that holds a lone surrogate is an error.

  The lock format does not change.
- More differences from 0.1.0, as `docs/LAYERING-SPEC.md` states them (TASK-55.9.4):
  - In layer YAML, tags such as `!foo` and `!!binary` are the error `yaml-tag`, and a second
    document in one file is an error.
  - A field that expects text and holds a number is an error that suggests quoting it:
    `version: 1.10` and `ref: 1.10` are the number 1.1, so write `version: "1.10"` and
    `ref: "1.10"`. A `ref` that is a list or a mapping is an error too, and a wrong `account`
    field is a schema error of `layer.yaml`.
  - A number in a typed Claude settings key or agent field must be finite and inside the range
    Claude Code documents.
  - A passthrough file's nested `env` mappings are checked down to 128 levels, so a file nested
    deeper than 32 levels is no longer refused for its depth.
  - `alz ls` lists skills and commands in name order. `alz explain` with a name the merged result
    does not have prints `error [explain] …`.
  - `alz build --adopt` does not adopt an agent whose frontmatter holds `hooks`, and it moves the
    scripts of `.claude/hooks/` to `.layer/hooks/scripts/`; the adopted hooks run them with
    `run: scripts/<path>`.
  - A refused `layer.lock`, cache entry or pinned tree gets the messages of §5.3 and §6.
  - Generated YAML quotes text that alz reads back as an integer, such as `0b1`, `-0x1` and
    `+0o7`, and text that starts with U+FEFF or is made only of spaces and line feeds and holds
    both. It writes U+007F to U+009F, U+2028 and U+2029 as escapes, and a space between two line
    feeds of long double-quoted text as one escaped space. Generated agents, rules and skills and
    adopted layer files change only where such text occurs.
  - When `alz init`, `alz build` or `alz check` may not write the git identity, the steps to run by
    hand are on the warning's line, separated by `;`, and a `.git/config` that is a link gets the
    reason `.git/config is not a regular file`.

  The lock format does not change. These are part of the minor version bump at the cutover.
