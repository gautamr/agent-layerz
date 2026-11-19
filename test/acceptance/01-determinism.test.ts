import { describe, expect, it } from "vitest";
import { fixture, layers, readTree } from "../helpers";

describe("§13.1 determinism", () => {
  it("building twice gives byte-identical output and check passes", async () => {
    const ws = fixture("01-determinism");
    const first = await layers(ws, "repo", ["build"]);
    expect(first.stderr).not.toMatch(/^error/m);
    expect(first.code).toBe(0);
    const afterFirst = readTree(`${ws.root}/repo`);

    const second = await layers(ws, "repo", ["build"]);
    expect(second.code).toBe(0);
    expect(readTree(`${ws.root}/repo`)).toEqual(afterFirst);

    const check = await layers(ws, "repo", ["check"]);
    expect(check.stderr).not.toMatch(/^error/m);
    expect(check.code).toBe(0);
  });

  it("output does not depend on where the workspace lives", async () => {
    const a = fixture("01-determinism");
    const b = fixture("01-determinism");
    expect((await layers(a, "repo", ["build"])).code).toBe(0);
    expect((await layers(b, "repo", ["build"])).code).toBe(0);
    expect(readTree(`${b.root}/repo`)).toEqual(readTree(`${a.root}/repo`));
  });

  it("check fails after a generated file is edited by hand", async () => {
    const ws = fixture("01-determinism");
    expect((await layers(ws, "repo", ["build"])).code).toBe(0);
    const { appendFileSync } = await import("node:fs");
    appendFileSync(`${ws.root}/repo/AGENTS.md`, "\nhand edit\n");
    const check = await layers(ws, "repo", ["check"]);
    expect(check.code).toBe(1);
    expect(check.stdout + check.stderr).toContain("AGENTS.md");
  });
});
