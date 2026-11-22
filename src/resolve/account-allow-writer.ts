import { randomBytes } from "node:crypto";
import {
  closeSync,
  constants,
  fchmodSync,
  fstatSync,
  fsyncSync,
  lstatSync,
  mkdirSync,
  openSync,
  realpathSync,
  renameSync,
  unlinkSync,
  writeSync,
} from "node:fs";
import { basename, dirname, isAbsolute, join, relative } from "node:path";
import { isDeepStrictEqual } from "node:util";
import type { Env } from "../core/env";
import { insideRoot, isWithin, readRegularFileWithId } from "../core/files";
import { displayPath } from "../core/sanitize";
import { parseYamlLimited } from "../core/yaml";
import {
  ALLOW_HEADER,
  type AllowedEntry,
  entryFor,
  loadAllowState,
  plainEntries,
  sameEntry,
  serializeAllowState,
  stateProblem,
} from "./account-allow-state";
import {
  type AccountFileContext,
  locateAccountFile,
  placementProblem,
  unplacedReason,
} from "./account-files";
import { realOrResolved } from "./account-folders";
import { WarningList } from "./account-warning";

/**
 * The only code that writes outside the repository and the layer cache (decision-20): it
 * writes `allowed.yaml`, its lock file and its temporary file, in one directory that comes from
 * `$HOME`, `XDG_STATE_HOME` and fixed names. No repository text becomes part of a path. It
 * refuses every state the reader of `allowed.yaml` ignores, and never repairs anything: no
 * `chmod` of an existing file or directory, no dropped entry, no removed stale lock.
 */

const FILE_NAME = "allowed.yaml";
const MAX_BYTES = 1024 * 1024;
const POLL_MS = 50;
export const DEFAULT_LOCK_WAIT_MS = 10_000;

export type AllowChange = { kind: "set"; entry: AllowedEntry } | { kind: "remove" };

export interface AllowUpdate {
  change: AllowChange;
  /** The real root of the repository. */
  repoRoot: string;
  home: string;
  /** Read by name only: `XDG_STATE_HOME`, the two account variables, `__MISE_DIFF`, `DIRENV_DIFF`. */
  env: Env;
  /** `process.getuid?.()`; undefined refuses, as the reader ignores the file. */
  uid: number | undefined;
  /** How long to wait for another run's lock file; default 10 s. */
  waitMs?: number;
  /** Called once, the first time the lock file is found held by another run. For tests. */
  onLockBusy?: () => void;
}

/** `reason` is a fixed phrase with paths through `displayPath`: it holds no repository text. */
export type AllowResult =
  | { kind: "written" | "unchanged" }
  | { kind: "refused"; reason: string; written: boolean };

type Placement = { refused: string } | { target: string; existing: boolean; linked: boolean };

function refusal(reason: string): { refused: string } {
  return { refused: reason };
}

/**
 * Where `path` is when nothing on the way is a link: the path as located, with `$HOME` spelled
 * as its real path (a temporary or network folder can be reached through a link of the system's
 * own). A file whose real path differs from it is a link, or lies below one.
 */
function lexicalReal(path: string, home: string): string {
  return isWithin(home, path) ? join(realOrResolved(home), relative(home, path)) : path;
}

function isDirectory(path: string): boolean {
  try {
    return lstatSync(path).isDirectory();
  } catch {
    return false;
  }
}

/**
 * Where `allowed.yaml` is written, decided the way the reader decides what it reads. Before any
 * file system call: `$HOME` absolute, a user id, a path that names no account folder, and no
 * `XDG_STATE_HOME` that mise or direnv moved. Then: a regular file or a link to one is kept and
 * written at its real path; a missing file gets the directory it is looked for in, created with
 * mode `0700` only when `create` is set and only after it is known to lie outside the
 * repository and every account folder. A dangling link and anything that is not a regular file
 * refuse.
 */
