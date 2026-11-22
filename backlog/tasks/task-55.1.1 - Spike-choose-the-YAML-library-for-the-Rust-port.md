---
id: TASK-55.1.1
title: 'Spike: choose the YAML library for the Rust port'
status: In Progress
assignee:
  - '@architect'
created_date: '2026-11-22T15:52:00Z'
updated_date: '2026-11-22T16:22:00Z'
labels:
  - area:rust
  - area:spec
milestone: m-7
dependencies: []
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
- [ ] #1 A written comparison of at least three maintained candidates against every criterion, with evidence from running them
- [ ] #2 A recommendation the owner can accept or reject, noting what changes for users if YAML parsing behaviour differs from the TypeScript reference
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
<!-- SECTION:NOTES:END -->
