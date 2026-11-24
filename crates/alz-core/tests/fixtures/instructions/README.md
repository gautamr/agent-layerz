The `AGENTS.md` and `CLAUDE.md` that `alz build` writes (spec §7.1, §10.3) for the repositories of
the acceptance fixtures `01-determinism` (repo), `16-hub` (repo-a, repo-b, the hub tagged
`layer-v0.1.0`) and `26-commit-identity` (repo) (TASK-55.6.3). Fixture `02-instructions` holds
its own expected files. The tests of `src/render/instructions/tests.rs` render each fixture's
merged stack and compare byte for byte.

How it is kept: by hand, with no other tool. A change to a rule of §7.1 changes the files it
touches in the same commit; the tests print the file that differs. For a new fixture, run
`alz build` in a scratch copy of the fixture, read every file against the spec, and only then
copy it here.
