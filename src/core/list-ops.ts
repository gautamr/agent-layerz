import { InputError } from "./diagnostics";

export type ListOp =
  | { kind: "replace"; items: string[] }
  | { kind: "add"; items: string[] }
  | { kind: "remove"; items: string[] };

const SEGMENT = /\s*([+-]?)\[([^\]]*)\]\s*/y;

/**
 * Parses an overlay list value: a YAML list (replace), or a string of segments
 * `+[a, b]` (append), `-[c]` (remove) and `[d]` (replace). `null`/`undefined` mean "keep".
 */
export function parseListOps(value: unknown, field: string): ListOp[] | undefined {
  if (value === undefined || value === null) return undefined;
  if (Array.isArray(value)) {
    if (value.some((item) => typeof item !== "string")) {
      throw new InputError(`'${field}' must be a list of strings`);
    }
    return [{ kind: "replace", items: value as string[] }];
  }
  if (typeof value !== "string") {
    throw new InputError(`'${field}' must be a list or a "+[…] -[…]" string`);
  }
  const ops: ListOp[] = [];
  SEGMENT.lastIndex = 0;
  while (SEGMENT.lastIndex < value.length) {
    const start = SEGMENT.lastIndex;
    const match = SEGMENT.exec(value);
    if (!match) {
      throw new InputError(
        `'${field}': cannot parse '${value.slice(start)}'; expected +[…], -[…] or […]`,
      );
    }
    const items = splitItems(match[2] ?? "");
    const sign = match[1];
    ops.push({ kind: sign === "+" ? "add" : sign === "-" ? "remove" : "replace", items });
  }
  return ops;
}

function splitItems(text: string): string[] {
  return text
    .split(",")
    .map((item) => item.trim().replace(/^(["'])(.*)\1$/, "$2"))
    .filter((item) => item.length > 0);
}

/** Applies ops in order. The result is de-duplicated and keeps first-seen order. */
export function applyListOps(base: readonly string[], ops: readonly ListOp[]): string[] {
  let result = [...base];
  for (const op of ops) {
    if (op.kind === "replace") result = [...op.items];
    else if (op.kind === "add") result = [...result, ...op.items];
    else result = result.filter((item) => !op.items.includes(item));
  }
  return [...new Set(result)];
}

export function describeOps(ops: readonly ListOp[]): string {
  return ops
    .map(
      (op) =>
        `${op.kind === "add" ? "+" : op.kind === "remove" ? "-" : ""}[${op.items.join(", ")}]`,
    )
    .join(" ");
}
