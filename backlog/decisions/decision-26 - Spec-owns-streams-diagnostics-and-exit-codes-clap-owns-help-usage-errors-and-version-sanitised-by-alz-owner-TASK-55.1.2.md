---
id: decision-26
title: >-
  Spec owns streams, diagnostics and exit codes; clap owns help, usage errors
  and --version, sanitised by alz (owner, TASK-55.1.2)
date: '2026-11-22T16:27:00Z'
status: accepted
---
## Context

This is doc-5 choice 5. decision-22 says user-facing messages follow the spec, and that where a
Rust convention changes them the spec is updated on purpose. The spec states the messages and
exit codes of individual commands, but not the general output contract. Several things exist
only in `src/cli` and the tests (25-cli, 27-install-local, 31-account-which, 32-account-allow):
- the diagnostic line `<level> [<code>] <where>: <message>`;
- exit 70 for an internal failure;
- `--version` printing a bare version, and `-v` as its short form;
- the custom usage text.

Checked with clap 4.6.9 (TASK-55.1.2 notes):
- `--version` prints `alz <version>` and `-V` is its short form; `-v` is a usage error.
- No command prints help on standard error and exits 2.
- A usage error prints `error: …`, a `Usage:` line and a hint, and exits 2.
- clap echoes arguments raw. On a terminal, ESC sequences and U+202E reach it unchanged. Off a
  terminal, CSI sequences are stripped but `\r` and U+202E stay.

Options considered: the spec owns the contract and clap owns help and usage; keep today's text
byte for byte; the spec owns all text, with clap only parsing.

## Decision

The spec owns the output contract:
- which stream each kind of output goes to;
- the diagnostic line format;
- exit codes: 0 ok, 1 failed, 2 usage error, 70 internal failure, plus the codes the spec states
  per command;
- every message the spec quotes;
- the rule that all untrusted text, arguments clap echoes included, has its control and format
  characters removed before it is printed.

clap owns help text, usage lines, usage-error wording and `--version` (`alz <version>`, with `-V`
and `--version`), with its defaults. The spec states only their stream and exit code. alz parses
with `try_parse` and renders clap's errors itself, sanitised, in the one place that renders
diagnostics. The TypeScript help and usage wording is not imitated.

## Consequences

- `docs/LAYERING-SPEC.md` gains an "Output and exit codes" section, with §10 pointing to it, in
  the pull request that builds the first Rust command exposing it. The section is spec work for a
  later task, not R0. The `--version` and `--help` tests of the skeleton (TASK-55.1.3) follow this
  decision.
- R1 (TASK-55.2) rewrites the assertions on `usage: alz` and on the bare version (25-cli,
  27-install-local, 31, 32) to hold for both binaries until cutover: exit code, stream, and
  version text that contains the version.
- The control-character test in 31 stays. It becomes the regression test for sanitising clap's
  errors.
- The diagnostic line format is unchanged, so the assertions on `error [<code>]` lines stay.
- Scripts and the npm wrapper (R7) must not expect a bare version from `--version`.
