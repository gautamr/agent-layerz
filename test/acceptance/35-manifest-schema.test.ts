import { writeFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import { referenceDrift } from "../drift";
import { fixture, layers, readTree, type Workspace } from "../helpers";

// §13.27: the layer and manifest schema (§5.1, §5.2). A schema error names the field by its key
// path and never prints the value (§10.1).

// Reserved marker: a refused value must never appear in any output.
const MARKER = "refmarker7";

/** The repository's manifest, with `ref` on the second stack entry (`stack[1]`). */
function setRef(ws: Workspace, ref: string): void {
  writeFileSync(
    `${ws.root}/repo/layer.yaml`,
    `stack:\n  - source: path:../personal\n  - source: path:./.layer\n    ref: ${ref}\ntargets: [claude]\n`,
  );
}

const REFUSED_REFS: [string, string, string][] = [
  ["a list", `[${MARKER}]`, MARKER],
  ["a number", "1.10", "1.10"],
];

describe("§13.27 stack entry ref (§5.2, TASK-61)", () => {
  describe.each(REFUSED_REFS)("ref as %s", (_label, ref, value) => {
    referenceDrift(
      "TASK-61",
      "alz lint refuses it as a manifest error naming the entry",
      async () => {
        const ws = fixture("35-manifest-schema");
        setRef(ws, ref);
        const result = await layers(ws, "repo", ["lint"]);
        expect(result.code).toBe(1);
        expect(result.stderr).toContain("error [manifest]");
        expect(result.stderr).toContain("stack[1].ref");
        expect(result.stdout).not.toContain(value);
        expect(result.stderr).not.toContain(value);
      },
    );

    // alz build exists in the Rust alz from R5 (TASK-55.6); until then this case fails against it.
    referenceDrift(
      "TASK-61",
      "alz build refuses it, names the entry and writes nothing",
      async () => {
        const ws = fixture("35-manifest-schema");
        setRef(ws, ref);
        const tree = readTree(`${ws.root}/repo`);
        const result = await layers(ws, "repo", ["build"]);
        expect(result.code).toBe(1);
        expect(result.stderr).toContain("error [manifest]");
        expect(result.stderr).toContain("stack[1].ref");
        expect(result.stdout).not.toContain(value);
        expect(result.stderr).not.toContain(value);
        expect(readTree(`${ws.root}/repo`)).toEqual(tree);
      },
    );
  });

  it("alz lint gets past a ref that is text", async () => {
    const ws = fixture("35-manifest-schema");
    setRef(ws, '"1.10"');
    const result = await layers(ws, "repo", ["lint"]);
    expect(result.stderr).not.toContain("error [manifest]");
    expect(result.stderr).not.toContain("stack[1].ref");
    expect([0, 1]).toContain(result.code);
  });
});

describe("§13.27 layer text fields (§5.1)", () => {
  referenceDrift(
    "TASK-55.3",
    "alz lint --layer refuses version: 1.10 and names version",
    async () => {
      const ws = fixture("35-manifest-schema");
      writeFileSync(
        `${ws.root}/personal/layer.yaml`,
        "name: personal\nkind: personal\nvisibility: public\nversion: 1.10\n",
      );
      const result = await layers(ws, "repo", ["lint", "--layer", "../personal"]);
      expect(result.code).toBe(1);
      expect(result.stderr).toContain("error [layer]");
      expect(result.stderr).toContain("version");
      expect(result.stderr).not.toContain("1.10");
    },
  );

  it('alz lint --layer accepts version: "1.10"', async () => {
    const ws = fixture("35-manifest-schema");
    writeFileSync(
      `${ws.root}/personal/layer.yaml`,
      'name: personal\nkind: personal\nvisibility: public\nversion: "1.10"\n',
    );
    const result = await layers(ws, "repo", ["lint", "--layer", "../personal"]);
    expect(result.stderr).toBe("");
    expect(result.code).toBe(0);
  });
});
