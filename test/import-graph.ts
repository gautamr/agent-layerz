import { existsSync, readFileSync, statSync } from "node:fs";
import { dirname, isAbsolute, join, relative, resolve } from "node:path";

export const SRC = join(import.meta.dirname, "../src");

/** What a source text imports as a value: static, side-effect, dynamic and `require` forms. */
export function importedSpecifiers(text: string): string[] {
  const found: string[] = [];
  // `import x from "y"`, `export { x } from "y"`; a type-only import is erased and does not count.
  const named =
    /^[ \t]*(?:import|export)\s+(type\s+)?(?:\{[^}]*\}|\*\s+as\s+\w+|\w+)(?:\s*,\s*(?:\{[^}]*\}|\*\s+as\s+\w+))?\s+from\s+["']([^"']+)["']/gm;
  for (const [, typeOnly, specifier] of text.matchAll(named)) {
    if (!typeOnly && specifier) found.push(specifier);
  }
  // `import "y"`, `import("y")`, `require("y")`.
  const others = [
    /^[ \t]*import\s+["']([^"']+)["']/gm,
    /\bimport\(\s*["'`]([^"'`]+)["'`]\s*\)/g,
    /\brequire\(\s*["'`]([^"'`]+)["'`]\s*\)/g,
  ];
  for (const pattern of others) {
    for (const [, specifier] of text.matchAll(pattern)) if (specifier) found.push(specifier);
  }
  // A specifier that is not a literal cannot be followed: it counts as an import of its own kind.
  if (/\bimport\(\s*(?!["'`])/.test(text)) found.push("<dynamic import>");
  if (/\brequire\(\s*(?!["'`])/.test(text)) found.push("<dynamic require>");
  return found;
}

/** The modules `entry` imports as values, transitively: type-only imports are erased and do not count. */
export function valueImports(entry: string): Set<string> {
  const seen = new Set<string>();
  const found = new Set<string>();
  const visit = (file: string) => {
    if (seen.has(file)) return;
    seen.add(file);
    for (const specifier of importedSpecifiers(readFileSync(file, "utf8"))) {
      if (!specifier.startsWith(".")) {
        found.add(specifier);
        continue;
      }
      const base = join(file, "..", specifier);
      const target = existsSync(`${base}.ts`) ? `${base}.ts` : join(base, "index.ts");
      found.add(target.slice(SRC.length + 1));
      visit(target);
    }
  };
  visit(join(SRC, entry));
  return found;
}

/** Repository root: the directory above `test/` and `src/`. */
export const REPO = join(import.meta.dirname, "..");

/**
 * Every specifier a source text refers to in any form, type-only imports included: `import` and
 * `export ... from`, `export * from`, side-effect imports, `import()` (inside `vi.mock` factories
 * too), `require`, and the `vi.mock`, `vi.doMock`, `vi.importActual` and `vi.importMock` literals.
 */
export function referencedSpecifiers(text: string): string[] {
  const typeOnly =
    /^[ \t]*(?:import|export)\s+type\s+(?:\{[^}]*\}|\*\s+as\s+\w+|\w+)(?:\s*,\s*(?:\{[^}]*\}|\*\s+as\s+\w+))?\s+from\s+["']([^"']+)["']/gm;
  const exportStar = /^[ \t]*export\s+\*\s+from\s+["']([^"']+)["']/gm;
  const mocks = /\bvi\.(?:mock|doMock|importActual|importMock)\(\s*["'`]([^"'`]+)["'`]/g;
  const extra = [typeOnly, exportStar, mocks].flatMap((pattern) =>
    [...text.matchAll(pattern)].flatMap(([, specifier]) => (specifier ? [specifier] : [])),
  );
  return [...importedSpecifiers(text), ...extra];
}

/** The file a relative or absolute specifier names from `file`: as written, `.ts`, or a directory `index.ts`. */
function resolveRelative(file: string, specifier: string): string {
  const base = resolve(dirname(file), specifier);
  const isFile = (path: string) => existsSync(path) && statSync(path).isFile();
  return [base, `${base}.ts`, join(base, "index.ts")].find(isFile) ?? `${base}.ts`;
}

/**
 * Every module `entry` reaches through any import form, transitively (see `referencedSpecifiers`).
 * Paths are relative to `root`; a package (a specifier that does not start with `.`) is reported
 * as written, and so is a specifier that cannot be followed (`<dynamic import>`). `entry` itself
 * is not reported. Only `.ts` files are read.
 */
export function reachableModules(entry: string, root: string = REPO): Set<string> {
  const seen = new Set<string>();
  const found = new Set<string>();
  const visit = (file: string) => {
    if (seen.has(file)) return;
    seen.add(file);
    if (!file.endsWith(".ts") || !existsSync(file)) return;
    for (const specifier of referencedSpecifiers(readFileSync(file, "utf8"))) {
      // An absolute specifier is a path: follow it like a relative one, so one that names a file
      // under src/ is reported as `src/...`. Only a package or a `<dynamic ...>` stays as written.
      if (!specifier.startsWith(".") && !isAbsolute(specifier)) {
        found.add(specifier);
        continue;
      }
      const target = resolveRelative(file, specifier);
      if (target !== entry) found.add(relative(root, target));
      visit(target);
    }
  };
  visit(entry);
  return found;
}
