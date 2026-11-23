---
id: TASK-55.3
title: 'R2: Core model in Rust'
status: In Progress
assignee:
  - '@architect'
created_date: '2026-11-22T12:54:00Z'
updated_date: '2026-11-23T05:27:00Z'
labels:
  - area:rust
milestone: m-7
dependencies:
  - TASK-55.2
  - TASK-64
  - TASK-65
parent_task_id: TASK-55
priority: medium
ordinal: 35390
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
doc-5 phase R2: manifest, layer meta, lock, account model, YAML with limits, diagnostics, sanitising.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The suite's model and lint cases pass against the Rust binary
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
### Scope

doc-5 R2: manifest, layer meta, lock, account model, YAML with limits, diagnostics, sanitising. By decision-30 R2 also fixes TASK-59, TASK-60, TASK-61 and TASK-62 in Rust, and implements §6 name order for the lock and content-hash model (TASK-64). Designed from the spec as Rust (decision-22, doc-6, gr-rust); `src/` is read for behaviour only and never changed. R2 relies on no Claude Code or Codex fact (target-facts): the account model holds names; folder paths are R6's.

### Module layout (doc-6 §2, decision-23)

alz-core, pure:
- `untrusted_text.rs` (+ `untrusted_text/format_characters.rs`): `Stripped` (class 1), `Escaped` (class 2), `QuotedKey`; Cf table of Unicode 17.0, checked against a vendored UCD extract. (R2.2)
- `diagnostic.rs`: `Level`, `Code`, `Where`, `Position`, `Message`, `Diagnostic`, `Diagnostics`. (R2.3)
- `yaml.rs` + `yaml/{text,events,tree,load,error,fields}.rs`: the one entry point. (R2.4)
- `layer.rs` + `layer/{name,kind,visibility,meta}.rs`, `identity.rs` + `identity/commit_identity.rs`, `account.rs` + `account/{name,declaration,protected}.rs`, `target.rs` (`TargetName` only). (R2.5)
- `path.rs` (`SafeRelPath`), `stack.rs` + `stack/{source,git_ref,manifest}.rs`. (R2.6)
- `lock.rs` + `lock/{object_id,content_hash,lock_file}.rs`, `render.rs` + `render/json.rs`. (R2.7)
- `render/yaml.rs`, the emitter. (R2.8)
- `account/{user_config,allow_state}.rs`. (R2.9)

alz-io: `fs.rs` + `fs/read_only_dir.rs`: `ReadOnlyDir::open`, `read_file(&SafeRelPath, cap)` with O_NOFOLLOW, O_NONBLOCK, fstat S_ISREG and cap + 1 (decision-27). It depends on alz-core for `SafeRelPath`. The walk, git and the cache stay R3's. (R2.10)

alz: `command.rs` + `command/lint.rs`; `report.rs` renders diagnostics and exit codes; `terminal_text.rs` keeps class 3 only. (R2.2, R2.3, R2.10)

### Types (parse, don't validate)

