---
name: acceptance-baseline-run
description: How to find which acceptance cases a port phase can reach: run the suite files against the Rust release binary with ALZ_BIN and count pass/fail per file and command
metadata:
  type: reference
---

To scope the "cases that alz <command> can reach" criterion of a port phase, run a baseline before planning. Do not reason it out from the test sources.

```
ALZ_BIN=$PWD/target/release/alz npm exec --no -- vitest run --project acceptance test/acceptance/{02,03}-*.test.ts --reporter=verbose > <scratchpad>/baseline.txt 2>&1
grep -cE '^\s+✓' …; grep -E '^\s+×' … | grep -i lint
```

The 15 merge files (tests 02-11, 17-19, 26, 29) took about a minute. On 2026-11-23 (R4 plan, 6c5c9fe) 84 of 199 cases passed. The only failing cases that drive lint were test 10's leak guard and secret scan. Every merge-rule case drives `build`.

Why lint reaches so little: the reference's `alz lint` runs every merge loader on each layer alone and drops the `unresolved` code (src/cli/commands/lint.ts). So no cross-layer verdict reaches lint.

**How to apply:** check that `target/release/alz` is current with `git log -1` against the binary's mtime, or run `mise run rust:build` first. Put the output in the scratchpad, never in the account folder (see [[fact-checks-without-account-folders]]). Record the counts per file in the phase task's notes.
