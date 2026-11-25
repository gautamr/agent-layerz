---
name: r6-planning-facts
description: Non-obvious facts found planning R6 (TASK-55.7): git follows a linked .git/config, std home_dir uses getuid, rustix 1.1.6 has termios/pty, what R5 left out of init and resolution
metadata:
  type: reference
---

Found while planning R6 (TASK-55.7, 2026-11-24). Check against current code before relying on them.

- `git config --file <repo>/.git/config <key> <v>` follows a link at that file (writes the target, keeps the
  link); recorded in target-facts §11 the same day. So §9.4's "not a symlink" check is alz's own and has a
  race window (owner question 1 in TASK-55.7).
- `std::env::home_dir` in the pinned 1.100.0 std (`library/std/src/sys/paths/unix.rs`) uses HOME when set and
  non-empty, else `getpwuid_r(getuid())`; the spec says geteuid. alz reads HOME itself first, so only an
  unset HOME reaches std. No new crate needed for TASK-72.
- rustix 1.1.6 has features `termios` (isatty, tcsetattr, `SpecialCodeIndex::VEOL`) and `pty` (openpt,
  grantpt, ptsname); `process` (already on) has getuid/geteuid. deny.toml lets only alz-io and tempfile
  depend on rustix.
- The Rust `alz init` (R5) only scaffolds: it does not resolve the stack afterwards, so its stack,
  account and identity warnings are all R6.
- No cache-only declaration reader exists in Rust (the TS `cached-stack.ts`); `layer/scan` already has
  `Purpose::Resolution` budgets and R3 writes the `pathref@` index.
- The reference doctor reads `ALZ_ACCOUNT_DIRS`; the spec never names it; no test uses it.
- `alz-core::git::child_environment` covers source calls only; its doc says §9.4 calls need their own.

**How to apply:** start R6 work and R8 planning from TASK-55.7's plan and notes (owner questions 1 to 9)
and subtasks TASK-55.7.2 to .16. See [[r5-planning-facts]] and [[private-security-tasks]].

R6.2 spec pass (2026-11-24): the reference prints account warnings as `account-<code>` with finer codes
(account-config-shape, account-file-ignored, account-home, ...). Lead decision the same day: diagnostic
codes are output, so parity applies unless a recorded reason says otherwise; the spec now lists the
reference's codes (R6.6 had built coarse account-config/account-allowed and follows up). Also: a failed
check after the .git/config write is an error (exit 1) in init and build --adopt, not a warning. Reference `quote()` is JSON.stringify of stripped text (double quotes). A literal
grep of test assertions cannot prove a templated spec line; a placeholder-span regex match needs spans
extracted from whitespace-flattened text, double-backtick spans first, or pairs go wrong.
