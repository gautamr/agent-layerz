import { createHash } from "node:crypto";
import { lstatSync, mkdirSync, readFileSync, symlinkSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";
import { capture } from "../exec";
import { tempDir } from "../run-root";

const repoRoot = join(import.meta.dirname, "..", "..");
const script = join(repoRoot, "scripts", "install-local.sh");

/** Runs the script with an isolated HOME, so npm never touches the real home folder. */
function install(binDir: string) {
  const home = tempDir("alz-install-home-");
  return capture("sh", [script], {
    cwd: repoRoot,
    env: { ...process.env, HOME: home, BIN_DIR: binDir, PREFIX: "" },
    timeoutMs: 100_000,
  });
}

function sha256(path: string): string {
  return createHash("sha256").update(readFileSync(path)).digest("hex");
}

describe("installing from source (scripts/install-local.sh)", () => {
  it("installs one regular executable that runs without node_modules", () => {
    const base = tempDir("alz-install-");
    const binDir = join(base, "bin");

    const result = install(binDir);

    expect(result.status, result.stderr).toBe(0);
    const target = join(binDir, "alz");
    const stat = lstatSync(target);
    expect(stat.isFile()).toBe(true);
    expect(stat.isSymbolicLink()).toBe(false);
    expect(stat.mode & 0o777).toBe(0o755);
    expect(result.stdout).toContain(`sha256   ${sha256(target)}`);
    expect(sha256(target)).toBe(sha256(join(repoRoot, "dist", "alz-standalone.mjs")));

    // An empty directory far from the checkout: nothing to resolve yaml or diff from.
    const empty = join(base, "empty");
    mkdirSync(empty);
    const pkg = JSON.parse(readFileSync(join(repoRoot, "package.json"), "utf8"));
    const version = capture(target, ["--version"], { cwd: empty, env: {} });
    expect(version.status).toBe(0);
    expect(version.stderr).toBe("");
    expect(version.stdout).toContain(pkg.version);
    const help = capture(target, ["--help"], { cwd: empty, env: {} });
    expect(help.status).toBe(0);
    expect(help.stdout).not.toBe("");
  }, 120_000);

  it("refuses to replace a symlink at the target", () => {
    const base = tempDir("alz-install-");
    const binDir = join(base, "bin");
    mkdirSync(binDir);
    symlinkSync(join(base, "elsewhere"), join(binDir, "alz"));

    const result = install(binDir);

    expect(result.status).toBe(1);
    expect(result.stderr).toContain("is a symlink");
    expect(lstatSync(join(binDir, "alz")).isSymbolicLink()).toBe(true);
  }, 120_000);
});
