import { existsSync, mkdirSync, writeFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import { fixture, layers, readFile } from "../helpers";

// A cloned repo is untrusted input: its manifest and lock must not reach outside it.
describe("untrusted repo content", () => {
  it("refuses a lock that lists a path outside the repo, and deletes nothing", async () => {
    const ws = fixture("14-non-destructive");
    expect((await layers(ws, "repo", ["build", "--adopt"])).code).toBe(0);
    mkdirSync(`${ws.root}/victim`);
    writeFileSync(`${ws.root}/victim/important.txt`, "keep me");
    const lock = JSON.parse(readFile(ws, "repo/layer.lock"));
    lock.files["../victim/important.txt"] = "sha256:00";
    writeFileSync(`${ws.root}/repo/layer.lock`, JSON.stringify(lock));

    const result = await layers(ws, "repo", ["build"]);
    expect(result.code).toBe(1);
    expect(result.stderr).toContain("unsafe entry '../victim/important.txt'");
    expect(existsSync(`${ws.root}/victim/important.txt`)).toBe(true);
  });

  it("refuses a lock whose layer sha is not a commit id", async () => {
    const ws = fixture("14-non-destructive");
    expect((await layers(ws, "repo", ["build", "--adopt"])).code).toBe(0);
    const lock = JSON.parse(readFile(ws, "repo/layer.lock"));
    lock.layers[0].sha = "../../../etc";
    writeFileSync(`${ws.root}/repo/layer.lock`, JSON.stringify(lock));
    const result = await layers(ws, "repo", ["build"]);
    expect(result.code).toBe(1);
    expect(result.stderr).toContain("unsafe entry '../../../etc'");
  });

  it.each([
    ["git:--upload-pack=touch PWNED@v1", "cannot start with '-'"],
    ["github:owner/repo@--upload-pack=touch", "is not a valid git ref"],
  ])("never passes %s to git as an option", async (source, message) => {
    const ws = fixture("14-non-destructive");
    writeFileSync(
      `${ws.root}/repo/layer.yaml`,
      `stack:\n  - source: ${source}\n  - source: path:./.layer\n`,
    );
    const result = await layers(ws, "repo", ["build"]);
    expect(result.code).toBe(1);
    expect(result.stderr).toContain(message);
    expect(existsSync(`${ws.root}/repo/PWNED`)).toBe(false);
  });

  it("rejects a manifest ref that git would read as an option", async () => {
    const ws = fixture("14-non-destructive");
    writeFileSync(
      `${ws.root}/repo/layer.yaml`,
      "stack:\n  - source: path:../personal\n    ref: --output=/tmp/x\n  - source: path:./.layer\n",
    );
    const result = await layers(ws, "repo", ["build"]);
    expect(result.code).toBe(1);
    expect(result.stderr).toContain("is not a valid git ref");
  });
});
