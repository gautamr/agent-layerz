// Regenerates the golden samples of alz's YAML emitter (decision-28, TASK-55.3.8): what the
// TypeScript reference (`stringifyYaml` in src/core/yaml.ts: lineWidth 0, minContentWidth 0) writes
// for a corpus of what alz writes. The Rust emitter (crates/alz-core/src/render/yaml.rs) must
// reproduce every sample byte for byte.
//
// Run from the repository root after `npm install`, and commit the output:
//
//   node crates/alz-core/tools/yaml-emit-samples.mjs
//
// The script reads src/ and node_modules and changes neither. It writes one file,
// crates/alz-core/tests/fixtures/yaml-emit/samples.json: a list of
// `{ name, class, input, yaml }`, where `input` is the value as JSON (strings, booleans, integers,
// lists and objects with their keys in the order the reference wrote them) and `yaml` is the
// reference's output. A Rust test cannot list a directory (alz-core bans the file system), so
// the samples are one file read with `include_str!`.
//
// Left out: an object with a whole-number key next to any other key. JavaScript orders such keys
// first, in numeric order, before the emitter sees them; what order alz writes them in is decided
// by the caller (TASK-66, R5). A map with one whole-number key and nothing else is in the corpus.

import { execFileSync } from "node:child_process";
import { mkdirSync, mkdtempSync, writeFileSync } from "node:fs";
import { createRequire } from "node:module";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";

const outFile = "crates/alz-core/tests/fixtures/yaml-emit/samples.json";

const bundleDir = mkdtempSync(join(tmpdir(), "alz-yaml-emit-"));
const bundle = join(bundleDir, "yaml.cjs");
execFileSync(
  "node_modules/.bin/esbuild",
  [
    "src/core/yaml.ts",
    "--bundle",
    "--platform=node",
    "--format=cjs",
    `--outfile=${bundle}`,
    "--log-level=warning",
  ],
  { stdio: "inherit" },
);
const { stringifyYaml } = createRequire(import.meta.url)(bundle);

/** A whole-number key in the sense of JavaScript's property order. */
const isIndexKey = (key) => /^(0|[1-9][0-9]*)$/.test(key) && Number(key) < 2 ** 32 - 1;

function checkOrder(value, where) {
  if (Array.isArray(value)) {
    for (const [index, item] of value.entries()) checkOrder(item, `${where}[${index}]`);
  } else if (value !== null && typeof value === "object") {
    const keys = Object.keys(value);
    if (keys.length > 1 && keys.some(isIndexKey)) {
      throw new Error(`${where}: a whole-number key next to others is left out (TASK-66)`);
    }
    for (const key of keys) checkOrder(value[key], `${where}.${JSON.stringify(key)}`);
  } else if (typeof value === "number") {
    if (!Number.isFinite(value) || Object.is(value, -0)) {
      throw new Error(`${where}: the sample file is JSON, which has no ${value}`);
    }
    if (Number.isInteger(value) && !Number.isSafeInteger(value) && Math.abs(value) < 2 ** 63) {
      throw new Error(
        `${where}: an integer between 2^53 and 2^63 is exact in alz and not in the reference`,
      );
    }
  }
}

const samples = [];
const names = new Set();

/** One sample: the reference's output for `value`. */
function sample(cls, name, value, drift) {
  const full = `${cls}/${name}`;
  if (names.has(full)) throw new Error(`duplicate sample ${full}`);
  names.add(full);
  checkOrder(value, full);
  const written = { name: full, class: cls, input: value, yaml: stringifyYaml(value) };
  samples.push(drift === undefined ? written : { ...written, drift });
}

const escapedRaw = /[\u007f-\u009f\u2028\u2029]/;
const blankBetweenLineFeeds = /\n \n/;

