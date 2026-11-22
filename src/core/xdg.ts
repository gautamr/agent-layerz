import { isAbsolute } from "node:path";
import type { Env } from "./env";

/** Where an XDG base directory is, and whether the variable that could move it was used. */
export interface XdgBase {
  dir: string;
  /** The variable held an absolute path and `dir` is that path. */
  fromVariable: boolean;
  /** The variable held a relative path, which the XDG Base Directory specification says to ignore. */
  relativeIgnored: boolean;
}

/**
 * The XDG Base Directory rule: an unset, empty or relative variable counts as unset, and the
 * default applies. A relative value is reported so a caller can warn about it; an empty one is
 * not (it is how a variable is cleared).
 */
export function xdgBase(env: Env, name: string, home: string, fallback: string): XdgBase {
  const value = env[name];
  if (value === undefined || value === "") {
    return { dir: `${home}/${fallback}`, fromVariable: false, relativeIgnored: false };
  }
  if (!isAbsolute(value)) {
    return { dir: `${home}/${fallback}`, fromVariable: false, relativeIgnored: true };
  }
  return { dir: value, fromVariable: true, relativeIgnored: false };
}
