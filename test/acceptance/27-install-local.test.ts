import { mkdirSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";
import { holdsVersion, layers, packageVersion, type Workspace } from "../helpers";
import { tempDir } from "../run-root";

// The install script itself (npm, esbuild on src/, dist/) is a TypeScript-reference concern:
// test/reference/27-install-local.test.ts. What stays here is what any one-file `alz` must do.

/** An empty directory far from the checkout, with its own HOME: nothing to resolve yaml or diff from. */
function emptyWorkspace(): Workspace {
  const base = tempDir("alz-install-");
  const root = join(base, "empty");
  const home = join(base, "home");
  mkdirSync(root);
  mkdirSync(home);
  return { root, home };
}

describe("the one-file binary runs on its own", () => {
  it("--version prints the package version, to stdout only", async () => {
    const result = await layers(emptyWorkspace(), ".", ["--version"]);
    expect(result.code).toBe(0);
    expect(result.stderr).toBe("");
    expect(holdsVersion(result.stdout, packageVersion())).toBe(true);
  });

  it("--help prints usage to stdout and exits 0", async () => {
    const result = await layers(emptyWorkspace(), ".", ["--help"]);
    expect(result.code).toBe(0);
    expect(result.stdout).not.toBe("");
  });
});
