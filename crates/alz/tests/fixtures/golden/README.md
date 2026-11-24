The files the TypeScript reference's `alz build` writes for every repository fixture of the
acceptance suite (`test/fixtures/<fixture>/<repo>`) that builds, made once by
`node crates/alz/tools/golden-trees.mjs` in a scratch copy of the whole fixture with an isolated
home folder (TASK-55.6.8). For `<fixture>/<repo>`:

- `files/` holds every file the build created, with its bytes and executable bit (git keeps the
  bit); the repository's own inputs are not copied, so the test reads them from `test/fixtures`.
- `stdout` and `stderr` hold what the build printed.

`16-hub` builds against a hub that is a git repository tagged `layer-v0.1.0`, made with a fixed
author, committer and date so the commit id in `layer.lock` is the same on every run; the script
and `crates/alz/tests/golden_trees.rs` make it the same way.

The test compares the paths, bytes and executable bits, `stdout` byte for byte, and `stderr` by the
level and code of each line. The place and wording of a diagnostic differ where the specification
words them differently from the reference (a `slot-missing` has a place, §7.3; the leak guard
reports once per file, §9.2). Left out:

- A repository whose build fails in the reference, because an acceptance case asserts the
  failure, not a tree.
- `04-tool-narrowing/repo-widen-command-operator` and `repo-widen-path-scope`: the reference
  builds them and the Rust `alz` refuses the overlay (`tool-widen`, §7.2), which is reference
  drift.