function placeAllowedFile(input: AllowUpdate, create: boolean): Placement {
  const { home, env, uid, repoRoot } = input;
  if (!isAbsolute(home)) return refusal("HOME is not an absolute path");
  if (uid === undefined) {
    return refusal("the owner of the file cannot be checked on this platform");
  }
  const unplaced = unplacedReason(FILE_NAME, home, env);
  if (unplaced !== undefined) return refusal(unplaced);
  const located = locateAccountFile(FILE_NAME, home, env).path;
  const locatedDir = dirname(located);
  const ctx: AccountFileContext & { uid: number } = { home, env, uid, root: repoRoot };

  let present = true;
  try {
    lstatSync(located);
  } catch (error) {
    if ((error as { code?: string }).code !== "ENOENT") {
      return refusal("its path cannot be resolved");
    }
    present = false;
  }

  if (present) {
    let real: string;
    let regular: boolean;
    try {
      real = realpathSync.native(located);
      regular = lstatSync(real).isFile();
    } catch {
      return refusal("it is a link that does not resolve");
    }
    if (!regular) return refusal("it is not a regular file");
    const problem = placementProblem(real, realOrResolved(locatedDir), ctx, located);
    if (problem !== undefined) return refusal(problem);
    // The real path differs from the located one when the file or any folder above it is a
    // link: then the target may be somebody else's file, and it is refused before anyone is asked.
    const linked = real !== lexicalReal(located, home);
    if (linked && !mayOverwriteLinked(real)) return refusal(LINKED_REFUSAL);
    return { target: real, existing: true, linked };
  }

  // The file is missing: find the directory it would be in, from the nearest one that exists.
  let ancestor = locatedDir;
  const rest: string[] = [];
  while (!isPresent(ancestor)) {
    rest.unshift(basename(ancestor));
    if (dirname(ancestor) === ancestor) return refusal("its folder cannot be found");
    ancestor = dirname(ancestor);
  }
  let realAncestor: string;
  try {
    realAncestor = realpathSync.native(ancestor);
  } catch {
    return refusal("its folder cannot be resolved");
  }
  if (!isDirectory(realAncestor)) return refusal("a parent of its folder is not a directory");
  const wouldBeDir = join(realAncestor, ...rest);
  const problem = placementProblem(join(wouldBeDir, FILE_NAME), realAncestor, ctx, located);
  if (problem !== undefined) return refusal(problem);
  if (rest.length === 0 || !create) {
    return { target: join(wouldBeDir, FILE_NAME), existing: false, linked: false };
  }

  try {
    mkdirSync(insideRoot(realAncestor, relative(realAncestor, wouldBeDir)), {
      recursive: true,
      mode: 0o700,
    });
  } catch {
    return refusal(`its folder ${displayPath(wouldBeDir, home)} cannot be created`);
  }
  let realDir: string;
  try {
    realDir = realpathSync.native(wouldBeDir);
  } catch {
    return refusal("its folder cannot be resolved");
  }
  const created = placementProblem(join(realDir, FILE_NAME), realDir, ctx, located);
  return created === undefined
    ? { target: join(realDir, FILE_NAME), existing: false, linked: false }
    : refusal(created);
}

function isPresent(path: string): boolean {
  try {
    lstatSync(path);
    return true;
  } catch {
    return false;
  }
}

/**
 * The checks `updateAllowed` makes before it creates or opens anything, without creating
 * anything: a fixed phrase when the file could not be written, else undefined. `alz account
 * allow` runs it before it asks the person anything.
 */
export function allowedPlacementProblem(input: AllowUpdate): string | undefined {
  const placed = placeAllowedFile(input, false);
  return "refused" in placed ? placed.refused : undefined;
}

/**
 * A link is followed on purpose, but only to a file that is empty, `[]`, or starts with the
 * header alz writes: anything else may be somebody else's file (decision-20).
 */
function mayOverwriteLinked(target: string): FileIdentity | undefined {
  const read = readRegularFileWithId(target, MAX_BYTES);
  if (typeof read === "string") return undefined;
  const text = read.bytes.toString("utf8");
  const trimmed = text.trim();
  const fine = trimmed === "" || trimmed === "[]" || text.startsWith(ALLOW_HEADER);
  return fine ? { dev: read.dev, ino: read.ino } : undefined;
}

