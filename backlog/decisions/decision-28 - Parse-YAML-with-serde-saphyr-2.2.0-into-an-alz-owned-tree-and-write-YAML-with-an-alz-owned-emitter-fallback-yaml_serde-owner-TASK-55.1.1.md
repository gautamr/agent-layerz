---
id: decision-28
title: >-
  Parse YAML with serde-saphyr 2.2.0 into an alz-owned tree and write YAML with
  an alz-owned emitter; fallback yaml_serde (owner, TASK-55.1.1)
date: '2026-11-22T16:37:00Z'
status: accepted
---
## Context

This is doc-5 choice 2. alz parses `layer.yaml`, frontmatter, settings, `accounts.yaml` and
`allowed.yaml` from untrusted sources. It writes YAML itself: agent, rule and skill frontmatter,
which `layer.lock` hashes, adopted layer files, and `allowed.yaml`. `layer.lock` is JSON.
`serde_yaml` is deprecated and built on unmaintained `unsafe-libyaml`.

The TASK-55.1.1 spike compared four maintained candidates against the TypeScript reference:
serde-saphyr 2.2.0, yaml_serde 0.10.7, noyalib 0.0.63 and saphyr 0.4.0. Both sides were run on
the same inputs: 301 real files, 45 malicious files, 21 scalar probes and 229 samples of the
TypeScript output. The measured table is in that task's notes.

What the spike found:
- serde-saphyr was the only candidate that:
  - forbids unsafe code;
  - has configurable alias, depth, node and replay limits;
  - rejects duplicate keys;
  - has structured errors that need not quote the input (with snippets off, only key names);
  - matches the reference on 301 of 301 real files with the options below.
- Its typed path turns a quoted `"true"` into true and `"16"` into 16. Its untrusted-input
  defaults lean towards YAML 1.1, and it needs node and replay limits to stop the alias bomb
  cheaply (without them: 184 ms and 107 MiB; with them: 5.9 ms and 7.9 MiB).
- No library reproduces the TypeScript output byte for byte. The best were 129 of 229 samples
  (yaml_serde) and 218 of 229 after adding a final newline (noyalib).
- yaml_serde contains C code translated to Rust with unsafe blocks, has no alias count limit or
  size limit, and accepts duplicate keys in the tree.

## Decision

**Library.** alz parses YAML with `serde-saphyr`, pinned `=2.2.0` with
`default-features = false` and `features = ["deserialize"]` (deserialisation only). One `alz-core`
module wraps it and sets:
- YAML 1.2 core behaviour: `strict_booleans`, `merge_keys: AsOrdinary`, duplicate keys as an
  error, `legacy_octal_numbers: false`, `with_snippet: false`;
- `non_finite_float_policy: PassThrough` and `underscored_numbers_as_strings`;
- a budget of `max_aliases: 100`, `max_depth: 64`, `max_nodes: 20000` and
  `alias_limits.max_total_replayed_events: 10000`.

Every untrusted file is read as strict UTF-8 and capped at 1 MiB before parsing.

**Tree and conversion.** alz reads every document into its own YAML tree through
`deserialize_any` and converts that tree to domain types in its own code. It never derives
`Deserialize` on domain types, so a quoted `"true"` stays a string. Conversion rules:
- A number where text is expected (for example `ref: 1.10`) is an error that names the field and
  the file and suggests quoting the value.
- Whole-number fields keep the TypeScript behaviour: a float with no fractional part (`1e3`,
  `2.0`) counts as an integer.

**Diagnostics.** Text comes from the error variant and its location only, never from the
library's `Display` text or a serde value.

**Generated YAML.** A small emitter in `alz-core` writes it, for the subset alz writes (string
maps, lists, booleans, integers, block strings). It is tested byte for byte against the
TypeScript output samples. No serialiser dependency.

**Fallback.** yaml_serde, if serde-saphyr becomes unusable. Adopting it needs alz's own alias
count, duplicate-key and size checks, and a new decision.

## Consequences

- Generated files and `layer.lock` hashes stay the same at cutover only if the emitter matches the
  TypeScript samples, so the samples become golden tests in R2 (TASK-55.3). The quoting rule goes
  into the spec.
- The parse behaviour changes in ways users can see, and a later spec task lists all of them in
  `docs/LAYERING-SPEC.md`.
  - New errors:
    - nesting deeper than 64 levels;
    - cyclic aliases;
    - NUL and other control bytes;
    - invalid UTF-8, where the reference substituted U+FFFD;
    - files over the node budget;
    - a number in a text field.
  - Silent changes:
    - a `%YAML 1.1` directive is ignored;
    - `017` is no longer the integer 17 (alz's conversion decides);
    - `0b101` is the integer 5 instead of a string;
    - `!!binary` text is decoded, unless `ignore_binary_tag_for_string` prevents it, which is
      unverified;
    - integers above 2^53 stay exact.
  - Error text becomes alz's own, with line and column, no input snippet and no alias name.
- Cutover needs a minor 0.x version bump because of these changes.
- A property test or fuzz target checks that no YAML diagnostic contains input bytes.
- `cargo deny` (TASK-55.1.4) keeps serde-saphyr at `=2.2.0`. Moving to 3.x is a new decision
  after a rerun of the spike's comparison.
- Adopt's comment-preserving edit of `layer.yaml` (`YAML.parseDocument` in the reference) has no
  library behind it. It becomes a targeted edit, planned in R5 (TASK-55.6).
- The spike's findings in the TypeScript reference are filed as TASK-59 to TASK-62: alias names
  quoted in errors, cyclic aliases accepted, `ref` converted with `String()`, and both parse
  functions applying the same alias limit.
