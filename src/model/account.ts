import { InputError } from "../core/diagnostics";
import { isMap } from "../core/yaml";

/** §7.13: the variable each account target is chosen by. */
export const ACCOUNT_TARGETS = {
  claude: "CLAUDE_CONFIG_DIR",
  codex: "CODEX_HOME",
} as const;

export type AccountTarget = keyof typeof ACCOUNT_TARGETS;

const TARGET_NAMES = Object.keys(ACCOUNT_TARGETS) as AccountTarget[];

/** One account name per target; a target left out is not decided by the layer. */
export type AccountMap = Partial<Record<AccountTarget, string>>;

export const ACCOUNT_NAME = /^[a-z0-9][a-z0-9-]*$/;
export const ACCOUNT_NAME_MAX = 64;

/** §7.13 'Account names': the name becomes part of a folder path, so nothing else passes. */
export function isAccountName(text: string): boolean {
  return text.length <= ACCOUNT_NAME_MAX && ACCOUNT_NAME.test(text);
}

const RULE = `a name of lower-case letters, digits and '-', starting with a letter or digit, at most ${ACCOUNT_NAME_MAX} characters`;

/**
 * §7.13 validation of the `account` field. Layer content, so it is checked at parse time.
 * Messages name the field and the rule and never repeat the value, which may be a path or a secret.
 */
export function parseAccount(raw: unknown): AccountMap {
  if (typeof raw === "string") {
    if (!isAccountName(raw)) throw new InputError(`'account' must be ${RULE}`);
    return Object.fromEntries(TARGET_NAMES.map((target) => [target, raw])) as AccountMap;
  }
  if (!isMap(raw)) {
    throw new InputError("'account' must be an account name or a mapping of target to name");
  }
  const keys = Object.keys(raw);
  if (keys.length === 0) throw new InputError("'account' must not be an empty mapping");
  if (keys.some((key) => !TARGET_NAMES.includes(key as AccountTarget))) {
    throw new InputError(`'account' keys must be target names (${TARGET_NAMES.join(", ")})`);
  }
  const map: AccountMap = {};
  for (const target of TARGET_NAMES) {
    if (!Object.hasOwn(raw, target)) continue;
    const name = raw[target];
    if (typeof name !== "string" || !isAccountName(name)) {
      throw new InputError(`'account.${target}' must be ${RULE}`);
    }
    map[target] = name;
  }
  return map;
}

/**
 * §7.9: variables no layer may set. They choose the account folder or locate alz's own files
 * and the layer cache.
 */
export const PROTECTED_VARIABLES = [
  "CLAUDE_CONFIG_DIR",
  "CODEX_HOME",
  "HOME",
  "XDG_CONFIG_HOME",
  "XDG_STATE_HOME",
  "XDG_CACHE_HOME",
] as const;

const PROTECTED_UPPER: readonly string[] = PROTECTED_VARIABLES;

/** ASCII case folding only, as a name set on Windows would compare. */
export function asciiUpper(text: string): string {
  return text.replace(/[a-z]/g, (char) => char.toUpperCase());
}

/** ASCII case folding only: a file system may fold more, so non-ASCII path text is refused instead. */
export function asciiLower(text: string): string {
  return text.replace(/[A-Z]/g, (char) => char.toLowerCase());
}

export function isProtectedName(name: string): boolean {
  return PROTECTED_UPPER.includes(asciiUpper(name));
}

/** The protected variables the Codex pattern matches. */
export function matchedProtectedNames(pattern: string): string[] {
  return PROTECTED_VARIABLES.filter((name) => codexPatternMatches(pattern, name));
}

/**
 * Mirrors wildmatch 2.6.1 as Codex builds its policy patterns (target-facts §12 item 15):
 * `*` matches any run of characters, `?` one character, no classes, and two characters are
 * equal when they are equal or their Unicode lower-case forms are.
 */
export function codexPatternMatches(pattern: string, name: string): boolean {
  const p = [...pattern];
  const n = [...name];
  // reach[j]: the pattern prefix read so far matches name[0..j).
  let reach: boolean[] = Array.from({ length: n.length + 1 }, (_, j) => j === 0);
  for (const token of p) {
    const next: boolean[] = new Array<boolean>(n.length + 1).fill(false);
    if (token === "*") {
      let seen = false;
      for (let j = 0; j <= n.length; j++) {
        seen = seen || reach[j] === true;
        next[j] = seen;
      }
    } else {
      for (let j = 1; j <= n.length; j++) {
        const char = n[j - 1] ?? "";
        const same = token === "?" || token === char || token.toLowerCase() === char.toLowerCase();
        next[j] = reach[j - 1] === true && same;
      }
    }
    reach = next;
  }
  return reach[n.length] === true;
}
