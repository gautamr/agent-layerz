import { readFileSync } from "node:fs";
import YAML from "yaml";
import { FatalError, InputError } from "./diagnostics";

export function parseYaml(text: string): unknown {
  try {
    return YAML.parse(text);
  } catch (error) {
    throw new InputError(`invalid YAML: ${(error as Error).message.split("\n")[0]}`);
  }
}

export function readYamlFile(path: string, where: string): unknown {
  try {
    return parseYaml(readFileSync(path, "utf8"));
  } catch (error) {
    throw new FatalError("yaml", (error as Error).message, where);
  }
}

/** Block-style YAML with no line folding, so output is stable and diff-friendly. */
export function stringifyYaml(value: unknown): string {
  return YAML.stringify(value, { lineWidth: 0, minContentWidth: 0 });
}

export type YamlMap = Record<string, unknown>;

export function isMap(value: unknown): value is YamlMap {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

/** Returns the keys of `map` that are not in `allowed`. */
export function unknownKeys(map: YamlMap, allowed: readonly string[]): string[] {
  return Object.keys(map).filter((key) => !allowed.includes(key));
}

export function asStringList(value: unknown, field: string): string[] {
  if (!Array.isArray(value) || value.some((item) => typeof item !== "string")) {
    throw new InputError(`'${field}' must be a list of strings`);
  }
  return value as string[];
}

export function asOptionalString(value: unknown, field: string): string | undefined {
  if (value === undefined || value === null) return undefined;
  if (typeof value !== "string") throw new InputError(`'${field}' must be a string`);
  return value;
}