/**
 * Reference drift of a text (TASK-73): why alz writes it differently from the reference, or
 * undefined when it does not. `isKey` says the text is a mapping key.
 *
 * - U+007F to U+009F, U+2028 and U+2029: the reference writes them raw, alz double-quotes the text
 *   and escapes them (spec section 10.2, rule 1).
 * - A space between two line feeds in long double-quoted text: the reference writes a backslash
 *   twice and a space, which reads back as a backslash. Only a class that holds such text asks for
 *   this check (`blankBetween`), because whether a text is double-quoted is not decided here; the
 *   Rust test checks that every sample marked as drift is written differently from the reference.
 */
function driftOfText(text, isKey, blankBetween) {
  if (escapedRaw.test(text)) return "escaped-characters-the-reference-writes-raw";
  if (
    blankBetween &&
    !isKey &&
    blankBetweenLineFeeds.test(text) &&
    JSON.stringify(text).length >= 40
  ) {
    return "space-between-line-feeds-in-long-double-quoted-text";
  }
  return undefined;
}

/** One scalar class, written as a map value, a map key and a list item. */
function scalarClass(cls, texts, { blankBetween = false } = {}) {
  const unique = [...new Set(texts)];
  const keyable = unique.filter((text) => !isIndexKey(text));
  const firstDrift = (list, isKey) =>
    list.map((text) => driftOfText(text, isKey, blankBetween)).find((reason) => reason);
  sample(
    cls,
    "as-values",
    Object.fromEntries(unique.map((text, index) => [`k${index}`, text])),
    firstDrift(unique, false),
  );
  // Whole-number keys next to others are left out; each is in a sample of its own below.
  sample(
    cls,
    "as-keys",
    Object.fromEntries(keyable.map((text) => [text, "x"])),
    firstDrift(keyable, true),
  );
  sample(cls, "as-items", unique, firstDrift(unique, false));
  sample(
    cls,
    "as-nested",
    {
      outer: { inner: unique },
      list: [Object.fromEntries(unique.map((text, index) => [`k${index}`, text]))],
    },
    firstDrift(unique, false),
  );
  for (const text of unique) {
    sample(
      `${cls}-single`,
      `value ${JSON.stringify(text)}`,
      { key: text },
      driftOfText(text, false, blankBetween),
    );
    sample(
      `${cls}-single`,
      `key ${JSON.stringify(text)}`,
      { [text]: "v" },
      driftOfText(text, true, blankBetween),
    );
  }
}

// ---- text that the reference writes plainly and alz's reader reads as something else ----
//
// Each of these is a sample whose reference output is NOT what alz writes: alz quotes the text so
// that what it writes it reads back as the same value (the round-trip law, TASK-55.3.8). The
// Rust test checks that alz's output differs and reads back, and pins alz's output for each.
const drifts = {
  // YAML 1.2 core has no binary integers and no signed octal or hexadecimal; alz's reader has.
  "integer-forms-only-alz-reads": [
    "0b101",
    "0b1",
    "0b0",
    "+0b1",
    "-0b1",
    "+0b101",
    "+0o17",
    "-0o17",
    "+0x1f",
    "-0x1f",
  ],
  // alz's reader refuses text that starts with a byte order mark.
  "byte-order-mark-first": ["\ufeff", "\ufeffa", "\ufeff\ufeff", "\ufeff x"],
  // A block of only spaces and line feeds loses its spaces when read (as a key it is quoted anyway).
  "blank-multi-line": [" \n", "  \n", "\n \n", " \n\n", "  \n\n  \n", " \n".repeat(20)],
};
for (const [name, texts] of Object.entries(drifts)) {
  for (const text of texts) {
    sample(
      "reference-reads-differently",
      `${name} ${JSON.stringify(text)} as value`,
      { key: text },
      name,
    );
    if (name !== "blank-multi-line") {
      sample(
        "reference-reads-differently",
        `${name} ${JSON.stringify(text)} as key`,
        { [text]: "v" },
        name,
      );
    }
    sample("reference-reads-differently", `${name} ${JSON.stringify(text)} as item`, [text], name);
  }
}

// ---- structure: what alz writes ----

