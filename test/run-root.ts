import { mkdtempSync } from "node:fs";
import { join } from "node:path";
import { inject } from "vitest";

/** The folder every temp folder and every child's TMPDIR of this run lies in (made by test/setup). */
export const tmpRoot = (): string => join(inject("runRoot"), "tmp");

/** A fresh folder under the run root, removed with it. Tests never call `tmpdir()` themselves. */
export const tempDir = (prefix: string): string => mkdtempSync(join(tmpRoot(), prefix));

/** The binary the acceptance suite spawns: ALZ_BIN, or the TypeScript reference built for this run. */
export const alzBin = (): string => inject("alzBin");
