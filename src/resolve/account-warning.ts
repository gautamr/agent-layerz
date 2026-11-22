/**
 * A problem found while resolving an account. `code` is stable (printed as `account-<code>`);
 * `text` is already safe to print: fixed phrases, paths through `displayPath`, repository text
 * through `quote`, and nothing else.
 */
export interface AccountWarning {
  code: string;
  text: string;
}

/** Collects warnings in the order they are found, each text once. */
export class WarningList {
  private readonly seen = new Set<string>();
  readonly items: AccountWarning[] = [];

  add(code: string, text: string): void {
    const key = `${code}\u0000${text}`;
    if (this.seen.has(key)) return;
    this.seen.add(key);
    this.items.push({ code, text });
  }
}
