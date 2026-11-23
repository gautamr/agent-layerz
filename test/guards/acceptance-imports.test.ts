import { mkdirSync, mkdtempSync, readdirSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { describe, expect, it } from "vitest";
import { REPO, reachableModules } from "../import-graph";

/**
 * The acceptance suite is the language-neutral oracle: it drives the binary and must not reach
 * the TypeScript implementation. This guard finds every acceptance file that does, by any route:
 * an import of any kind (type-only included), a dynamic `import()` or `require` (inside a
 * `vi.mock` factory too), transitively through test helpers, or through `test/import-graph.ts`,
 * the door that reads src/ for `SRC` and `valueImports`.
 *
 * The written list below must equal the violators exactly. Converting a file removes it from
 * the list in the same change; a file that is fixed but still listed fails as well. R1.7 empties it.
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

/** Imports src/ directly: none since R1.6. */
const DIRECT: readonly string[] = [];

/** Reach src/ only through test/helpers.ts (which runs `run` from src/cli/main in process). */
const THROUGH_HELPERS = [
  "test/acceptance/01-determinism.test.ts",
  "test/acceptance/02-instructions.test.ts",
  "test/acceptance/03-agent-overlay.test.ts",
  "test/acceptance/04-tool-narrowing.test.ts",
  "test/acceptance/05-skill-override.test.ts",
  "test/acceptance/06-slots.test.ts",
  "test/acceptance/07-permission-floor.test.ts",
  "test/acceptance/08-locked-hook.test.ts",
  "test/acceptance/09-mcp-allowlist.test.ts",
  "test/acceptance/10-leak-guard.test.ts",
  "test/acceptance/11-visibility.test.ts",
  "test/acceptance/12-account-independence.test.ts",
  "test/acceptance/14-non-destructive.test.ts",
  "test/acceptance/15-protected-paths.test.ts",
  "test/acceptance/16-hub.test.ts",
  "test/acceptance/17-rules.test.ts",
  "test/acceptance/18-native-fields.test.ts",
  "test/acceptance/19-hook-commands.test.ts",
  "test/acceptance/20-adopt-native.test.ts",
  "test/acceptance/21-init-stack.test.ts",
  "test/acceptance/22-ls.test.ts",
  "test/acceptance/23-rename-compat.test.ts",
  "test/acceptance/24-untrusted-repo.test.ts",
  "test/acceptance/25-cli.test.ts",
  "test/acceptance/26-commit-identity.test.ts",
  "test/acceptance/28-cache-integrity.test.ts",
  "test/acceptance/29-account-layer.test.ts",
  "test/acceptance/30-account-cache-read.test.ts",
  "test/acceptance/31-account-which.test.ts",
  "test/acceptance/32-account-allow.test.ts",
];

const WRITTEN: readonly string[] = [...DIRECT, ...THROUGH_HELPERS].sort();

/** A throwaway repository: `files` maps repository-relative paths to contents. */
const repoWith = (files: Record<string, string>): string => {
  const root = mkdtempSync(join(tmpdir(), "alz-guard-"));
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
