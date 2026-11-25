---
name: spec-test
description: Write a Rust test from a requirement of docs/LAYERING-SPEC.md. Use when adding or changing alz behaviour, fixing a bug, or closing a gap in the spec's test coverage.
---
# Write a test from the spec

The specification, `docs/LAYERING-SPEC.md`, is the contract, and Rust tests written from it are
the proof that alz does what it says. A test asserts what the spec requires, never what the code
happens to do. When the spec is silent or wrong, change the spec in the same change, then test it.

## Choose the kind of test
| The requirement is about | Test | Where |
| --- | --- | --- |
| A value that must be valid, parsed or merged (names, paths, limits, merge laws) | Unit test, plus a `proptest` property for a law (associative, idempotent, monotone) | `#[cfg(test)] mod tests` beside the code, or `crates/<crate>/tests/` |
| An effect (files, git, the cache, a terminal) | Integration test of the effect crate against a real temporary directory | `crates/alz-io/tests/`, helpers from `alz-test-support` |
| A command: exit code, stdout, stderr, files on disk | `assert_cmd` test (`cargo_bin_cmd!("alz")`) in a scratch directory with an isolated `HOME` | `crates/alz/tests/` |
| Output for a whole repository and its layers | A fixture under `fixtures/NN-<topic>/` copied into a scratch directory, then built and compared with the expected tree | `crates/alz/tests/` |
| An untrusted input format | Examples for each rule, plus a `cargo fuzz` target when it parses bytes | `fuzz/` |

## Steps
1. Find the requirement: the spec section, the sentence, and the guarantee in it. Name the test
   after the guarantee ("refuses a lock that lists a path outside the repo"), not the
   implementation, and put the section number in the doc comment of the test file or module.
2. Write the test first and watch it fail for the right reason.
3. Make the change in the code. Use the gr-rust skill for how it is shaped.
4. Run the single test, then `mise run rust:check`.

## Rules
- Assert the exit code, the message the user sees and the files on disk. Help, usage errors and
  version follow decision-26: `--version` exits 0 with empty stderr and stdout that contains the
  version; `--help` exits 0 with stdout not empty; a usage error exits 2 with empty stdout and
  stderr not empty. Never compare their full text.
- To prove that something did not happen, compare the whole tree (paths, bytes, modes) before
  and after, with `read_tree` from `alz-test-support`.
- Use real temporary directories (`ScratchDir`), never a mocked file system. Give each run an
  isolated `HOME` and never read or write the real `~/.claude*`, `~/.codex*` or `~/.cache`.
- Fixtures under `fixtures/` are inputs: copy them into a scratch directory and never edit them
  in place. A repository fixture has `personal/` (a layer with `layer.yaml`) and `repo/` (a
  `layer.yaml` stack plus `.layer/`); expected output goes in the crate's `tests/fixtures/`.
- Output is deterministic: a test that builds twice compares the bytes.
- A run that starts a process has a timeout, so a command that blocks fails instead of hanging.
- Tests sit in a `mod tests` marked `#[cfg(test)]`, as `clippy::tests_outside_test_module` asks.
  Fix a lint instead of suppressing it; a needed suppression is `#[expect(lint, reason = "...")]`.
- A bug fix gets a regression test named after the guarantee it protects.