Every value from untrusted input has one constructor that returns `Result`, with private fields:
- Names: `LayerName`, `AccountName`, `SlotName`, `KindLabel`.
- Paths and refs: `SafeRelPath` (its `Ord` is `str`'s, which is §6 name order), `GitRef`, `GitUrl` (refuses credentials, §5.2).
- Ids and hashes: `ObjectId = Sha1([u8; 20]) | Sha256([u8; 32])`, `ContentHash`, `FileDigest`.
- Identity and text fields: `IdentityName`, `EmailAddress`, `LayerVersion`, `Description`.

Sum types:
- `Visibility` with `Ord` as the privacy order; `Kind { Client, Other(KindLabel) }`; `TargetName { Claude, Codex }`.
- `AccountChoice { Default, Named }`; `AccountDeclaration { Every, PerTarget(NonEmptyTargetMap) }`.
- `Source = Path | Git | GitHub`.

Records: `LayerMeta`, `Manifest` holding `Stack<Declared>` (the first typestate of doc-6 §3.3; `Resolved` is R3's), `Lock`, `LayerFiles(BTreeMap<SafeRelPath, FileDigest>)`, so the hash order holds by construction, `UserConfig`, `AllowState`. Output collections are `BTreeMap`/`BTreeSet` or ordered `Vec`, never `HashMap`.

### Errors and diagnostics (decision-26 classes)

There is one error enum per boundary: `YamlError`, `SchemaErrors` (every field error of a file, so lint reports them all), `LockError`, `SourceError`, and alz-io's `ReadError`. Each converts with `From` into `Diagnostic { level, code, at, message }`.

`Message` is built only from `&'static str` and safe parts:
- `Stripped` (class 1): everything untrusted in a diagnostic;
- `QuotedKey`: the only input text a YAML or schema diagnostic may hold, cut at 64 code points after stripping, then `…`. Schema errors never print values;
- `Escaped` (class 2): built now, first used by `account allow` in R6;
- numbers, and the sealed `Printable` trait for validated ASCII names.

So a message that holds raw input does not compile. Class 3 stays in the binary, which renders clap's output and strips each argument fragment with `Stripped`. The binary renders `<level> [<code>] <where>: <message>` on stderr. Exit codes: 0 ok, 1 failed, 2 usage, 70 panic (unchanged).

### YAML entry point (decision-28, §7.13)

`yaml::parse` is the only caller of serde-saphyr. clippy `disallowed-methods` in alz-core enforces it, with one `#[expect]` (TASK-62 AC #2).

1. `YamlText::from_bytes`: the 1 MiB cap, strict UTF-8 and the raw control-character set (tab, LF and CR allowed).
2. An event pre-pass over `serde_saphyr::granit_parser`, decision-28's open choice of how tags reach alz. It refuses tags outside the 1.2 core set (`yaml-tag`), a second document, cyclic and unresolved aliases, and decoded scalars with any control character other than tab and LF. It counts the four §7.13 budgets exactly as the spec defines them: 100 aliases as written, 64 levels of nesting, 20000 nodes, 10000 replayed events.
3. serde-saphyr `deserialize_any`, with decision-28's options, into alz's tree. Its own budgets stay on as defence in depth.
4. `yaml/fields.rs`, the conversion helpers. A number where text is expected is an error that suggests quotes; a float without a fraction counts as a whole number; mapping keys stay in document order (TASK-66 decides output order in R5).

### Tests

- Unit tests next to the code, assert_cmd tests in `crates/alz/tests`.
- Golden tests: the YAML emitter against new reference samples (R2.8, since the spike's samples are gone), the lock emitter against this repository's `layer.lock`, and content-hash vectors from `test/spec/content-hash.ts`.
- Property tests (`proptest`, decision-35) state these laws:
  - class 1 output is clean, and stripping is idempotent;
  - class 2 output is printable ASCII and injective;
  - no rendered diagnostic holds a control character;
  - no YAML diagnostic holds input text except one quoted key;
  - newtypes round-trip (parse(print(x)) == x);
  - name order equals UTF-8 byte order and code point order;
  - lock emit then parse is the identity, and YAML emit then parse is the identity.

### Acceptance cases R2 makes pass against ALZ_BIN=target/release/alz (R2.11)

All of these run `alz lint`. In a repository, a valid manifest and lock end with `error [not-ported]`, exit 1, until R3. No other command exists in R2's binary, so the rest of the suite fails against it until R3 to R6.
- 29 "account layer field: schema": 26 cases.
- 26: two lint cases, plus a new lint twin of the commit_identity refusal table.
- New 34-yaml-limits, mostly referenceDrift: TASK-59, TASK-60 and TASK-62 boundaries, tags, decoded control characters, documents.
- Stack-mode `ref` cases for TASK-61.
- 10's clean-layer case passes incidentally; the leak guard is R5's.

### Untrusted inputs

`layer.yaml` (layer and stack), `layer.lock`, and the shapes of `accounts.yaml` and `allowed.yaml`. Every byte of them goes through the cap, UTF-8, the budgets and the typed conversion. The `--layer` path and the working directory come from the person; `layer.yaml` is opened no-follow as a regular file.

### User-facing changes

- R2.1 writes the spec section "Output and exit codes".
- Schema errors never print values.
- Keys are cut in messages.
- New YAML errors (limits, tags, control characters, UTF-8, cyclic and unresolved aliases, numbers in text fields), and a lock with duplicate keys is invalid.

Each change is listed as drift with its task. Nothing is released in R2; cutover (R8) carries the minor 0.x bump (decision-28).

### Dependencies

All accepted. Each is added by the subtask that first needs it, never all at once:
- serde-saphyr =2.2.0 (decision-28), added by R2.4;
- rustix >= 1.1.6 (decision-27), added by R2.10;
- thiserror =2.0.21 with only `std` (decision-31), added by R2.4;
- serde =1.0.229 without derive (decision-32), added by R2.4;
- serde_json =1.0.154, parse only (decision-33), added by R2.7;
- sha2 =0.11.1 with no default features (decision-34), added by R2.7;
- proptest =1.12.0, dev-only, `std` only (decision-35), added by R2.2.

No unicode crate: the table is vendored and checked.

Open, Q7: serde-saphyr's `deserialize` feature brings in encoding_rs, licensed `(Apache-2.0 OR MIT) AND BSD-3-Clause`. `deny.toml` does not allow BSD-3-Clause, so `mise run rust:deny` fails once R2.4 adds serde-saphyr (scratch graph, 2026-11-23).

### Owner answers (2026-11-23, in the notes)

1. Keys are cut at 64 code points after stripping, then `…`.
2. The five crates above are approved.
3. Raw YAML text may hold tab, LF and CR; decoded strings only tab and LF (decision-28 amended).
4. Schema errors never print values.
5. Stack-mode lint stops with `error [not-ported]`, exit 1, until R3, and TASK-61 closes in R2 through its lint case.
6. R3 and R4 are judged by Rust tests plus the CLI cases they can reach; the suite files must pass by R5.

### Bugs

TASK-59, TASK-60 and TASK-62 are fixed by R2.4. TASK-61 is fixed by R2.6. All four depend on R2.11, which closes them with the CLI evidence; TASK-61's build case turns green in R5 (answer 5). TASK-71's parse rule lands in R2.6; TASK-71 itself closes in R3.

### Subtasks

TASK-55.3.1 to TASK-55.3.11 (R2.1 to R2.11).
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
decision-30: R2 also fixes TASK-59 to TASK-62 in Rust and implements name order (§6, TASK-64) for the lock and content hash model.

Architect (2026-11-23): plan written and split into TASK-55.3.1 to TASK-55.3.11; owner questions Q1 to Q6 in a comment; TASK-59 to TASK-62 now depend on TASK-55.3.11, which closes them.

Owner answers (2026-11-23): (1) a quoted key name is cut at 64 characters, counted after stripping, then '…'; (2) approve thiserror, serde without derive, serde_json for parsing only, sha2 and proptest (dev) — one decision each; (3) allow CR in raw YAML text, refuse it in decoded strings (amend decision-28); (4) schema errors never print values, only the field path and quoted keys (reference drift where the reference prints more); (5) stack-mode alz lint gives a fixed error [not-ported], exit 1, until R3, and TASK-61 closes in R2 through its lint case; (6) R3 and R4 are judged by Rust tests plus the acceptance cases their CLI can reach; the full suite files must pass by R5.
<!-- SECTION:NOTES:END -->

## Comments

<!-- COMMENTS:BEGIN -->
author: @architect
created: 2026-11-23T05:19:00Z

---

Owner questions for the R2 plan. R2.1 (TASK-55.3.1) waits for Q1, Q3 and Q4; R2.2 onward waits for Q2; R2.10 waits for Q5.

Q1. How long may a key name quoted in a diagnostic be? It is counted after class 1 stripping, then `…` is added (decision-28: key names are the only input text a YAML diagnostic may hold).
Options: (a) 32 code points; (b) 64, the length limit of an account name; (c) 128.
Recommendation: (b).

Q2. Five new dependencies, one decision each, with versions fixed at intake and cargo deny passing:
- thiserror 2, for alz-core and alz-io;
- serde 1 without derive, needed to write alz's own visitors for serde-saphyr and serde_json;
- serde_json 1, parse only, for layer.lock now and the JSON files of R4 and R5 later; alz writes JSON with its own emitter;
- sha2 (RustCrypto, pure Rust), for the §5.3 content hash and the §6 cache names;
- proptest 1, as a dev-dependency.
Alternatives:
- hand-written Error impls instead of thiserror;
- an alz-owned JSON parser instead of serde_json (more security-critical code to own);
- ring or aws-lc-rs instead of sha2 (C and assembly, heavier builds);
- no proptest, which conflicts with decision-26's required property tests.
Recommendation: all five.

Q3. Control characters in YAML. decision-28's default allows only tab and line feed, in raw text and in decoded strings. A file with CRLF line ends then fails, because of its raw CR.
Options:
- (a) keep the default: CRLF files are refused;
- (b) raw text also allows CR, because YAML treats it as a line break, while decoded strings allow only tab and line feed (a CR there can only come from a `\r` escape);
- (c) allow CR in both.
U+0085 (NEL) stays refused in every option.
Recommendation: (b).

Q4. Should schema errors ever print a value? The reference prints refs (`'x' is not a valid git ref`), workspace paths and every unknown field name, with no length limit.
Options:
- (a) never: name the field by its key path, with input keys quoted, stripped and cut (Q1);
- (b) print values, stripped and cut;
- (c) print values only for fields that cannot hold a secret.
Recommendation: (a). It is how the account field works already (§7.13) and follows decision-28's rule.

Q5. What R2's `alz lint` does in a repository whose manifest and lock are valid. Linting the stack's layers needs resolution, which is R3's.
Options:
- (a) a fixed `error [not-ported]` that names R3, exit 1, removed by R3. TASK-61's lint case is then checked through the CLI in R2;
- (b) no stack mode until R3. TASK-61 closes on unit tests, and its lint case turns green in R3;
- (c) keep TASK-61 open until R5 and remove it from R3's dependencies.
TASK-61 AC #1 says "lint and build error", but `build` exists only from R5. With (a) or (b), its build half is checked by the shared parser's unit tests, and the build drift case is written now.
Recommendation: (a), and accept TASK-61 closing in R2 that way.

Q6 (outside R2, found while planning). R3's and R4's criteria name suite files that mostly run `alz build` (R3: 16, 24, 28, 30; R4: 02 to 11, 17 to 19, 26, 29), but R5 builds `build`.
Options:
- (a) bring a minimal `alz build` forward into R3;
- (b) restate R3's and R4's criteria as Rust tests plus the CLI cases they can reach, so R5 carries the suite;
- (c) leave them as they are.
Recommendation: (b).

Choices I made and am reporting, not asking about:
- the Cf table is checked against a vendored extract of UCD 17.0 data, kept with its Unicode licence notice;
- the YAML emitter's golden samples are generated again by a Node tool that only reads `src/`, because the spike's samples are gone;
- the tag mechanism is the granit_parser event pre-pass. That is one of decision-28's three listed ways, and R2.4's first test pins it.

---

author: @architect
created: 2026-11-23T05:27:00Z

---

Q7 (new, from the dependency check of 2026-11-23). serde-saphyr 2.2.0's deserialize feature (decision-28) brings in encoding_rs_io and encoding_rs 0.8.44, licensed (Apache-2.0 OR MIT) AND BSD-3-Clause. deny.toml allows MIT, Apache-2.0 and Unicode-3.0 only, so mise run rust:deny fails as soon as R2.4 (TASK-55.3.4) adds serde-saphyr. Seen in a scratch crate with this repository's deny.toml; the five new crates (decision-31 to decision-35) pass licences, bans, sources and advisories there.
Options:
- (a) add BSD-3-Clause to deny.toml's allow list, with a comment naming encoding_rs through serde-saphyr, and ship its notice with the binaries (R7);
- (b) allow it only for encoding_rs, through a cargo-deny licence exception, so no other crate can bring it in unnoticed;
- (c) fall back to yaml_serde (decision-28's fallback), which has the weaknesses that decision lists.
Recommendation: (b). R2.4 is blocked on this until it is answered.

---
<!-- COMMENTS:END -->
