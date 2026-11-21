import { stripControl } from "./sanitize";

export type Level = "error" | "warning";

export interface Diagnostic {
  level: Level;
  /** Short kebab-case code, e.g. `tool-widen`. */
  code: string;
  message: string;
  /** `<layer>:<path>` or a repo-relative path. */
  where?: string;
}

/** Collects errors and warnings so one run reports every problem it can find. */
export class Diagnostics {
  readonly items: Diagnostic[] = [];

  error(code: string, message: string, where?: string): void {
    this.items.push({ level: "error", code, message, where });
  }

  warn(code: string, message: string, where?: string): void {
    this.items.push({ level: "warning", code, message, where });
  }

  get hasErrors(): boolean {
    return this.items.some((item) => item.level === "error");
  }

  get errors(): Diagnostic[] {
    return this.items.filter((item) => item.level === "error");
  }

  get warnings(): Diagnostic[] {
    return this.items.filter((item) => item.level === "warning");
  }
}

/** A problem that stops the run, such as a missing manifest. */
export class FatalError extends Error {
  constructor(
    readonly code: string,
    message: string,
    readonly where?: string,
  ) {
    super(message);
  }
}

export function formatDiagnostic(item: Diagnostic): string {
  const where = item.where ? `${stripControl(item.where)}: ` : "";
  // Messages can carry text from layer files and manifests, so they never reach a terminal raw.
  return `${item.level} [${item.code}] ${where}${stripControl(item.message)}`;
}

/** Code for references to a more general layer; `lint` of a single layer skips these. */
export const UNRESOLVED = "unresolved";

/** Error raised by a small parser; the caller adds the location. */
export class InputError extends Error {}
