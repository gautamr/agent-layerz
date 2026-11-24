The files `alz build` writes for every repository fixture of the acceptance suite
(`fixtures/<fixture>/<repo>`) that builds (spec §4.3 repo-level files, §10.3 output of build;
TASK-55.6.8). For `<fixture>/<repo>`:

- `files/` holds every file the build created, with its bytes and executable bit (git keeps the
  bit); the repository's own inputs are not copied, so the test reads them from `fixtures`.
- `stdout` and `stderr` hold what the build printed.

`16-hub` builds against a hub that is a git repository tagged `layer-v0.1.0`, made with a fixed
author, committer and date so the commit id in `layer.lock` is the same on every run;
`crates/alz/tests/golden_trees.rs` makes it the same way.

The test compares the paths, bytes and executable bits, `stdout` byte for byte, and `stderr` by the
level and code of each line (the place and wording of a diagnostic are tested where it is made).
Left out:

- A repository whose build fails, because an acceptance case asserts the failure, not a tree.
- `04-tool-narrowing/repo-widen-command-operator` and `repo-widen-path-scope`: alz refuses the
  overlay (`tool-widen`, §7.2), so there is no tree.

How it is kept: by hand, with no other tool. A change to a rule of the spec changes the files it
touches in the same commit; the test prints the first file that differs. For a new fixture
repository, run `alz build` in a scratch copy of the whole fixture with an isolated home folder,
read every file against the spec, and only then copy `files/`, `stdout` and `stderr` here.
