# Changelog

All notable changes to agent-layerz are recorded here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and versions follow
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed

- The Rust `alz` checks layer files more strictly than the TypeScript `alz`, as
  `docs/LAYERING-SPEC.md` §5.2, §7.2, §7.7, §7.8, §7.9, §9.2 and §10.1 say (TASK-55.5). A layer
  that the TypeScript `alz` accepts can fail when the repository moves to the Rust `alz`:
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
    and `codex_hooks` in any `features` table, `approval_policy` and `sandbox_mode` in a
    profile, and `profile` itself.
  - An agent overlay may only make an agent stricter. Its `permission_mode` may not be looser
    than that of the agent it refines or than the permissions floor, and it may not take an
    entry out of `claude.disallowedTools`.
  - A tool path scope that starts with `~` or `/`, holds a backslash or starts with a drive
    letter is covered only by an equal scope.
  - Security-relevant native settings (`docs/LAYERING-SPEC.md` §7.9) may only get stricter down
    the stack: sandbox switches, allow lists that may gain no entries, and deny lists that only
    grow. Reserved keys are refused in `settings.yaml`: plugin and marketplace sources, keys that
    run a command or route traffic, and keys meant for user or managed settings.
  - An agent's native fields may not hold `claude.mcpServers`, `codex.mcp_servers` or `hooks`.
  - `permissions.yaml` has the floor fields `disable_bypass_mode`, `block_reads_outside` and
    `block_writes_outside`. Once a layer sets one to `true`, a more specific layer cannot set it
    back to `false`.
  - The leak guard decodes UTF-16 files (with a byte order mark) and removes NULs before it
    checks. `^` and `$` in `forbid_patterns` match at every line, and the secret scan reads the
    whole `layer.yaml`.

  These are breaking changes for some layers. The cutover to the Rust `alz` is a minor version
  bump (0.x).

- The Rust `alz` resolves layer sources more strictly than the TypeScript `alz`, as
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
- The Rust `alz` sorts names, paths and keys by Unicode code point everywhere, as
  `docs/LAYERING-SPEC.md` (§6, name order) says (TASK-64). The TypeScript `alz` sorts by UTF-16
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
