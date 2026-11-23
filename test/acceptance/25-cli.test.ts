import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import { fixture, layers } from "../helpers";

describe("alz CLI", () => {
  it("--version prints the package version", async () => {
    const { version } = JSON.parse(
      readFileSync(`${import.meta.dirname}/../../package.json`, "utf8"),
    );
    const result = await layers(fixture("14-non-destructive"), "repo", ["--version"]);
    expect(result.code).toBe(0);
    expect(result.stderr).toBe("");
    expect(result.stdout).toContain(version);
  });
});