/** The file that was read, so a swap before the rename is noticed. */
interface FileIdentity {
  dev: number;
  ino: number;
}

const LINKED_REFUSAL =
  "allowed.yaml is a link to a file that is not empty, [] or written by alz account allow; alz does not overwrite it";

const sleep = (ms: number) => new Promise<void>((resolve) => setTimeout(resolve, ms));

type Lock = { fd: number } | { busy: true } | { error: string };

/** `O_CREAT|O_EXCL|O_NOFOLLOW`, mode 0600: a planted file or link in the way is never followed. */
async function acquire(lockPath: string, waitMs: number, onBusy?: () => void): Promise<Lock> {
  const deadline = Date.now() + waitMs;
  let waited = false;
  for (;;) {
    try {
      const fd = openSync(
        lockPath,
        constants.O_WRONLY | constants.O_CREAT | constants.O_EXCL | constants.O_NOFOLLOW,
        0o600,
      );
      try {
        writeSync(fd, `${process.pid}\n`);
      } catch (error) {
        // The lock file is ours (O_EXCL) but unusable: do not leave it behind.
        closeSync(fd);
        unlinkSync(lockPath);
        throw error;
      }
      return { fd };
    } catch (error) {
      const code = (error as { code?: string }).code ?? "unknown error";
      if (code !== "EEXIST") return { error: code };
      if (Date.now() >= deadline) return { busy: true };
      if (!waited) onBusy?.();
      waited = true;
      await sleep(POLL_MS);
    }
  }
}

/** Removes the lock file only when it is still the file this process created. */
function release(lockPath: string, fd: number): void {
  try {
    // Compare before closing: the descriptor is what proves which file is ours.
    const mine = fstatSync(fd);
    const now = lstatSync(lockPath);
    if (now.ino === mine.ino && now.dev === mine.dev) unlinkSync(lockPath);
  } catch {
    // A lock that cannot be removed is reported by the next run; nothing is forced.
  } finally {
    try {
      closeSync(fd);
    } catch {
      // already closed
    }
  }
}

/**
 * Records or removes the entry for `repoRoot` in `allowed.yaml`, under an exclusive lock file
 * next to the file that is replaced (decision-20). Waiting for the lock is asynchronous; once it
 * is held, everything up to the rename is synchronous. Refuses without changing anything when
 * the file or the directory is in a state the reader ignores, when the file holds an entry the
 * reader ignores, when the result would exceed 1 MiB or would not parse back to the same
 * entries. A remove with no file creates nothing.
 */
export async function updateAllowed(input: AllowUpdate): Promise<AllowResult> {
  const { change, home } = input;
  const refuse = (reason: string, written = false): AllowResult => ({
    kind: "refused",
    reason,
    written,
  });
  const placed = placeAllowedFile(input, change.kind === "set");
  if ("refused" in placed) return refuse(`allowed.yaml cannot be written: ${placed.refused}`);
  if (change.kind === "remove" && !placed.existing) return { kind: "unchanged" };
  const { target } = placed;
  const dir = dirname(target);
  const lockPath = insideRoot(dir, `${basename(target)}.lock`);

  const lock = await acquire(lockPath, input.waitMs ?? DEFAULT_LOCK_WAIT_MS, input.onLockBusy);
  if ("busy" in lock) {
    return refuse(
      `${displayPath(lockPath, home)} exists, so another alz account allow may be running; if none is, delete that file and run this again`,
    );
  }
  if ("error" in lock) {
    return refuse(`the lock file ${displayPath(lockPath, home)} cannot be created (${lock.error})`);
  }
  try {
    return writeUnderLock(input, placed, target, dir);
  } finally {
    release(lockPath, lock.fd);
  }
}

