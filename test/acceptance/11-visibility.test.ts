import { describe, expect, it } from "vitest";
import { fixture, layers } from "../helpers";

describe("§13.11 visibility", () => {
  it("a public layer extending a private one fails", async () => {
    const ws = fixture("11-visibility");
    const result = await layers(ws, "repo", ["build"]);
    expect(result.code).toBe(1);
    expect(result.stderr).toMatch(/error.*public layer 'personal'.*private layer 'acme'/);
  });
});
