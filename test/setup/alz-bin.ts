import { spawnSync } from "node:child_process";
import { accessSync, chmodSync, constants, realpathSync, statSync, writeFileSync } from "node:fs";
import { isAbsolute, join } from "node:path";
import { buildSync } from "esbuild";

const SOURCE_ROOT = join(import.meta.dirname, "..", "..");

/**
 * The binary the acceptance suite runs, as given in ALZ_BIN: an absolute path to an executable
 * regular file, checked after realpath. Throws a message that names ALZ_BIN for anything else,
 * so the run stops before the first test. The path is never run through a shell.
 */
export function checkedAlzBin(value: string): string {
  const refuse = (why: string): never => {
    throw new Error(
      `ALZ_BIN must be an absolute path to an executable file, but ALZ_BIN=${JSON.stringify(value)} ${why}`,
    );
  };
  if (value === "") return refuse("is empty (unset it to test the TypeScript reference)");
  if (!isAbsolute(value)) return refuse("is not an absolute path");
  const real = ((): string => {
    try {
      return realpathSync(value);
    } catch {
      return refuse("does not exist");
    }
  })();
  if (!statSync(real).isFile()) return refuse(`is not a regular file (${real})`);
  try {
    accessSync(real, constants.X_OK);
  } catch {
    return refuse(`is not executable (${real})`);
  }
  return value;
}

/** The longest interpreter path a `#!` line is trusted to hold (older kernels cut it at 127). */
export const MAX_SHEBANG_PATH_BYTES = 200;

/**
 * The first line of the reference binary: `#!<node>`. A `#!` line cannot quote or escape its
 * interpreter, so a path with whitespace (a space, a tab, a newline) would run the wrong program,
 * and one that is too long is cut. Both are refused, with the path in the message.
 */
export function shebangLine(nodePath: string): string {
  const why = /\s/.test(nodePath)
    ? "contains whitespace"
    : Buffer.byteLength(nodePath) > MAX_SHEBANG_PATH_BYTES
      ? `is longer than ${MAX_SHEBANG_PATH_BYTES} bytes`
      : undefined;
  if (why !== undefined) {
    throw new Error(
      `cannot build the TypeScript reference: the node path ${JSON.stringify(nodePath)} ${why}, and a #! line cannot hold it. Run the tests with a node installed under a plain path, or set ALZ_BIN.`,
    );
  }
  return `#!${nodePath}`;
}

/**
 * Bundles the TypeScript reference from src/cli/bin.ts with the options of `npm run
 * build:standalone` into `<dir>/alz`, mode 0755. The first line becomes `#!<absolute node path>`,
 * so a case that sets PATH to something without node still starts it.
 */
export function buildReference(dir: string, nodePath: string = process.execPath): string {
  const shebang = shebangLine(nodePath);
  const target = join(dir, "alz");
  const [output] = buildSync({
    entryPoints: [join(SOURCE_ROOT, "src", "cli", "bin.ts")],
    bundle: true,
    platform: "node",
    format: "esm",
    target: "node22",
    banner: {
      js: 'import{createRequire}from"node:module";const require=createRequire(import.meta.url);',
    },
    logLevel: "warning",
    write: false,
    outfile: target,
  }).outputFiles;
  if (output === undefined) throw new Error("esbuild produced no output for src/cli/bin.ts");
  const withoutShebang = output.text.replace(/^#![^\n]*\n/, "");
  writeFileSync(target, `${shebang}\n${withoutShebang}`);
  chmodSync(target, 0o755);
  return target;
}

/**
 * The pseudo-terminal driver `pty-run` (crates/alz-test-support), which the cases that need a
 * person at a terminal spawn instead of alz. `ALZ_PTY_RUN` names a built one (an absolute path,
 * checked like ALZ_BIN); without it the driver is built with cargo, which the Rust job and the
 * local toolchain have. It is a test tool, never shipped.
 */
export function ptyRunBin(given: string | undefined = process.env.ALZ_PTY_RUN): string {
  if (given !== undefined) {
    if (!isAbsolute(given)) throw new Error(`ALZ_PTY_RUN must be an absolute path, got ${given}`);
    accessSync(given, constants.X_OK);
    return given;
  }
  const built = spawnSync(
    "cargo",
    [
      "build",
      "--quiet",
      "--locked",
      "-p",
      "alz-test-support",
      "--bin",
      "pty-run",
      "--message-format=json",
    ],
    { cwd: SOURCE_ROOT, encoding: "utf8", maxBuffer: 64 * 1024 * 1024 },
  );
  if (built.status !== 0) {
    throw new Error(
      `cannot build the pseudo-terminal driver (cargo build -p alz-test-support --bin pty-run); set ALZ_PTY_RUN to a built one.\n${built.stderr}${built.error ?? ""}`,
    );
  }
  interface CargoMessage {
    reason?: string;
    executable?: string | null;
    target?: { name?: string };
  }
  const [executable] = built.stdout
    .split("\n")
    .filter((line) => line.startsWith("{"))
    .map((line) => JSON.parse(line) as CargoMessage)
    .filter((m) => m.reason === "compiler-artifact" && m.target?.name === "pty-run")
    .map((m) => m.executable)
    .filter((path): path is string => typeof path === "string")
    .slice(-1);
  if (executable === undefined) throw new Error("cargo built no pty-run executable");
  return executable;
}