function writeUnderLock(
  update: AllowUpdate,
  where: { existing: boolean; linked: boolean },
  file: string,
  folder: string,
): AllowResult {
  const { home, env, uid, repoRoot } = update;
  const fail = (reason: string, written = false): AllowResult => ({
    kind: "refused",
    reason,
    written,
  });
  const ctx: AccountFileContext = { home, env, uid, root: repoRoot };
  const loaded = loadAllowState(ctx, new WarningList());
  const problem = stateProblem(loaded);
  if (problem !== undefined) return fail(problem);
  if (where.existing && loaded.file.status === "missing") {
    return fail("allowed.yaml went missing while it was being read");
  }
  if (loaded.file.status === "read") {
    let now: string;
    try {
      now = realpathSync.native(locateAccountFile(FILE_NAME, home, env).path);
    } catch {
      return fail("allowed.yaml could not be resolved again");
    }
    if (now !== file) return fail("allowed.yaml changed place while it was being read");
  }
  const identity = where.linked ? mayOverwriteLinked(file) : undefined;
  if (where.linked && identity === undefined) return fail(LINKED_REFUSAL);
  const { state } = loaded;
  const current = entryFor(state, repoRoot);
  const others = [
    ...state.entries.filter((item) => item.real !== repoRoot).map((item) => item.entry),
    ...state.unresolved,
  ];
  let next: AllowedEntry[];
  if (update.change.kind === "set") {
    if (current !== undefined && sameEntry(current, update.change.entry)) {
      return { kind: "unchanged" };
    }
    next = [...others, update.change.entry];
  } else {
    if (current === undefined) return { kind: "unchanged" };
    next = others;
  }

  const text = serializeAllowState(next);
  if (Buffer.byteLength(text) > MAX_BYTES) {
    return fail("the result would be larger than 1 MiB, which alz ignores");
  }
  let parsed: unknown;
  try {
    parsed = parseYamlLimited(text);
  } catch {
    parsed = undefined;
  }
  if (!isDeepStrictEqual(parsed, plainEntries(next))) {
    return fail("the result would not read back to the same entries");
  }

  const written = writeThroughTemp(folder, file, text, identity);
  if (written !== undefined) return fail(`allowed.yaml cannot be written (${written})`);

  const back = loadAllowState(ctx, new WarningList());
  const entry = entryFor(back.state, repoRoot);
  const wanted = update.change.kind === "set" ? update.change.entry : undefined;
  const reads =
    back.file.status === "read" &&
    back.state.shape === "list" &&
    back.state.ignored.length === 0 &&
    (wanted === undefined ? entry === undefined : entry !== undefined && sameEntry(entry, wanted));
  return reads
    ? { kind: "written" }
    : fail("allowed.yaml was written but does not read back", true);
}

/**
 * A temporary file in the same directory (`O_EXCL|O_NOFOLLOW`, mode 0600 whatever the umask),
 * written in full, flushed and renamed over the target. Returns an error code, or undefined.
 * Any failure removes the temporary file.
 */
function writeThroughTemp(
  dir: string,
  target: string,
  text: string,
  expected: FileIdentity | undefined,
): string | undefined {
  const temp = insideRoot(
    dir,
    `.${basename(target)}.${process.pid}-${randomBytes(4).toString("hex")}.tmp`,
  );
  let fd: number | undefined;
  try {
    fd = openSync(
      temp,
      constants.O_WRONLY | constants.O_CREAT | constants.O_EXCL | constants.O_NOFOLLOW,
      0o600,
    );
    fchmodSync(fd, 0o600);
    const bytes = Buffer.from(text);
    for (let at = 0; at < bytes.length; ) at += writeSync(fd, bytes, at, bytes.length - at);
    fsyncSync(fd);
    closeSync(fd);
    fd = undefined;
    // A link's target was checked when it was read: it must still be that file.
    if (expected !== undefined) {
      const now = lstatSync(target);
      if (now.dev !== expected.dev || now.ino !== expected.ino || !now.isFile()) {
        throw Object.assign(new Error("changed"), { code: "the target changed" });
      }
    }
    renameSync(temp, target);
    return undefined;
  } catch (error) {
    if (fd !== undefined) {
      try {
        closeSync(fd);
      } catch {
        // already closed
      }
    }
    try {
      unlinkSync(temp);
    } catch {
      // the temporary file was never created, or is already gone
    }
    return (error as { code?: string }).code ?? "unknown error";
  }
}
