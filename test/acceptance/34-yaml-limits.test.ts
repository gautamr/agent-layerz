import { writeFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import { referenceDrift } from "../drift";
import { fixture, layers, type Workspace } from "../helpers";

// §13.26: the YAML limits and the other rules that make a layer.yaml invalid (§7.13), seen through
// `alz lint --layer`. A document with nothing but text in it lints; a document that needs nesting
// uses an unknown field, which the schema refuses with the code `layer`, so that case proves the
// parse itself passed by showing no `yaml` diagnostic.

const BASE = "name: limits\nkind: personal\nvisibility: public\nversion: 1.0.0\n";
const MIB = 1024 * 1024;
// Reserved marker: it must never reach any output.
const MARKER = "secret-marker-9f3a1c";

async function lint(ws: Workspace, content: string | Buffer) {
  writeFileSync(`${ws.root}/layer/layer.yaml`, content);
  return layers(ws, "layer", ["lint", "--layer", "."]);
}

/** The file holds the base plus a comment line, padded so that the whole file is `size` bytes. */
function padded(size: number): string {
  const head = `${BASE}# `;
  return `${head}${"x".repeat(size - head.length - 1)}\n`;
}

/** The parse passed: no YAML diagnostic, only the schema's refusal of the unknown field `x`. */
function expectParsed(result: { code: number; stderr: string }): void {
  expect(result.stderr).not.toContain("error [yaml");
  expect(result.stderr).toContain("error [layer]");
  expect(result.code).toBe(1);
}

function expectYamlError(result: { code: number; stderr: string }): void {
  expect(result.code).toBe(1);
  expect(result.stderr).toContain("error [yaml]");
}

/** `forbid_patterns` is a list of text, so aliases of scalars lint. */
function aliasesOverThreeAnchors(total: number): string {
  const first = Math.ceil(total / 3);
  const second = Math.ceil((total - first) / 2);
  const third = total - first - second;
  const uses = (name: string, n: number) => Array.from({ length: n }, () => `*${name}`);
  const items = [
    "&a qqa",
    "&b qqb",
    "&c qqc",
    ...uses("a", first),
    ...uses("b", second),
    ...uses("c", third),
  ];
  return `${BASE}forbid_patterns: [${items.join(", ")}]\n`;
}

const nested = (levels: number) => `${BASE}x: ${"[".repeat(levels)}${"]".repeat(levels)}\n`;

/** Scalars of a list under an unknown field; the mapping, its four pairs and the key and list of `forbid_patterns` are 11 nodes. */
function nodes(total: number): string {
  const items = Array.from({ length: total - 11 }, () => "x");
  return `${BASE}forbid_patterns: [${items.join(", ")}]\n`;
}

/** `uses` aliases of one anchor on a sequence of `size` scalars, replayed once each. */
function replays(parts: [number, number][]): string {
  const anchors = parts.map(([, size], n) => `&s${n} [${Array(size).fill("y").join(", ")}]`);
  const uses = parts.flatMap(([count], n) => Array.from({ length: count }, () => `*s${n}`));
  return `${BASE}x: [${[...anchors, ...uses].join(", ")}]\n`;
}

describe("§13.26 YAML limits (§7.13)", () => {
  it("lints a layer.yaml of exactly 1 MiB", async () => {
    const content = padded(MIB);
    expect(Buffer.byteLength(content)).toBe(MIB);
    const result = await lint(fixture("34-yaml-limits"), content);
    expect(result.stderr).toBe("");
    expect(result.code).toBe(0);
  });

  referenceDrift("TASK-62", "refuses a layer.yaml of 1 MiB and 1 byte", async () => {
    const content = padded(MIB + 1);
    expect(Buffer.byteLength(content)).toBe(MIB + 1);
    expectYamlError(await lint(fixture("34-yaml-limits"), content));
  });

  referenceDrift("TASK-62", "refuses a layer.yaml that is not valid UTF-8", async () => {
    const content = Buffer.concat([
      Buffer.from(`${BASE}# `),
      Buffer.from([0xff, 0xfe]),
      Buffer.from("\n"),
    ]);
    expectYamlError(await lint(fixture("34-yaml-limits"), content));
  });

  it("lints a layer.yaml with CRLF line ends", async () => {
    const result = await lint(fixture("34-yaml-limits"), BASE.replaceAll("\n", "\r\n"));
    expect(result.stderr).toBe("");
    expect(result.code).toBe(0);
  });

  it("lints 100 aliases over three anchors", async () => {
    const result = await lint(fixture("34-yaml-limits"), aliasesOverThreeAnchors(100));
    expect(result.stderr).toBe("");
    expect(result.code).toBe(0);
  });

  referenceDrift("TASK-62", "refuses 101 aliases over three anchors", async () => {
    expectYamlError(await lint(fixture("34-yaml-limits"), aliasesOverThreeAnchors(101)));
  });

  it("parses a document nested 64 levels deep", async () => {
    expectParsed(await lint(fixture("34-yaml-limits"), nested(63)));
  });

  referenceDrift("TASK-60", "refuses a document nested 65 levels deep", async () => {
    expectYamlError(await lint(fixture("34-yaml-limits"), nested(64)));
  });

  it("lints a document of 20000 nodes", async () => {
    const result = await lint(fixture("34-yaml-limits"), nodes(20000));
    expect(result.stderr).toBe("");
    expect(result.code).toBe(0);
  });

  referenceDrift("TASK-62", "refuses a document of 20001 nodes", async () => {
    expectYamlError(await lint(fixture("34-yaml-limits"), nodes(20001)));
  });

  it("parses a document that replays 10000 events through aliases", async () => {
    // 99 uses of a sequence of 98 scalars (100 events each) and one use of another of the same size.
    expectParsed(
      await lint(
        fixture("34-yaml-limits"),
        replays([
          [99, 98],
          [1, 98],
        ]),
      ),
    );
  });

  referenceDrift(
    "TASK-62",
    "refuses a document that replays 10001 events through aliases",
    async () => {
      expectYamlError(
        await lint(
          fixture("34-yaml-limits"),
          replays([
            [99, 98],
            [1, 99],
          ]),
        ),
      );
    },
  );
});

describe("§13.26 what else makes a layer.yaml invalid (§7.13)", () => {
  referenceDrift("TASK-59", "refuses an unresolved alias and never prints its name", async () => {
    const result = await lint(fixture("34-yaml-limits"), `${BASE}description: *${MARKER}\n`);
    expectYamlError(result);
    expect(result.stdout).not.toContain(MARKER);
    expect(result.stderr).not.toContain(MARKER);
  });

  referenceDrift("TASK-60", "refuses a cyclic alias", async () => {
    expectYamlError(await lint(fixture("34-yaml-limits"), `${BASE}x: &a [*a]\n`));
  });

  referenceDrift("TASK-55.3", "refuses a local tag with the code yaml-tag", async () => {
    const result = await lint(fixture("34-yaml-limits"), `${BASE}description: !foo bar\n`);
    expect(result.code).toBe(1);
    expect(result.stderr).toContain("error [yaml-tag]");
    expect(result.stderr).not.toContain("foo");
  });

  referenceDrift("TASK-55.3", "refuses !!binary with the code yaml-tag", async () => {
    const result = await lint(fixture("34-yaml-limits"), `${BASE}description: !!binary aGk=\n`);
    expect(result.code).toBe(1);
    expect(result.stderr).toContain("error [yaml-tag]");
  });

  referenceDrift("TASK-55.3", "refuses an escape that decodes to a control character", async () => {
    expectYamlError(await lint(fixture("34-yaml-limits"), `${BASE}description: "a\\x07b"\n`));
  });

  it("refuses a file with two documents", async () => {
    expectYamlError(await lint(fixture("34-yaml-limits"), `${BASE}---\n${BASE}`));
  });
});
