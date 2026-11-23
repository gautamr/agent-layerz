import { describe, expect, it } from "vitest";
import { fixture, holdsVersion, layers, packageVersion } from "../helpers";

describe("alz CLI", () => {
  it("--version prints the package version", async () => {
    const result = await layers(fixture("14-non-destructive"), "repo", ["--version"]);
    expect(result.code).toBe(0);
    expect(result.stderr).toBe("");
    expect(holdsVersion(result.stdout, packageVersion())).toBe(true);
  });
});
