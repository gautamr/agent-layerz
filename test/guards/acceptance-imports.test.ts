import { mkdirSync, readdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { describe, expect, it } from "vitest";
import { REPO, reachableModules } from "../import-graph";
import { tempDir } from "../run-root";

/**
 * The acceptance suite is the language-neutral oracle: it drives the binary and must not reach
 * the TypeScript implementation. This guard finds every acceptance file that does, by any route:
 * an import of any kind (type-only included), a dynamic `import()` or `require` (inside a
 * `vi.mock` factory too), transitively through test helpers, or through `test/import-graph.ts`,
 * the door that reads src/ for `SRC` and `valueImports`.
 *
 * The written list below must equal the violators exactly. It has been empty since R1.7, when the
 * acceptance suite began to spawn the binary under test: a file that reaches src/ again fails.
 *
 * A second guard keeps the acceptance suite out of the shared temp folder: every temp folder is
 * made with `tempDir` from test/run-root.ts, under the run root, which teardown removes.
 */

const ACCEPTANCE = "test/acceptance";
const IMPORT_GRAPH = "test/import-graph.ts";

/** What makes a reached module a reach into the TypeScript implementation. */
const reachesImplementation = (reached: string): boolean =>
  reached.startsWith("src/") || reached === IMPORT_GRAPH || reached.startsWith("<");

const acceptanceFiles = (root: string): string[] =>
  readdirSync(join(root, ACCEPTANCE))
    .filter((name) => name.endsWith(".ts"))
    .sort()
    .map((name) => `${ACCEPTANCE}/${name}`);

const violators = (root: string): string[] =>
  acceptanceFiles(root).filter((file) =>
    [...reachableModules(join(root, file), root)].some(reachesImplementation),
  );

/** The violators missing from the written list, and the listed files that are no longer violators. */
const mismatch = (root: string, written: readonly string[]) => {
  const found = violators(root);
  return {
    unlisted: found.filter((file) => !written.includes(file)),
    stale: written.filter((file) => !found.includes(file)),
  };
};

/** Imports src/ directly, or reaches it through a helper: none since R1.7. */
const WRITTEN: readonly string[] = [];

/** The acceptance files that name `tmpdir`: they must make their folders with `tempDir`. */
const tmpdirUsers = (root: string): string[] =>
  acceptanceFiles(root).filter((file) => /\btmpdir\b/.test(readFileSync(join(root, file), "utf8")));

/** A throwaway repository: `files` maps repository-relative paths to contents. */
const repoWith = (files: Record<string, string>): string => {
  const root = tempDir("alz-guard-");
  for (const [path, text] of Object.entries(files)) {
    mkdirSync(dirname(join(root, path)), { recursive: true });
    writeFileSync(join(root, path), text);
  }
  return root;
};

const CONTROL_FILE = `${ACCEPTANCE}/control.test.ts`;
const SRC_MODULE = { "src/x.ts": "export const x = 1;\n" };

describe("the acceptance suite does not reach src/", () => {
  it("lists exactly the acceptance files that reach src/", () => {
    expect(mismatch(REPO, WRITTEN)).toEqual({ unlisted: [], stale: [] });
  });

  it("keeps every acceptance file out of the shared temp folder", () => {
    expect(tmpdirUsers(REPO)).toEqual([]);
  });

  it("finds an acceptance file that calls tmpdir()", () => {
    const root = repoWith({
      [CONTROL_FILE]: 'import { tmpdir } from "node:os";\nexport const t = tmpdir();\n',
    });
    expect(tmpdirUsers(root)).toEqual([CONTROL_FILE]);
  });

  describe("positive controls: each route into src/ is found", () => {
    const controls: Record<string, Record<string, string>> = {
      "a type-only import": {
        [CONTROL_FILE]: 'import type { X } from "../../src/x";\n',
      },
      "an await import inside a vi.mock factory": {
        [CONTROL_FILE]: [
          'import { vi } from "vitest";',
          'vi.mock("node:fs", async () => (await import("../../src/x")).x);',
          "",
        ].join("\n"),
      },
      "an import through a test helper": {
        [CONTROL_FILE]: 'import { helper } from "../helper";\n',
        "test/helper.ts": 'import { x } from "../src/x";\nexport const helper = x;\n',
      },
      "a join(SRC, ...) entry point": {
        [CONTROL_FILE]: [
          'import { join } from "node:path";',
          'import { SRC } from "../import-graph";',
          'export const entry = join(SRC, "cli/bin.ts");',
          "",
        ].join("\n"),
        [IMPORT_GRAPH]: 'export const SRC = "src";\n',
      },
    };

    for (const [name, files] of Object.entries(controls)) {
      it(`finds ${name}`, () => {
        const root = repoWith({ ...SRC_MODULE, ...files });
        expect(violators(root)).toEqual([CONTROL_FILE]);
        expect(mismatch(root, [])).toEqual({ unlisted: [CONTROL_FILE], stale: [] });
      });
    }

    it("does not flag a file that reaches only packages and test helpers", () => {
      const root = repoWith({
        ...SRC_MODULE,
        [CONTROL_FILE]: 'import { join } from "node:path";\nimport { h } from "../clean";\n',
        "test/clean.ts": "export const h = 1;\n",
      });
      expect(violators(root)).toEqual([]);
    });

    it("fails a file that is fixed but still listed", () => {
      const root = repoWith({ [CONTROL_FILE]: "export const clean = 1;\n" });
      expect(mismatch(root, [CONTROL_FILE])).toEqual({ unlisted: [], stale: [CONTROL_FILE] });
    });
  });

  it("reports repository-relative paths for everything it reaches", () => {
    const root = repoWith({
      ...SRC_MODULE,
      [CONTROL_FILE]: 'import { helper } from "../helper";\n',
      "test/helper.ts": 'import type { x } from "../src/x";\nexport type H = typeof x;\n',
    });
    expect([...reachableModules(join(root, CONTROL_FILE), root)].sort()).toEqual([
      "src/x.ts",
      "test/helper.ts",
    ]);
  });
});