sample("structure", "agent-frontmatter", {
  name: "reviewer",
  description: "Reviews a change and reports problems, never edits.",
  tools: "Read, Grep, Glob, Bash",
  model: "sonnet",
  permissionMode: "plan",
  skills: ["gr-rust", "add-acceptance-test"],
  isolation: "worktree",
  memory: "project",
});
sample("structure", "agent-native-map", {
  name: "architect",
  description: "Plans work.",
  hooks: { PreToolUse: [{ matcher: "Bash", hooks: [{ type: "command", command: "echo hi" }] }] },
  mcpServers: { backlog: { command: "backlog", args: ["mcp", "start"], env: { A: "1" } } },
  maxTurns: 25,
  background: true,
});
sample("structure", "rule-paths", { paths: ["crates/**", "xtask/**", "Cargo.toml", "mise.toml"] });
sample("structure", "rule-paths-with-indicators", {
  paths: ["**/*.rs", "{a,b}/*.md", "!src/**", "[abc]/x", "*.md", "#hidden", "a b/c"],
});
sample("structure", "skill-frontmatter", {
  name: "next-task",
  description: "Pick the next unblocked task: reads the board, then routes it.",
  "argument-hint": "[task id]",
  "disable-model-invocation": true,
});
sample("structure", "skill-slot-stub", {
  name: "deploy",
  description: "Slot stub: deploy is not defined for this repo",
});
sample("structure", "allowed-yaml", [
  {
    root: "/home/user/dev/project-a",
    account: ["work", "personal"],
    source: "~/.config/alz/accounts.yaml",
  },
  {
    root: "/home/user/dev/project-b",
    account: ["personal"],
    source: "~/.config/alz/accounts.yaml",
  },
]);
sample("structure", "allowed-yaml-empty", []);
sample("structure", "empty-map", {});
sample("structure", "nested-empty-collections", {
  a: {},
  b: [],
  c: [[]],
  d: [{}],
  e: { f: {}, g: [] },
});
sample("structure", "list-of-maps", [
  { a: 1, b: "x" },
  { c: [1, 2], d: { e: true } },
]);
sample("structure", "list-of-lists", [[1, 2], [[3], [4, [5]]], []]);
sample("structure", "deep-nesting", { a: { b: { c: { d: { e: { f: ["x", { g: "y" }] } } } } } });
sample("structure", "booleans-and-integers", {
  yes: true,
  no: false,
  zero: 0,
  negative: -5,
  big: 9007199254740991,
  small: -9007199254740991,
  list: [true, false, 0, 1, -1],
});
sample("structure", "map-in-list-in-map", { a: [{ b: [{ c: "d" }] }] });
sample("structure", "single-whole-number-key", { 1: "x" });
sample("structure", "single-whole-number-key-nested", { a: { 10: ["y"] } });
sample("structure", "single-leading-zero-key", { "007": "x" });
sample("structure", "mixed-collection-values", {
  list: ["a", { b: "c" }, ["d"], 1, true, ""],
  map: { x: [], y: {}, z: "" },
});

// ---- numbers with a fraction or an exponent, and null (alz writes what the reference writes) ----

sample("numbers", "fractions", {
  half: 0.5,
  negative: -0.5,
  quarter: 100.25,
  sum: 0.1 + 0.2,
  small: 0.000001234,
  list: [0.5, -1.5, 3.25],
});
sample("numbers", "exponents", {
  large: 1e21,
  larger: 1.5e300,
  largest: Number.MAX_VALUE,
  small: 1e-7,
  smaller: -1.5e-10,
  smallest: Number.MIN_VALUE,
});
sample("numbers", "whole-numbers-beyond-64-bits-or-written-without-exponent", {
  a: 1e20,
  b: 123456789012345680000,
  c: -1e20,
});
sample("numbers", "whole-numbers-with-a-fraction-in-the-source", { a: 1.0, b: 100.0, c: -2.0 });
sample("numbers", "null", {
  a: null,
  b: [null, 1],
  c: { d: null, e: {} },
});
sample("numbers", "null-and-numbers-in-lists", [null, 0.5, [null], { a: 0.5 }]);

