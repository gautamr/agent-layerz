import { existsSync, readFileSync } from "node:fs";
import { join } from "node:path";

export const SRC = join(import.meta.dirname, "../src");

/** What a source text imports as a value: static, side-effect, dynamic and `require` forms. */
export function importedSpecifiers(text: string): string[] {
  const found: string[] = [];
  // `import x from "y"`, `export { x } from "y"`; a type-only import is erased and does not count.
  const named =
    /^(?:import|export)\s+(type\s+)?(?:\{[^}]*\}|\*\s+as\s+\w+|\w+)(?:\s*,\s*(?:\{[^}]*\}|\*\s+as\s+\w+))?\s+from\s+["']([^"']+)["']/gm;
  for (const [, typeOnly, specifier] of text.matchAll(named)) {
    if (!typeOnly && specifier) found.push(specifier);
  }
  // `import "y"`, `import("y")`, `require("y")`.
  const others = [
    /^import\s+["']([^"']+)["']/gm,
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
