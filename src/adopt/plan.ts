import { existsSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { isMap, parseYaml, type YamlMap } from "../core/yaml";
import type { ResolvedConfig } from "../model/config";

/**
 * §4.3 `--adopt`: everything adoption will do, computed before anything is written. If
 * `problems` is not empty, nothing is applied, so a partial adoption never happens.
 */
export class AdoptionPlan {
  /** Repo-layer path → final text (YAML files already merged with what is on disk). */
  readonly layerFiles = new Map<string, string | { yaml: unknown }>();
  /** Native files moved byte-for-byte: repo path → repo-layer path. */
  readonly moves: { from: string; to: string }[] = [];
  /** Converted native files, deleted once their layer files are written. */
  readonly removes: string[] = [];
  readonly manifest = { addDirs: [] as string[], mcpEnable: [] as string[] };
  /** What each native file became, for the report. */
  readonly report: { from: string; to: string[] }[] = [];
  readonly notes: string[] = [];
  readonly problems: string[] = [];

  constructor(
    readonly repoRoot: string,
    /** The repo layer, relative to the repo root (e.g. `.layer`). */
    readonly layerDir: string,
    readonly layerName: string,
    /** The build before adoption: models and agents of the more general layers. */
    readonly config: ResolvedConfig,
  ) {}

  /** Queues a text file in the repo layer; it must not exist yet. */
  writeText(from: string, path: string, text: string): void {
    if (this.layerFiles.has(path) || existsSync(join(this.repoRoot, this.layerDir, path))) {
      this.problems.push(`${from}: ${this.layerDir}/${path} already exists`);
      return;
    }
    this.layerFiles.set(path, text);
  }

  /** Queues YAML for a repo-layer file, merged into what is already there (maps deep, lists appended). */
  mergeYaml(from: string, path: string, value: unknown): void {
    const queued = this.layerFiles.get(path);
    let current: unknown;
    if (queued !== undefined) {
      if (typeof queued === "string") {
        this.problems.push(`${from}: ${path} is written twice`);
        return;
      }
      current = queued.yaml;
    } else {
      const abs = join(this.repoRoot, this.layerDir, path);
      try {
        current = existsSync(abs) ? parseYaml(readFileSync(abs, "utf8")) : undefined;
      } catch (error) {
        this.problems.push(
          `${from}: cannot merge into ${this.layerDir}/${path}: ${(error as Error).message}`,
        );
        return;
      }
    }
    const merged = mergeValues(current, value);
    if (merged === CONFLICT) {
      this.problems.push(
        `${from}: ${this.layerDir}/${path} has a different shape; merge it by hand`,
      );
      return;
    }
    this.layerFiles.set(path, { yaml: merged });
  }

  move(from: string, to: string): void {
    if (existsSync(join(this.repoRoot, this.layerDir, to))) {
      this.problems.push(`${from}: ${this.layerDir}/${to} already exists`);
      return;
    }
    this.moves.push({ from, to });
    this.report.push({ from, to: [`${this.layerDir}/${to}`] });
  }

  converted(from: string, to: string[]): void {
    this.removes.push(from);
    this.report.push({
      from,
      to: to.map((path) => (path === "layer.yaml" ? path : `${this.layerDir}/${path}`)),
    });
  }
}

const CONFLICT = Symbol("conflict");

function mergeValues(current: unknown, next: unknown): unknown {
  if (current === undefined || current === null) return next;
  if (Array.isArray(current) && Array.isArray(next)) {
    const seen = new Set(current.map((item) => JSON.stringify(item)));
    return [...current, ...next.filter((item) => !seen.has(JSON.stringify(item)))];
  }
  if (isMap(current) && isMap(next)) {
    const result: YamlMap = { ...current };
    for (const [key, value] of Object.entries(next)) {
      const merged = mergeValues(result[key], value);
      if (merged === CONFLICT) return CONFLICT;
      result[key] = merged;
    }
    return result;
  }
  return JSON.stringify(current) === JSON.stringify(next) ? current : CONFLICT;
}