// ---- top-level scalars (not what alz writes today; the emitter is total over its values) ----

for (const [name, value] of [
  ["word", "x"],
  ["yes", "yes"],
  ["number-like", "123"],
  ["empty", ""],
  ["multi-line", "a\nb"],
  ["multi-line-final-lf", "a\nb\n"],
  ["document-marker", "---"],
  ["document-marker-after-line", "a\n---\nb"],
  ["control", "a\u0007b"],
  ["true", true],
  ["false", false],
  ["integer", 42],
  ["negative", -7],
  ["float", 0.5],
  ["null", null],
]) {
  sample("top-level", name, value);
}

// ---- scalar classes ----

scalarClass("plain-words", [
  "word",
  "two words",
  "many words in a row",
  "a-b",
  "a.b",
  "a/b",
  "path/to/file.md",
  "a_b",
  "a=b",
  "a+b",
  "a<b>c",
  "a|b",
  "a&b",
  "a*b",
  "a!b",
  "a%b",
  "a@b",
  "a`b",
  "a?b",
  "a,b",
  "a [b]",
  "a{b}",
  "a ? b",
  "<<",
  "=",
  "a - b",
  "x:y",
  "http://example.com/a?b=c#d",
  "100%",
  "Dr. Who",
]);

scalarClass("yaml-1-1-words", [
  "y",
  "n",
  "Y",
  "N",
  "yes",
  "no",
  "Yes",
  "No",
  "YES",
  "NO",
  "on",
  "off",
  "On",
  "Off",
  "ON",
  "OFF",
  "tRue",
  "fAlse",
  "nULL",
]);

scalarClass("core-schema-scalars", [
  "true",
  "false",
  "True",
  "False",
  "TRUE",
  "FALSE",
  "null",
  "Null",
  "NULL",
  "~",
  "",
  "0",
  "123",
  "-1",
  "+1",
  "-0",
  "007",
  "0o17",
  "0o8",
  "0x10",
  "0xFF",
  "0xfg",
  "1.5",
  ".5",
  "1.",
  "-.5",
  "+1.5",
  "1e3",
  "1E-3",
  "1.5e+3",
  ".5e2",
  "1e",
  ".inf",
  ".Inf",
  ".INF",
  "-.inf",
  "+.inf",
  ".nan",
  ".NaN",
  ".NAN",
  ".iNf",
  "1_000",
  "1.0.0",
  "12:30",
  "1:2:3",
  "2024-01-01",
  "2024-01-01T10:00:00Z",
  "0B1",
  "1,000",
  "--1",
  "1-2",
  "+",
  "1 2",
  "0.",
  "0x",
  "0o",
  "٣",
]);

scalarClass("indicators", [
  "- a",
  "-a",
  "--",
  "---a",
  "-",
  "-\t",
  "?",
  "? a",
  "?a",
  "?\ta",
  ", a",
  "[a",
  "]a",
  "{a",
  "}a",
  "#a",
  "&a",
  "*a",
  "!a",
  "|a",
  ">a",
  "'a",
  '"a',
  "%a",
  "@a",
  "`a",
  ":a",
  ": a",
  ":",
  "a:",
  "a: b",
  "a:b",
  "a :b",
  "a:\tb",
  "a #b",
  "a#b",
  "a\t#b",
  "a ",
  " a",
  "a\t",
  "\ta",
  " ",
  "  ",
  "a  b",
  "a\tb",
  "#",
  " #",
]);

scalarClass("quotes", [
  "it's",
  'say "hi"',
  `both ' and "`,
  "'",
  '"',
  "''",
  '""',
  "a\\b",
  "\\",
  "\\n",
  '"a"',
  "'a'",
  "a'",
  'a"',
  "'a' and 'b'",
  '"a" and "b"',
  "a\\\"b'c",
  "x\\",
]);

