The `AGENTS.md` and `CLAUDE.md` the TypeScript reference writes for the repositories of the
acceptance fixtures `01-determinism` (repo), `16-hub` (repo-a, repo-b, the hub tagged
`layer-v0.1.0`) and `26-commit-identity` (repo), made once with `alz build` (TASK-55.6.3).
Fixture `02-instructions` holds its own expected files. The tests of
`src/render/instructions/tests.rs` render each fixture's merged stack and compare byte for byte.
