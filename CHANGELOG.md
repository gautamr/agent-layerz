# Changelog

All notable changes to agent-layerz are recorded here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and versions follow
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed

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