scalarClass("multi-line", [
  "a\nb",
  "a\nb\n",
  "a\nb\n\n",
  "a\nb\n\n\n",
  "a\n\nb",
  "a\n\n\nb",
  "\nb",
  "\n\nb\n",
  "\n",
  "\n\n",
  "a\n ",
  "a\n \n",
  "\t\na",
  " a\nb",
  "  a\nb\n",
  "a\n b",
  "a\n  b\n",
  "a \nb",
  "a\t\nb",
  "a\nb ",
  "a\nb\t",
  "a\n\tb",
  "a\n\nb\n\n",
  "line one\nline two\nline three",
  "key: value\nother: value",
  "a\n#b",
  "a\n- b",
  "a\n? b",
  "a\n: b",
  "a\n'b'",
  'a\n"b"',
  "first\n\n\n\nlast\n",
  "\n  x",
  "\n\n  x\n",
  "\n \nx",
  "x\n\n  ",
  "x\n  \ny",
  "a\n  \n  b\n",
  `${"long line ".repeat(30)}\nnext`,
  "a".repeat(200),
  "word ".repeat(60).trimEnd(),
  "\n".repeat(5),
  "x\n".repeat(30),
  "- a\n- b\n- c\n",
  "# heading\n\nbody text\n",
  "```\ncode\n```\n",
  "a\\nb\n",
  "tab\there\nand\nthere",
]);

scalarClass("document-markers", [
  "---",
  "--- a",
  "---\n",
  "...",
  "... a",
  "a\n---",
  "a\n--- b",
  "a\n...",
  "a\n... b",
  "---\na",
  "...\na",
  "%a\nb",
  "a\n%b",
  "a\r---",
  "a\u2028---",
  "a\u2029...",
  "a ---",
  "a---",
  "--- ",
]);

scalarClass("control-characters", [
  "\u0000",
  "a\u0000b",
  "\u0001",
  "a\u0007b",
  "\u0008",
  "\t",
  "a\tb",
  "\u000b",
  "a\u000bb",
  "\u000c",
  "\r",
  "a\rb",
  "a\r\nb",
  "a\r\nb\r\n",
  "\u001b[0m",
  "\u001b",
  "\u001f",
  "\u007f",
  "\u0080",
  "\u0085",
  "a\u0085b",
  "\u009f",
  "\u00a0",
  "a\u00a0b",
  "\u0001\n",
  "a\n\u0001",
  "\u0001a\nb",
  "a\u0000\nb",
]);

scalarClass("unicode", [
  "é",
  "日本語",
  "👍🏽",
  "👨‍👩‍👧",
  "\u2028",
  "\u2029",
  "a\u2028b",
  "a\u2029b",
  "\u2028\n",
  "a\ufeffb",
  "\u200d",
  "\u200b",
  "\u202e",
  "ñandú: café",
  "é\nè",
  "- é",
  "é:",
  "\u{1F600}",
  "\u{10FFFF}",
  "\ufffd",
  "\u3000",
  "a\u3000",
  "\u2003x",
]);

// Double-quoted text with real line breaks: a string with a control character is written in
// double quotes, and the reference folds a line feed there when the string is 40 or more long.
const forty = "0123456789".repeat(4);
scalarClass("double-quoted-line-breaks", [
  "a\nb\u0001",
  `${forty}\nb\u0001`,
  `${forty}\n\nb\u0001`,
  `${forty}\n\n\nb\u0001`,
  `${forty}\n\n\n\nb\u0001`,
  `${forty} \nb\u0001`,
  `${forty}\n b\u0001`,
  `${forty}\n  b\u0001`,
  `${forty}\n`,
  `${forty}\n\u0001`,
  `${forty}\nb\n\u0001`,
  `${forty}\nb\n\n\u0001\n`,
  `\u0001${forty}\n${forty}`,
  `\u0001${forty}\n`,
  `\u0001${forty}\n\n`,
  `\u0001${forty}\n\nx`,
  `\u0001${forty}\n  \nx`,
  `\u0001${forty}\n\tx`,
  `\u0001${forty} \n x`,
  `x\u0001\n${forty}\n${forty}\n${forty}`,
  `\n${forty}\u0001`,
  `\n\n${forty}\u0001`,
  `${"a\n".repeat(20)}\u0001`,
  `${"a \n".repeat(20)}\u0001`,
  `${"a\n\n".repeat(20)}\u0001`,
  `${forty}\r\n${forty}`,
  `${forty}\u0001\r${forty}`,
  `${"é".repeat(19)}\nb\u0001`,
  `${"é".repeat(20)}\nb\u0001`,
  `${"👍".repeat(10)}\nb\u0001`,
  `${"👍".repeat(9)}\nb\u0001`,
  `"${forty}"\n'x'\u0001`,
  `${forty}\\n\n\u0001`,
]);

