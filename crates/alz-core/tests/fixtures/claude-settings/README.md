The `.claude/settings.json`, `.mcp.json` and `.claude/hooks/**` that the TypeScript reference
writes for the repositories of the acceptance fixtures `01-determinism` (repo), `07-permission-floor`
(repo-combine), `08-locked-hook` (repo-unlocked), `09-mcp-allowlist` (repo), `16-hub` (repo-a, the
hub tagged `layer-v0.1.0`) and `19-hook-commands` (repo), made once with `alz build` in a scratch
copy of each fixture (TASK-55.6.5).

`mix` is a stack written for this test, because the fixtures use few of the output's keys: its
`personal` layer and manifest are in `mix/input/`, and the reference's output in the other
folders of `mix/`. The hook script `guard.sh` there has the executable bit and `fmt.sh` has not.

The reference has no floor fields (TASK-55.5.16, reference drift), so no golden holds them: the
unit tests of `src/target/claude/settings/tests.rs` state them from spec §7.6 and target-facts §13.4.
