import { writeFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import { fixture, layers } from "../helpers";

describe("§13.10 leak guard", () => {
  it("a clean public layer passes lint", async () => {
    const ws = fixture("10-leak-guard");
    const result = await layers(ws, ".", ["lint", "--layer", "layer-clean"]);
    expect(result.stderr).toBe("");
    expect(result.code).toBe(0);
  });

  it("a public layer containing a forbidden pattern fails lint", async () => {
    const ws = fixture("10-leak-guard");
    const result = await layers(ws, ".", ["lint", "--layer", "layer-forbidden"]);
    expect(result.code).toBe(1);
    expect(result.stderr).toMatch(/error.*forbid.*skills\/billing\/SKILL\.md/);
  });

  it("a public layer containing a fake token fails lint", async () => {
    const ws = fixture("10-leak-guard");
    // Assembled at run time so the repository never contains a token-shaped string.
    const fakeToken = ["ghp", "_", "a1B2c3D4".repeat(4), "e5F6"].join("");
    writeFileSync(`${ws.root}/layer-token/instructions/10-ci.md`, `- CI token: ${fakeToken}\n`);
    const result = await layers(ws, ".", ["lint", "--layer", "layer-token"]);
    expect(result.code).toBe(1);
    expect(result.stderr).toMatch(/error.*secret.*instructions\/10-ci\.md/);
    expect(result.stderr).not.toContain(fakeToken);
  });
});