// A space between two line feeds in long double-quoted text (TASK-73 drift): the reference writes
// it as a backslash twice and a space, which reads back as a backslash. The texts are all
// double-quoted: they hold U+0001, or end in blanks after a line feed, which a block cannot keep.
// The short ones and the keys are no drift; the Rust test checks the rest differ.
scalarClass(
  "double-quoted-blank-between-line-feeds",
  [
    `${forty}\n \nb\u0001`,
    `${forty}\n \n\nb\u0001`,
    `${forty}\n\n \nb\u0001`,
    `${forty}\n \n \nb\u0001`,
    `${forty} \n \nb\u0001`,
    `${forty}\n  \nb\u0001`,
    `${forty}\n \n\u0001`,
    `\u0001${forty}\n \nx`,
    `${forty}\n \nb\n `,
    `${forty}\n \n\nb\n `,
    `${forty}\n\n \nb\n\n `,
    "a\n \nb\u0001",
    `${"é".repeat(19)}\n \nb\u0001`,
  ],
  { blankBetween: true },
);

// Long double-quoted text with characters the reference writes raw and alz escapes (TASK-73 drift).
scalarClass("escaped-characters-in-long-text", [
  `${forty}\n\u0085x`,
  `\u2028${forty}\nb`,
  `${forty}\u2029\nb`,
  `${forty}\n\u007f`,
  `\u009f\n${forty}\nx`,
  `${forty}\n \n\u0085`,
  `${forty}\u0080\u0081\u0082`,
]);

// Keys that need care, and keys at the length that makes the reference write `? key`.
sample("keys", "simple", { a: 1, "a b": 2, "a-b": 3, a_b: 4, "a.b": 5, é: 6 });
sample("keys", "empty-key", { "": "x" });
sample("keys", "empty-key-with-others", { before: 1, "": [1, 2], after: { "": "y" } });
sample("keys", "empty-key-with-block-value", { "": "a\nb\n" });
sample("keys", "empty-key-with-map-value", { "": { a: 1, b: [2] } });
sample("keys", "empty-key-in-list", [{ "": "x", a: "y" }]);
sample("keys", "multi-line-key", { "a\nb": "x" });
sample("keys", "multi-line-key-block", { "a\nb\n": "x" });
sample("keys", "multi-line-key-spaces", { "a \nb": "x", "a\n b": "y" });
sample("keys", "multi-line-key-control", { "a\nb\u0001": "x", [`${forty}\nb\u0001`]: "y" });
sample("keys", "control-key", { "\u0001": "x", "a\u0000": "y" });
sample("keys", "document-marker-top-level", { "---": "x" });
sample("keys", "document-marker-top-level-prefix", { "--- a": "x" });
sample("keys", "document-marker-nested", { a: { "---": "x", "...": "y", "--- a": "z" } });
sample("keys", "document-marker-in-list", [{ "---": "x" }]);
sample("keys", "yaml-keys", {
  true: 1,
  false: 2,
  null: 3,
  "~": 4,
  yes: 5,
  no: 6,
  on: 7,
  off: 8,
  1.5: 9,
  ".inf": 10,
});
sample("keys", "single-number-key", { 123: "x" });
sample("keys", "single-true-key", { true: "x" });
for (const length of [1023, 1024, 1025, 1100]) {
  const key = "k".repeat(length);
  sample("keys", `long-${length}`, { [key]: "x" });
  sample("keys", `long-${length}-in-list`, [{ [key]: ["x"] }]);
  sample("keys", `long-${length}-nested`, { a: { [key]: { b: 1 } } });
}
sample("keys", "long-quoted-key", { [`${"k".repeat(1020)} : x`]: "v" });
sample("keys", "long-key-of-wide-characters", { ["é".repeat(1024)]: "x", ["👍".repeat(600)]: "y" });
sample("keys", "long-key-with-block-value", { ["k".repeat(1030)]: "a\nb\n" });
sample("keys", "long-key-with-list-value", { ["k".repeat(1030)]: ["a", "b"] });
sample("keys", "long-key-with-empty-value", { ["k".repeat(1030)]: [], other: {} });
sample("keys", "yes-no-keys-nested", { a: { y: 1, n: 2, "": 3 } });

