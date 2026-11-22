---
id: TASK-55.1.1
title: 'Spike: choose the YAML library for the Rust port'
status: Done
assignee:
  - '@architect'
created_date: '2026-11-22T15:52:00Z'
updated_date: '2026-11-22T16:39:00Z'
labels:
  - area:rust
  - area:spec
milestone: m-7
dependencies: []
references:
  - decision-28
parent_task_id: TASK-55.1
priority: high
ordinal: 59390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
alz parses layer.yaml, layer.lock, frontmatter and settings from untrusted repositories. serde_yaml is no longer maintained. Compare the maintained candidates (check current maintenance and soundness yourself; do not rely on memory) against: active maintenance and licence; serde support; limits on aliases, nesting depth and input size; error messages that do not quote the input; YAML 1.1 versus 1.2 behaviour compared with what the TypeScript reference accepts today (booleans like yes/no, numbers, merge keys); and the byte-for-byte output alz writes (YAML it generates). Write a small throwaway comparison, not production code.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 A written comparison of at least three maintained candidates against every criterion, with evidence from running them
- [x] #2 A recommendation the owner can accept or reject, noting what changes for users if YAML parsing behaviour differs from the TypeScript reference
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
### Spike result (2026-11-22, @architect): YAML library for the Rust port

Evidence status: the TypeScript reference was RUN (the real `src/` functions `parseYaml`, `parseYamlLimited`, `stringifyYaml`, `parseMarkdown`, `parseManifest` and `serializeAllowState`, bundled with the repo's esbuild; yaml 2.9.1, Node 22.23.5) on 301 real files (test/fixtures, .layer, a cached personal layer), 45 malicious files and 21 scalar probes. The Rust candidates were checked against crates.io metadata, the RustSec advisory-db (HEAD 2026-11-22) and their published source. The Rust comparison crate was NOT built or run (turn limit), so every Rust runtime claim below is UNVERIFIED and comes from reading the source. AC #1 asks for evidence from running the candidates: it is not met yet.

Correction to the task text: `layer.lock` is JSON (`src/model/lock.ts` uses `JSON.parse`), not YAML. The YAML alz writes: agent, rule and skill frontmatter (hashed in layer.lock), adopt's layer files and its comment-preserving edit of `layer.yaml` (`YAML.parseDocument`), the leak-guard rescan and `allowed.yaml`.

### Candidates

| Criterion | serde-saphyr 2.2.0 | yaml_serde 0.10.7 | noyalib 0.0.63 | saphyr 0.4.0 (no serde) |
| --- | --- | --- | --- | --- |
| Maintenance | 42 releases since 2025-09; 2.2.0 and 3.0.0-rc.1 on 2026-11-18; 708 reverse deps | YAML Organization fork of serde_yaml, 7 releases since 2026-01, last 2026-08-18; 488 reverse deps | 60 releases since 2026-05-10 (0.0.x churn); 68 reverse deps | 24 releases since 2024-04, last 2026-11-21 |
| Licence | MIT OR Apache-2.0 | MIT OR Apache-2.0 (libyaml-rs: MIT) | MIT OR Apache-2.0 | MIT OR Apache-2.0 |
| Soundness, advisories | `forbid(unsafe_code)`, 0 unsafe lines; parser granit-parser (same author, forbid unsafe); no advisory | 58 unsafe lines, plus libyaml-rs (c2rust translation of C, 236 unsafe lines, one release); `panic!("unresolved alias")` in de.rs | `forbid(unsafe_code)`; RUSTSEC-2026-0333 (budgets skipped on typed path, fixed 0.0.53); same author's serde_yml and libyml were archived as unsound (RUSTSEC-2025-0068, -0067) | 0 unsafe lines; no advisory |
| serde | yes (de and ser) | yes (de and ser) | yes | no: hand mapping from a tree |
| Alias limit | configurable: `max_aliases` (default 50k), replay and anchor-ratio limits | fixed: alias jumps up to 100 x events | configurable: `max_alias_expansions`, ratio | `alias_limit` in nodes (default 100k) |
| Depth limit | configurable `max_depth` (default 64), flow nesting 255 | fixed recursion 128 | configurable `max_depth` | none found in loader |
| Size limit | `max_reader_input_bytes` (readers only), `max_nodes`, `max_events`, `max_total_scalar_bytes` | none | `max_document_length`, `max_nodes`, more | none |
| Errors without input | structured `Error` enum (~65 variants), so alz can render its own text; default `with_snippet: true` prints a source window and must be off; serde value errors carry the value | `Display` strings; serde invalid-type text quotes the value | structured errors, snippets optional | parser errors carry position only |
| YAML 1.2 scalars vs TS | defaults lean 1.1 (`strict_booleans: false` accepts yes/no/on/off, `merge_keys: Merge`); options give 1.2 core | 1.2 core booleans only; no merge on deserialise | `yaml_version` defaults to 1.2; strict and legacy switches | 1.2 core |
| Byte-for-byte output | serializer exists; match with TS UNVERIFIED | libyaml emitter; match UNVERIFIED | UNVERIFIED | emitter exists; UNVERIFIED |

Ruled out: serde_yaml (deprecated 2024-03, on unsafe-libyaml: RUSTSEC-2024-0449 unmaintained), serde_yml (RUSTSEC-2025-0068 unsound), serde_norway (last release 2024-12), serde_yaml_ng (2024-05, on unmaintained unsafe-libyaml), yaml-rust (RUSTSEC-2024-0320), serde_yaml_bw (C-translated libyaml dependency; its author now maintains serde-saphyr).

### What the TypeScript reference accepts today (measured)

- YAML 1.2 core schema. Booleans are only true/True/TRUE/false/False/FALSE. yes/no/on/off/y/n/tRuE are strings, so `options.commit_generated: yes` is an error, and `locked: yes`, `floor: yes` and `allow_widen: yes` silently count as false.
- Null: null, Null, NULL, ~ and empty. Integers: `017` is 17 (decimal), `0o17` is 15, `0x1F` is 31; `0X1F`, `0b101` and `1_000` are strings. `1e3` (1000) and `2.0` (2) pass the integer budget checks. `.inf`, `.Inf`, `.INF`, `.nan` and `.NaN` are floats. `1.10` becomes 1.1. `12:30:00`, dates and timestamps are strings.
- `<<` is an ordinary key: no merging (s02, s03). A `%YAML 1.1` directive switches the document to 1.1 rules (x07: `yes` became true, the merge was applied, the key `n` became "false").
- Duplicate keys are an error; so is more than one document. Unknown tags become strings with a warning; `!!binary` becomes bytes and then fails the type checks.
- Aliases: yaml's default `maxAliasCount` is 100, so `parseYaml` already has the same limit as `parseYamlLimited` (150 aliases fail in both, 50 pass, 30 aliases of a 100-item list pass; the alias bomb stops in 2 ms).
- Depth: no limit. The JS stack overflows somewhere between 200 and 1000 levels (2000 flow levels and 1000 block levels fail), and the reported column differs from run to run (741 against 979), so it is not deterministic.
- Size: no parser limit. 32 MiB parsed in about 5 s. The 1 MiB read cap exists only on the account-resolution paths.
- Self-referencing and recursive aliases (x16, x19) parse into cyclic objects without an error.
- NUL and BEL bytes are accepted in plain scalars. Invalid UTF-8 becomes U+FFFD. A BOM is stripped and CRLF is accepted.
- Errors keep only the first line. One case leaks input: the unresolved-alias message prints the alias name (e03 printed the secret marker). The other 12 secret cases did not leak. Model errors quote unknown key names (e07).
- `ref` goes through `String()`: `ref: [x]` is accepted as ref "x" (e13), and `ref: 1.10` would become "1.1".
- Non-string keys are turned into strings (a null key becomes ""; a collection key becomes "[ a, b ]").
- Emission (`lineWidth: 0`): yes/no/on/off, `1_000`, `1.0.0`, `2026-11-22` and `12:30` are written plain. "true", "123", "0x10", ".inf", "~" and "" are written in double quotes. Multi-line strings become literal blocks (`|-`, `|`, `|+`, `|2-`). Control characters are escaped ("bell\a"). Floats come out as `1e+21` and `1e-7`; empty collections as `[]` and `{}`. Integer-like keys move to the front (JS object order). `allowed.yaml` writes `claude: yes` plain. 229 TS output samples are saved as golden candidates.

### Recommendation (the owner can accept or reject; it is provisional until a Rust run confirms the UNVERIFIED rows)

1. Parse with serde-saphyr, pinned `=2.2.0`, `default-features = false`, `features = ["deserialize"]`; not 3.0.0-rc.1. One alz-core module wraps it and sets: `strict_booleans: true`, `merge_keys: AsOrdinary` (as TS does), duplicate keys as an error, `legacy_octal_numbers: false`, `with_snippet: false`, a budget with `max_aliases: 100` and `max_depth: 64`. Before parsing, alz caps every untrusted file at 1 MiB. alz renders its own diagnostic per error variant with line and column only, and never shows the library's `Display` text or a serde value. A property test or fuzz target checks that no diagnostic contains input bytes.
2. alz writes its generated YAML itself, with a small emitter for the subset it writes (string maps, lists, bools, integers). The quoting rule goes in the spec, and the emitter is tested byte-for-byte against the TS golden samples, so generated files and layer.lock hashes stay the same at cutover. No serializer dependency. Adopt's comment-preserving `layer.yaml` edit becomes a targeted list-append edit, planned separately (UNVERIFIED).
3. Fallback: yaml_serde (YAML Organization). It has fixed limits and uses C-translated unsafe code.

### What changes for users against the TS reference if accepted

- These stay the same: yes/no/on/off as strings, `<<` as an ordinary key, duplicate keys and multiple documents as errors, the alias limit of 100.
- These become errors where TS accepted them (stricter): cyclic aliases; NUL or control bytes; invalid UTF-8 (instead of U+FFFD); a list or number in a string field (`ref: [x]`; `ref: 1.10` stays "1.10" instead of becoming "1.1"); nesting over 64 (TS already fails somewhere between 200 and 1000); possibly `1e3`/`2.0` for integer budgets and `%YAML 1.1` documents. How serde-saphyr handles those last two is UNVERIFIED.
- Error text changes to text the spec defines, and the unresolved-alias error no longer prints the alias name.
- Generated output stays the same only if the emitter matches the golden samples. Otherwise the first Rust build regenerates files, changes the layer.lock hashes, and `alz check` fails until a rebuild. Either way the parse changes are user-visible: the spec should list them, and cutover needs a minor 0.x version bump.

### Found in the TS reference (proposed as discovered tasks, not created)

(a) The unresolved-alias error quotes the alias name. (b) Cyclic aliases are accepted. (c) `ref` is coerced with `String()`. (d) `parseYamlLimited` is the same as `parseYaml`, because the default alias limit is already 100. (e) `locked: yes` silently means not locked.

Evidence is in the session scratchpad `yaml-spike/` (corpus generator, TS probe, `out/ts/results.jsonl`, `out/ts/emit/`), which is temporary.

### Rust comparison run (2026-11-22, @architect): measured, replaces the UNVERIFIED Rust rows above

Built and ran a throwaway crate (Rust 1.99.1, release build, macOS) with serde-saphyr 2.2.0, yaml_serde 0.10.7, noyalib 0.0.63 (latest on crates.io, 2026-11-19) and saphyr 0.4.0 (tree only, no serde) on the same inputs as the TS run: 301 real files, 45 malicious files, 21 scalar probes, 229 TS emitted samples. One process per (library, mode, file); time measured in-process, peak RSS with `/usr/bin/time -l`. Every file is read as strict UTF-8 first (as alz would). Variants: `ss-default` (serde-saphyr defaults), `ss-alz` (options from the recommendation: strict_booleans, merge_keys AsOrdinary, duplicate keys Error, legacy octal off, with_snippet false, max_aliases 100, max_depth 64), `ss-alz-snip` (ss-alz with snippets on), `ss-alz2` (ss-alz plus non_finite_float_policy PassThrough, underscored_numbers_as_strings, max_nodes 20000, alias_limits.max_total_replayed_events 10000), `yaml_serde`, `noyalib` (defaults), `noyalib-alz` (strict booleans, merge AsOrdinary, duplicates Error, max_alias_expansions 100, max_depth 64), `saphyr`. Modes: `any` (untyped tree through `deserialize_any`), `typed` (Option<bool>/Option<u64>/Option<String> fields), `manifest` (partial layer.yaml struct, deny_unknown_fields), `frontmatter`.

| Criterion (measured) | serde-saphyr (ss-alz2) | yaml_serde | noyalib | saphyr |
| --- | --- | --- | --- | --- |
| Real files, tree equal to TS | 301/301 (all variants) | 301/301 | 301/301 | 301/301 |
| Alias bomb m01 (TS: error 1.8 ms) | error 5.9 ms, 7.9 MiB. ss-alz (aliases 100 only): stopped by max_nodes 250k after 184 ms, 107 MiB | "repetition limit exceeded" 59 ms, 40 MiB | default 279 ms, 177 MiB; -alz 28 ms, 75 MiB | 5.5 ms, 17 MiB |
| 150 / 50 / 30-to-list aliases (TS: err/ok/ok) | err at 101 / ok / ok (= TS) | ok / ok / ok (no count limit) | err / err / err (alias:anchor ratio > 10) | ok / ok / ok |
| Depth (TS: 200 ok, 2000 and block 1000 stack overflow) | 200 flow: error at 65; block 1000: error at 65; flow 2000 and 100000: "recursion limit" at 256 | error at 129 everywhere; 100000 flow 308 ms, 42 MiB | error at 129 (default), 65 (-alz) | flow 200 ok, flow >255 error, block 1000 accepted (224 ms): no block limit |
| Size: 1.5 MiB / 32 MiB / 8 MiB scalar (TS: 175 ms / 3.9 s / 28 ms, all ok) | ss-alz: 32 ms ok / Nodes budget error 131 ms / 11 ms ok. ss-alz2: node budget 20000 rejects 1.5 MiB and 32 MiB in 11-13 ms | 40 ms / ok 1.1 s, 777 MiB / 141 ms | 31 ms / max_sequence_length error 103 ms / 24 ms | 29 ms / ok 0.7 s, 865 MiB / 28 ms |
| Secret in error text, 22 error cases from e01-e13 (TS: 4/27, alias name and unknown key) | snippets on: 22/22 quote the input line. with_snippet false: 3/22, only key names (duplicate key e04, unknown field e07), 0 quote an input line. Structured: `Error::DuplicateMappingKey { key, location }`, `Error::location()` | 4/21: wrong-type errors quote the value (e06, e11), plus key names | 4/21 (5 in -alz): unknown-anchor error prints the alias name (e03, like TS), key names | 0/8: position only |
| Duplicate keys (TS: error) | error | ACCEPTED silently in the tree (x02, e04) | ACCEPTED by default (policy Last); error with policy Error | error |
| Multiple documents (TS: error) | error | error | error | returns 2 documents; caller checks |
| yes/no/on/off, y/n (TS: strings) | strings (tree); typed bool: error | strings; typed: error | strings; typed: error | strings |
| True/TRUE (TS: bool) | bool | bool | bool; -alz (strict_booleans): string, typed error | bool |
| Quoted "true", "16" into bool/u64 (TS: rejected) | ACCEPTED as true and 16 (typed path coerces quoted scalars); tree keeps strings | error (= TS) | error | n/a |
| 017 / 0o17 / 0x1F / 0b101 / 1_000 (TS: 17 / 15 / 31 / "0b101" / "1_000") | float 17 / 15 / 31 / int 5 / "1_000" (ss-alz and ss-alz2); typed u64: 017 error, 1_000 = 1000 even with underscored_numbers_as_strings | "017" / 15 / 31 / "0b101" / "1_000" | 17 / 15 / 31 / "0b101" / "1_000" | same as noyalib |
| 1e3, 2.0 into an integer (TS: 1000, 2) | error | error | 1000, 2 | n/a |
| .inf/.nan (TS: floats) | default policy REJECTS the whole document on an untyped parse (s01 fails); PassThrough: floats | floats | floats | floats |
| 1.10, 0x10, true into a String field (TS: String() gives "1.1", "16", "true") | "1.10", "0x10", "true" (typed); tree: 1.1, 16, true | same as serde-saphyr | error (strict) | n/a |
| `<<` merge (TS: ordinary key) | AsOrdinary: ordinary key (= TS); default: merges, and errors on x18 | ordinary key | default merges, error on x18; -alz: ordinary | ordinary key |
| `%YAML 1.1` directive (TS: switches to 1.1) | ignored with alz options: yes stays a string, no merge | ignored | ignored | ignored |
| Cyclic aliases x16, x19 (TS: accepted, cyclic objects) | error | error (recursion limit) | error | silently BadValue |
| NUL / BEL in a scalar (TS: accepted) | error / error | error / error | error / error | error / ACCEPTED |
| `!!binary` (TS: bytes, then type error) | decoded into the string "hello" | base64 text as string | tagged value | BadValue |
| Custom tag `!foo`, `!!python/object` (TS: string) | string (= TS) | tagged enum / string | tagged | tagged / BadValue |
| Non-string keys, complex key (TS: stringified) | typed keys kept (int, bool, null, seq) | typed keys | typed; -alz stringifies scalars | typed |
| Emitter byte-identical to the 229 TS samples | 75/229 (ss-default; quotes yes/no/on/off, 1_000, 1.0.0, dates and the key `n`) | 129/229 (single quotes where TS uses double) | 0/229; 218/229 if a final newline is added | 0/229 (`---` header, `~` for null) |

Invalid UTF-8 fails in every case at alz's own UTF-8 read (TS turned it into U+FFFD). Raw rows: `out/rust/results.jsonl` (3,792 rows) and `out/rust/emit.jsonl` (916 rows).

### Does the recommendation stand?

Yes for the library: serde-saphyr pinned `=2.2.0`, fallback yaml_serde. The run changes how alz uses it:

1. Option set (all measured): strict_booleans, merge_keys AsOrdinary, duplicate_keys Error, legacy_octal_numbers false, with_snippet false, non_finite_float_policy PassThrough, underscored_numbers_as_strings true, and a budget with max_aliases 100, max_depth 64, max_nodes 20000 and alias_limits.max_total_replayed_events 10000. Without the node and replay limits the alias bomb costs 184 ms and 107 MiB. The spec should set max_nodes from the largest real file, which passes at 20000, and keep the 1 MiB file cap.
2. alz parses into its own YAML tree through `deserialize_any` and converts that tree to domain types in alz code. It does not derive `Deserialize` on domain structs. The typed path turns quoted "true" into true and "16" into 16 (`allow_widen: "true"` would widen where TS did not) and reads `1_000` as 1000. The tree keeps those as strings, as TS does.
3. Diagnostics come from the `Error` variant and `location()` only, never from `Display`. Even with snippets off, duplicate-key and unknown-field messages print the key.
4. alz keeps its own emitter. No library reproduces the TS output: the best are yaml_serde at 129/229 and noyalib at 218/229 after adding a final newline.

The fallback still needs work: yaml_serde has no alias count limit (150 aliases pass), accepts duplicate keys in the tree (alz's tree builder sees both entries and would have to reject them itself) and has no size limit (32 MiB took 777 MiB). noyalib has the closest emitter and configurable limits. Against it: its defaults accept duplicate keys and reject 30 aliases, and the advisory history is unchanged.

### What changes for users against TS (with the option set above)

- Unchanged: yes/no/on/off/y/n are strings; true/True/TRUE are booleans; null forms; 0o17 and 0x1F; .inf/.nan; dates are strings; `<<` is an ordinary key; duplicate keys and multiple documents are errors; the alias limit is 100.
- New errors: nesting over 64 (TS accepted 200); cyclic aliases; NUL and BEL bytes; invalid UTF-8; files over the node budget; 1e3 or 2.0 for an integer, unless alz's tree conversion accepts integral floats as TS does. A non-string scalar in a string field (`ref: 1.10`) becomes an error, because the tree holds 1.1 and the original text is gone.
- Silent value changes: a `%YAML 1.1` directive no longer switches rules; `017` is float 17 (alz's conversion decides); `0b101` is the integer 5 instead of a string; integers above 2^53 stay exact; `-0` is 0; `!!binary` text is decoded into a string. The option `ignore_binary_tag_for_string` should prevent that last one, but it was not run (UNVERIFIED).
- Error text becomes alz's own, with line and column; no input snippets and no alias name.
- These are user-visible: list them in the spec and bump the minor 0.x version at cutover (unchanged from above).

### Rerun

```
S=<session scratchpad>/yaml-spike      # temporary; crate in $S/rust
cd $S/rust && CARGO_TARGET_DIR=$PWD/target cargo build --release
rm -rf $S/out/rust && python3 -I driver.py $S && python3 -I driver.py $S ss-alz2   # about 30 s
python3 -I analyse.py $S real|mal|leak|scalars|emit
```

Correction to the Rust comparison run above: `out/rust/results.jsonl` has 3,935 rows, not 3,792. That figure includes the ss-alz2 rerun. `emit.jsonl` has 916 rows.

2026-11-22 architect, finalisation. The owner accepted the recommendation, with additions, as decision-28:
- serde-saphyr =2.2.0, deserialisation only, with the ss-alz2 option set, max_depth 64 and the 1 MiB cap;
- an alz-owned tree read through deserialize_any;
- diagnostics from the error variant and location only;
- an alz-owned emitter;
- fallback yaml_serde;
- a number in a text field is an error that names the field and file and suggests quotes;
- integral floats count as integers, as in TypeScript;
- the list of user-visible differences goes into a later spec task, with a minor version bump at cutover.

Evidence:
- AC #1: the "Rust comparison run" table above. Four maintained candidates, each run on 301 real files, 45 malicious files, 21 scalar probes and 229 TS samples, against every criterion in the description. Maintenance, licence and advisories come from crates.io and the advisory-db, as recorded. Raw rows are in that session's out/rust/results.jsonl (3,935 rows) and emit.jsonl (916 rows). I did not rerun it at finalisation; the scratchpad is temporary.
- AC #2: "Does the recommendation stand?" and "What changes for users against TS" above. decision-28 records the owner's acceptance.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Compared four maintained YAML crates against the TypeScript reference by running all of them on the same inputs: serde-saphyr 2.2.0, yaml_serde 0.10.7, noyalib 0.0.63 and saphyr 0.4.0. The inputs were 301 real files, 45 malicious files, 21 scalar probes and 229 TS output samples. The full table and the rerun recipe are in the notes.

Result, accepted by the owner as decision-28:
- serde-saphyr =2.2.0, deserialisation only, with YAML 1.2 core options and alias, depth, node and replay limits, plus a 1 MiB cap per file;
- parsing into an alz-owned tree, with conversion in alz code;
- diagnostics without input text;
- an alz-owned emitter tested byte for byte against the TS samples;
- fallback yaml_serde.

Findings in the TS reference are filed as TASK-59 to TASK-62.

Left open:
- the list of user-visible parse differences, for a later spec task (minor version bump at cutover);
- ignore_binary_tag_for_string was not run;
- adopt's comment-preserving layer.yaml edit is to be planned in R5.

No production code. Checks: the comparison runs recorded in the notes; no repository tests apply.
<!-- SECTION:FINAL_SUMMARY:END -->
