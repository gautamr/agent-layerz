---
id: TASK-55.5.14
title: 'R4.14: The leak guard for public layers in alz lint'
status: Done
assignee:
  - '@implementer'
created_date: '2026-11-23T12:28:00Z'
updated_date: '2026-11-23T15:27:00Z'
labels:
  - area:rust
  - area:security
milestone: m-7
dependencies:
  - TASK-55.5.13
  - TASK-55.5.1
parent_task_id: TASK-55.5
priority: high
type: task
ordinal: 127390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Plan for this step (§9.2 as R4.1 writes it; owner answer 2 and decision-38).
- alz-core `leak_guard.rs`, for a layer with visibility public. It checks every file of LayerContents (layer.yaml without its forbid_patterns, as the reference does) against three things:
  - the layer's forbid_patterns;
  - the secret scan of R4.11;
  - the email and private-host heuristics, which give the warnings `leak-email` and `leak-host`.
- forbid_patterns compile with regex-lite (decision-38), with size limit 64 KiB and nest limit 32. A layer has at most 64 entries, each at most 512 characters. Before compiling, a look-around (`(?=`, `(?!`, `(?<=`, `(?<!`) or a backreference (`\1` to `\9`, `\k<`) is found and refused with a message that says these are not supported. Every refusal is `leak-forbid` and names the entry by its index, never its text.
- Matches use is_match only. A diagnostic names the file and the pattern's index, never the matched text.
- alz lint runs the guard on each public layer, in both modes.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Test 10's forbidden-pattern and fake-token cases pass against the Rust binary (mise run test:acceptance with ALZ_BIN on test 10)
- [x] #2 A forbid_patterns entry with look-around or a backreference is refused with leak-forbid and a message naming the feature; one that does not compile, is longer than 512 characters, passes the size or nest limit, or is the 65th entry is refused naming its index; no diagnostic holds the entry's or the matched text (unit tests)
- [x] #3 The email and private-host heuristics warn as leak-email and leak-host, as the reference does (unit tests)
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
R4.14 implementation (not committed; ACs not checked).
Files: crates/alz-core/src/leak_guard.rs + leak_guard/{forbid,heuristics,layer_yaml,tests}.rs (new); diagnostic.rs (codes leak-forbid, leak-email, leak-host; Position::line); secret.rs (pub(crate) bounded(): one builder with SIZE_LIMIT/NEST_LIMIT for token shapes and forbid_patterns); lint.rs (LayerLint::meta, with_diagnostics); stack/resolved.rs (StackReport::resolved_layers: layers kept even when the stack is invalid or an entry failed); alz-io ReadOnlyDir::scan; alz command/lint.rs (guard in --layer and stack mode); crates/alz/tests/{lint,stack_lint}.rs; test/acceptance/34-yaml-limits.test.ts (see below).
AC1: ALZ_BIN=target/release/alz vitest --project acceptance test/acceptance/10-leak-guard.test.ts -> 3/3 pass (clean layer, forbidden pattern, fake token; stderr never holds the token). 
AC2 (unit tests leak_guard::tests): look-around (4 forms) and backreferences (\1, \2, \k<n>) refused with 'uses look-around' / 'uses a backreference' naming the feature; escaped or in-class '(?=' and '\\1' are text; non-compiling and \p{L}, nest 40, size past 64 KiB refused by number; 513 chars refused, 512 (also 512 multibyte) accepted; the 65th entry refused by number, first 64 used; test no_diagnostic_holds_a_refused_entry_or_a_matched_text checks no entry or matched text is printed.
AC3 (heuristics tests + leak_guard::tests): leak-email and leak-host warnings, once per file; public domains (example.com/org/net, users.noreply.github.com, anthropic.com) do not warn; exit code stays 0.
Integration (assert_cmd): crates/alz/tests/lint.rs (4 cases, --layer), stack_lint.rs (guard runs for each public layer although another entry failed).
Checks: mise run rust:ci exit 0; mise run lint ok; mise run test 1177 passed, 80 expected fail; full acceptance suite against the Rust binary: 196 passed, 681 failed of 882 (everything driving build/init/check etc., not R4.14).
Test change: 34-yaml-limits 'lints 100 aliases' and 'lints a document of 20000 nodes' carried 100 and ~20000 forbid_patterns entries in a PUBLIC layer, which the spec's 64-entry cap now refuses (leak-forbid) in the Rust alz; both documents are now private layers (PRIVATE_BASE). They still test the YAML limits and pass on the TypeScript reference and the Rust binary.
Choices where spec §9.2 is silent or loose (spec gaps, to settle in R4.1/spec):
1. Granularity: one diagnostic per (file, entry), per (file, secret kind), and one warning per file for leak-email and leak-host; no line numbers (the reference reported per line). Patterns match the whole file text, not line by line (spec: 'text of every file'), so a pattern with (?s) or \s can span lines and ^/$ need (?m).
2. Binary and non-UTF-8 files are scanned too (read lossily); the reference skipped files with a NUL in the first 8 KiB. Spec says 'every file'.
3. layer.yaml 'without its forbid_patterns field': the lines from the key up to the next top-level key are removed (comment-only lines in that span stay); if the key is not alone on its line (flow mapping on one line) nothing is removed. The reference re-serialised the YAML (comments dropped). The spec does not say how the field is removed.
4. Spec lists 'does not compile', 'compiles to more than 64 KiB' and 'nests more than 32 levels' as separate refusals; regex-lite gives no reliable way to tell them apart, so one message covers the three. Spec could say they share a message.
5. More than 64 entries: only entry #65 gets a diagnostic ('it and every later entry are refused'); the rest are not compiled. Spec says only 'at most 64 entries'.
6. A refused-entry diagnostic is located at <layer>:layer.yaml (no line). The look-around/backreference pre-scan skips escapes and character classes.
7. alz lint --layer reads the layer with the build's walk (Purpose::Build, no links, 16 MiB cap); a layer that cannot be read is layer-unreadable, same as in stack mode. .env files in --layer mode are not reported (not in this task).
8. Spec §9.2 lists codes in §10.1 but the leak-email/leak-host heuristics (email regex, public domains, host suffixes) are specified only by example in the text; I used the reference's patterns, ASCII-only \b.

Verified by the lead (2026-11-23): mise run rust:ci exits 0, lint passes, src/ unchanged; test 10 passes 3/3 against the Rust binary (implementer's run). Choices accepted (binary files scanned, one diagnostic per file and pattern, the forbid_patterns span removed from layer.yaml line-wise); the test 34 change to private layers accepted (the owner's 64-entry cap).
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
The leak guard checks every file of a public layer in alz lint: forbid_patterns compiled with regex-lite under the decision-38 limits (look-around and backreferences refused by name, at most 64 entries of 512 characters), the secret scan's token shapes, and email and host warnings; no diagnostic holds a matched text. Test 10 passes against the Rust binary.
<!-- SECTION:FINAL_SUMMARY:END -->
