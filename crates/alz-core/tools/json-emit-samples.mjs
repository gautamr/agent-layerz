// Regenerates the golden samples of alz's stable JSON emitter (decision-33, TASK-55.3.7): what
// the TypeScript reference (`stableJson` in src/core/stable-json.ts) writes for a corpus of what
// alz writes. The Rust emitter (crates/alz-core/src/render/json.rs) must reproduce every sample
// byte for byte.
//
// Run from the repository root after `npm install`, and commit the output:
//
//   node crates/alz-core/tools/json-emit-samples.mjs
//
// The script reads src/ and node_modules and changes neither. It writes one file,
// crates/alz-core/tests/fixtures/json-emit/samples.json: a list of `{ name, input, json }`, where
// `input` is the value as JSON and `json` is the reference's output. A Rust test cannot list a
// directory (alz-core bans the file system), so the samples are one file read with `include_str!`.
//
// Left out, because the Rust emitter deliberately differs (reference drift, TASK-64):
// - keys that differ above U+FFFF and in U+E000-U+FFFF (the reference orders by UTF-16 unit);
// - an object with a whole-number key next to any other key (JavaScript moves it first).
// The script refuses a corpus that holds either.

import { execFileSync } from "node:child_process";
import { mkdtempSync, writeFileSync } from "node:fs";
import { createRequire } from "node:module";
import { tmpdir } from "node:os";
import { join } from "node:path";

const outFile = "crates/alz-core/tests/fixtures/json-emit/samples.json";

const bundleDir = mkdtempSync(join(tmpdir(), "alz-json-emit-"));
const bundle = join(bundleDir, "stable-json.cjs");
execFileSync(
  "node_modules/.bin/esbuild",
  [
    "src/core/stable-json.ts",
    "--bundle",
    "--platform=node",
    "--format=cjs",
    `--outfile=${bundle}`,
    "--log-level=warning",
  ],
  { stdio: "inherit" },
);
const { stableJson } = createRequire(import.meta.url)(bundle);

const isIndexKey = (key) => /^(0|[1-9][0-9]*)$/.test(key) && Number(key) < 2 ** 32 - 1;

function checkKeys(value, where) {
  if (Array.isArray(value)) {
    for (const [index, item] of value.entries()) checkKeys(item, `${where}[${index}]`);
  } else if (typeof value === "object" && value !== null) {
    const keys = Object.keys(value);
    if (keys.length > 1 && keys.some(isIndexKey)) {
      throw new Error(`${where}: a whole-number key next to others is reference drift`);
    }
    const astral = keys.some((key) => /[\u{10000}-\u{10ffff}]/u.test(key));
    const high = keys.some((key) => /[-￿]/u.test(key));
    if (astral && high) throw new Error(`${where}: keys above U+FFFF and in U+E000-U+FFFF`);
    for (const [key, item] of Object.entries(value)) checkKeys(item, `${where}.${key}`);
  }
}

const corpus = [
  ["null", null],
  ["true", true],
  ["false", false],
  ["integer zero", 0],
  ["negative integer", -42],
  ["large integer", 9007199254740991],
  ["float", 1.5],
  ["negative float", -0.5],
  ["float below one", 0.000001],
  ["float with a small exponent", 1e-7],
  ["float with an exponent and digits", 1.2345e-10],
  ["whole number above 2^53", 12345678901234567000],
  ["float at the exponent limit", 1e21],
  ["float with an exponent and digits above", 1.5e300],
  ["smallest float", 5e-324],
  ["largest float", 1.7976931348623157e308],
  ["empty text", ""],
  ["plain text", "hello"],
  ["escapes", 'quote " backslash \\ b \b f \f n \n r \r t \t'],
  ["control characters", "\u0000\u0001\u001f\u007f\u0080"],
  ["line separators are not escaped", "a b c"],
  ["non-ascii text", "ü 中 😀 �"],
  ["empty array", []],
  ["empty object", {}],
  ["array of values", [1, "two", null, [], {}, [3]]],
  ["nested empties", { a: [], b: {}, c: [[]] }],
  ["keys are sorted", { b: 1, a: 2, B: 3, _: 4, "-": 5, "a-b": 6, "a/b": 7 }],
  ["keys sort by code point", { é: 1, e: 2, z: 3, 中: 4, "�": 5, "￿": 6 }],
  ["a numeric looking key next to nothing", { 10: "x" }],
  ["keys that look like numbers but are not indexes", { "-1": 1, "01": 2, 1.5: 3, a: 4 }],
  ["keys with escapes", { 'a"b': 1, "a\nb": 2, "a\\b": 3 }],
  ["nested objects", { z: { y: { x: [1, { b: 1, a: 2 }] } }, a: null }],
  [
    "a lock",
    {
      lockVersion: 1,
      visibility: "public",
      layers: [
        {
          name: "personal",
          source: "path:../p",
          rev: "v1",
          sha: "b8b1dadfa8feded6f6d93583dd449585eaafb9f8",
          contentHash: `sha256:${"0".repeat(64)}`,
        },
      ],
      files: { "b.md": `sha256:${"1".repeat(64)}`, "a/b.md": `sha256:${"2".repeat(64)}` },
    },
  ],
];

const samples = corpus.map(([name, input]) => {
  checkKeys(input, name);
  return { name, input, json: stableJson(input) };
});

writeFileSync(outFile, `${JSON.stringify(samples, null, 1)}\n`);
// The repository's formatter checks every JSON file, so the output is formatted the way it wants.
execFileSync("node_modules/.bin/biome", ["format", "--write", outFile], { stdio: "inherit" });
console.log(`wrote ${samples.length} samples to ${outFile}`);
