import { execFileSync } from "node:child_process";
import type { Env } from "../resolve/git";

/** Runs `<bin> --version` and extracts x.y.z; undefined when the tool is not installed. */
export function installedVersion(bin: string, env: Env): string | undefined {
  try {
    const out = execFileSync(bin, ["--version"], {
      env,
      encoding: "utf8",
      stdio: ["ignore", "pipe", "ignore"],
      timeout: 10_000,
    });
    return /(\d+\.\d+\.\d+)/.exec(out)?.[1];
  } catch {
    return undefined;
  }
}

export function compareVersions(a: string, b: string): number {
  const pa = a.split(".").map(Number);
  const pb = b.split(".").map(Number);
  for (let i = 0; i < 3; i++) {
    const diff = (pa[i] ?? 0) - (pb[i] ?? 0);
    if (diff !== 0) return Math.sign(diff);
  }
  return 0;
}

export function inRange(version: string, range: { min: string; below: string }): boolean {
  return compareVersions(version, range.min) >= 0 && compareVersions(version, range.below) < 0;
}
