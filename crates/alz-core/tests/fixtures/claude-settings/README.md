The `.claude/settings.json`, `.mcp.json` and `.claude/hooks/**` that the `claude` adapter writes
(spec §8.1) for the repositories of the acceptance fixtures `01-determinism` (repo),
`07-permission-floor` (repo-combine), `08-locked-hook` (repo-unlocked), `09-mcp-allowlist` (repo),
`16-hub` (repo-a, the hub tagged `layer-v0.1.0`) and `19-hook-commands` (repo) (TASK-55.6.5).

`mix` is a stack written for this test, because the fixtures use few of the output's keys: its
`personal` layer and manifest are in `mix/input/`, and the expected output in the other folders of
`mix/`. The hook script `guard.sh` there has the executable bit and `fmt.sh` has not.

No golden holds the floor fields: the unit tests of
`src/target/claude/settings/tests.rs` state them from spec §7.6 and target-facts §13.4.

How it is kept: by hand, with no other tool. A change to a rule of §8.1 changes the files it
touches in the same commit; the tests print the file that differs. For a new fixture, run
`alz build` in a scratch copy of the fixture, read every file against the spec, and only then
copy it here.
