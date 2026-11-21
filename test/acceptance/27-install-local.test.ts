import { spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import { lstatSync, mkdirSync, mkdtempSync, readFileSync, symlinkSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { describe, expect, it } from "vitest";

const repoRoot = join(import.meta.dirname, "..", "..");
const script = join(repoRoot, "scripts", "install-local.sh");

/** Runs the script with an isolated HOME, so npm never touches the real home folder. */
function install(binDir: string) {
  const home = mkdtempSync(join(tmpdir(), "alz-install-home-"));
  return spawnSync("sh", [script], {
    cwd: repoRoot,
    encoding: "utf8",
    env: { ...process.env, HOME: home, BIN_DIR: binDir, PREFIX: "" },
  });
}

function sha256(path: string): string {
  return createHash("sha256").update(readFileSync(path)).digest("hex");
}

describe("installing from source (scripts/install-local.sh)", () => {
  it("installs one regular executable that runs without node_modules", () => {
    const base = mkdtempSync(join(tmpdir(), "alz-install-"));
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
    const version = spawnSync(target, ["--version"], { cwd: empty, encoding: "utf8" });
    expect(version.status, version.stderr).toBe(0);
    expect(version.stdout.trim()).toBe(pkg.version);
    const help = spawnSync(target, ["--help"], { cwd: empty, encoding: "utf8" });
    expect(help.status, help.stderr).toBe(0);
    expect(help.stdout).toContain("usage: alz");
  }, 120_000);

  it("refuses to replace a symlink at the target", () => {
    const base = mkdtempSync(join(tmpdir(), "alz-install-"));
    const binDir = join(base, "bin");
    mkdirSync(binDir);
    symlinkSync(join(base, "elsewhere"), join(binDir, "alz"));

    const result = install(binDir);

    expect(result.status).toBe(1);
    expect(result.stderr).toContain("is a symlink");
    expect(lstatSync(join(binDir, "alz")).isSymbolicLink()).toBe(true);
  }, 120_000);
});