// Indentation of block text at every level.
const block = "first\nsecond\n";
sample("indentation", "block-at-top", { a: block });
sample("indentation", "block-in-map", { a: { b: block } });
sample("indentation", "block-deep", { a: { b: { c: { d: block } } } });
sample("indentation", "block-in-list", [block, [block, [block]]]);
sample("indentation", "block-in-list-in-map", { a: [block, { b: block, c: [block] }] });
sample("indentation", "block-with-leading-spaces-deep", {
  a: { b: ["  x\ny\n", "\n  x\n", "x\n\n"] },
});
sample("indentation", "block-keep-and-strip", {
  a: ["x\ny\n\n", "x\ny", "x\ny\n\n\n"],
  b: { c: "x\n\n" },
});
sample("indentation", "blank-lines-in-block", { a: { b: "x\n\n\ny\n\nz\n" } });
sample("indentation", "single-quoted-multi-line", {
  a: { b: 'say "hi"\nthere' },
  c: ['say "hi"\n\nthere'],
});
sample("indentation", "single-quoted-multi-line-spaces", {
  a: { b: 'say "hi" \nthere', c: 'say "hi"\n there' },
});
sample("indentation", "double-quoted-deep", {
  a: { b: [`\u0001${forty}\nx`, `${forty}\n\n\nb\u0001`] },
});
sample("indentation", "document-marker-in-block-deep", { a: { b: "x\n---\ny\n", c: "...\nz" } });
sample("indentation", "document-marker-line-in-single-quoted", { a: 'say "x"\n---\ny' });
sample("indentation", "long-line-not-folded", {
  a: "word ".repeat(80).trimEnd(),
  b: ["word ".repeat(80).trimEnd()],
  c: { d: `${"word ".repeat(80)}\nsecond` },
});
sample("indentation", "long-quoted-not-folded", {
  a: `it's ${"word ".repeat(80)}`,
  b: `${"word ".repeat(80)}: x`,
});
sample("indentation", "long-double-quoted-not-folded", {
  a: `${"é".repeat(100)}\u0001`,
  b: `\u0001${"word ".repeat(80)}`,
});
sample("indentation", "list-of-block-maps", [{ a: block, b: 1 }, { c: { d: block } }]);

mkdirSync(dirname(outFile), { recursive: true });
// The repository's formatter ignores the fixture folders, so the output is formatted through
// standard input under a path it does not ignore, the way it formats every other JSON file.
const formatted = execFileSync(
  "node_modules/.bin/biome",
  ["format", "--stdin-file-path=crates/alz-core/tools/samples.json"],
  { input: JSON.stringify(samples, null, 1), encoding: "utf8" },
);
writeFileSync(outFile, formatted.endsWith("\n") ? formatted : `${formatted}\n`);
console.log(`${samples.length} samples written to ${outFile}`);
