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

**Amended 2026-11-22 (owner, R0 review).** The security review of TASK-55.1 found that this
decision did not say whether a line feed counts as a control character, how text that a person
decides on is shown, or what a panic prints. The skeleton's sanitiser, written for clap's output,
keeps line feeds and lists format characters by hand.

## Decision

The spec owns the output contract:
- which stream each kind of output goes to;
- the diagnostic line format;
- exit codes: 0 ok, 1 failed, 2 usage error, 70 internal failure, plus the codes the spec states
  per command;
- every message the spec quotes;
- the rule that all untrusted text, arguments clap echoes included, has its control and format
  characters removed before it is printed (amended below: three classes).

clap owns help text, usage lines, usage-error wording and `--version` (`alz <version>`, with `-V`
and `--version`), with its defaults. The spec states only their stream and exit code. alz parses
with `try_parse` and renders clap's errors itself, sanitised, in the one place that renders
diagnostics. The TypeScript help and usage wording is not imitated.

**Amended 2026-11-22 (owner, R0 review).** Untrusted text is printed in exactly one of three
classes, and the spec's "Output and exit codes" section names them:
1. **Stripped**, for diagnostics. Untrusted text quoted in a diagnostic has every control and
   format character removed, line feed included: U+0000 to U+001F, U+007F to U+009F, U+2028,
   U+2029, and every character of the Unicode general category Cf. This is the TypeScript
   `stripControl` (`src/core/sanitize.ts`).
2. **Escaped**, for text a person decides on, such as the layer source `alz account allow` shows
   before it asks. It is quoted and nothing is removed: printable ASCII (U+0020 to U+007E) stays,
   `"` and `\` get a backslash, and every other character is written `\u{XXXX}`, upper-case hex
   with at least four digits. Two texts that differ only by an invisible or look-alike character
   never print alike. This is the TypeScript `quoteEscaped`.
3. **clap's rendered output** (help, usage lines and usage errors) is the only output that keeps
   its own line feeds. The argument fragments it echoes are untrusted and get class 1, so no line
   feed, carriage return, escape or format character from an argument reaches the output.

"Format character" means the whole category Cf, not a list picked by hand. The code names the
Unicode version of its table, and a test checks the table against that version's data.

A panic hook prints one fixed message to standard error, with no payload, location or backtrace,
because a payload can hold untrusted text, and the process exits 70. The message goes into the
spec section.

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
- **Amended 2026-11-22 (owner, R0 review):**
  - Classes 1 and 2 are the reference's own two functions, so they add no change users see,
    except that the Cf table may differ from the reference's `\p{Cf}` by a few characters when
    the two Unicode versions differ.
  - The classes move into `alz-core` in R2 (TASK-55.3), so diagnostics, YAML errors
    (decision-28) and the binary share one implementation. The skeleton's
    `crates/alz/src/terminal_text.rs` moves there and its hand-picked list is replaced.
  - The tests state these laws: class 1 output holds no control or format character for any
    input (property test); class 2 output is printable ASCII, and two different inputs never give
    the same output (property test); an argument holding a line feed, a carriage return, ESC and
    U+202E gives a usage error that contains none of them (the test in 31, extended); a panic
    exits 70 with the fixed message.
