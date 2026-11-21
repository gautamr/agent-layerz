import { InputError } from "../core/diagnostics";
import { isControl, isFormat } from "../core/sanitize";
import { isMap, unknownKeys } from "../core/yaml";

/** §7.12: the identity every commit is authored and committed under. */
export interface CommitIdentity {
  name: string;
  email: string;
}

const NAME_MAX = 128;
const EMAIL_MAX = 254;
/** Characters git trims from both ends of a name (`strbuf_addstr_without_crud`) or that shells read specially. */
const EDGE = /^[.,:;"'\\<>]|[.,:;"'\\<>]$/;
const EMAIL_PATTERN = /^[A-Za-z0-9._%+-]+@[A-Za-z0-9-]+(\.[A-Za-z0-9-]+)+$/;

/**
 * §7.12 validation. The identity is layer content that reaches a git subprocess, so this runs
 * at parse time. Messages never repeat the offending value.
 */
export function parseCommitIdentity(raw: unknown): CommitIdentity {
  if (!isMap(raw)) throw new InputError("'commit_identity' must be a mapping with name and email");
  const extra = unknownKeys(raw, ["name", "email"]);
  if (extra.length > 0 || !("name" in raw) || !("email" in raw)) {
    throw new InputError("'commit_identity' must have exactly the keys name and email");
  }
  const { name, email } = raw;
  if (typeof name !== "string" || typeof email !== "string") {
    throw new InputError("'commit_identity' name and email must be strings");
  }
  const chars = [...name];
  if (chars.length < 1 || chars.length > NAME_MAX) {
    throw new InputError(`'commit_identity.name' must be 1 to ${NAME_MAX} characters`);
  }
  if (name !== name.trim()) {
    throw new InputError("'commit_identity.name' must not start or end with whitespace");
  }
  if (EDGE.test(name)) {
    throw new InputError(
      "'commit_identity.name' must not start or end with . , : ; \" ' \\ < or > (git trims them)",
    );
  }
  if (name.startsWith("-")) {
    throw new InputError("'commit_identity.name' must not start with '-'");
  }
  if (chars.some((char) => isControl(char.codePointAt(0) ?? 0))) {
    throw new InputError("'commit_identity.name' must not contain control characters");
  }
  if (chars.some(isFormat)) {
    throw new InputError("'commit_identity.name' must not contain invisible format characters");
  }
  if (/[<>`\\]/.test(name)) {
    throw new InputError(
      "'commit_identity.name' must not contain '<', '>', a backtick or a backslash",
    );
  }
  if (email.length > EMAIL_MAX) {
    throw new InputError(`'commit_identity.email' must be at most ${EMAIL_MAX} characters`);
  }
  if (email.startsWith("-")) {
    throw new InputError("'commit_identity.email' must not start with '-'");
  }
  if (email.startsWith(".") || email.endsWith(".")) {
    throw new InputError("'commit_identity.email' must not start or end with '.'");
  }
  if ([...email].some(isFormat)) {
    throw new InputError("'commit_identity.email' must not contain invisible format characters");
  }
  if (!EMAIL_PATTERN.test(email)) {
    throw new InputError("'commit_identity.email' is not a plain ASCII email address");
  }
  return { name, email };
}
