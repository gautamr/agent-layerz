import { parseArgs } from "node:util";
import { version } from "../../package.json";
import { FatalError, formatDiagnostic } from "../core/diagnostics";
import { stripControl } from "../core/sanitize";
import {
  ACCOUNT_TARGETS,
  type AccountMap,
  type AccountTarget,
  isAccountName,
} from "../model/account";
import { accountWhichCommand } from "./commands/account";
import { accountAllowCommand } from "./commands/account-allow";
import { buildCommand } from "./commands/build";
import { checkCommand } from "./commands/check";
import { diffCommand } from "./commands/diff";
import { doctorCommand } from "./commands/doctor";
import { explainCommand } from "./commands/explain";
import { initCommand } from "./commands/init";
import { lintCommand } from "./commands/lint";
import { lsCommand } from "./commands/ls";
import type { Io } from "./io";

export type { Io } from "./io";

const USAGE = `usage: alz <command> [options]   (alz --version)

  init [--stack <source>]... [--hub <dir>]   scaffold a repo: these layers, then .layer/
  init --kind layer [--name <n>] [--visibility <v>]   scaffold a new layer
  build [--adopt]                     resolve, merge, render, write, update layer.lock
                                      --adopt turns hand-written .claude/, CLAUDE.md, AGENTS.md
                                      and .mcp.json into repo-layer sources first
  check [--skip-identity]             fail if the repo differs from a fresh build or the lock,
                                      or the git identity differs from the declared commit
                                      identity (--skip-identity: CI runners, other people)
  ls [--live]                         what the stack builds, by layer; --live also asks Claude
                                      what it loaded (one short model call) and compares
  explain <type> <name>               which layers contributed to an item (agent, skill, command, rule, hook, mcp, instruction)
  diff                                the change a pending build would make
  doctor                              account-dir collisions, tool versions, slots, budgets
  lint [--layer <path>]               schema, leak guard and slots of a layer
  account which [--use <target>=<name>]...   the account each target would use here, and why
  account allow [--remove]            accept (or withdraw) the account this repository's layers name;
                                      a person at a terminal outside an agent session only
`;

/** `--use <target>=<name>`, as the alz shell function passes an `alz use` choice. */
function parseUse(items: string[]): AccountMap | string {
  const map: AccountMap = {};
  for (const item of items) {
    const at = item.indexOf("=");
    const target = item.slice(0, at);
    const name = item.slice(at + 1);
    if (at < 0 || !Object.hasOwn(ACCOUNT_TARGETS, target)) {
      return "--use needs <target>=<name>, with claude or codex as the target";
    }
    if (!isAccountName(name))
      return "--use needs an account name (lower-case letters, digits and '-')";
    if (map[target as AccountTarget] !== undefined) return `--use names ${target} twice`;
    map[target as AccountTarget] = name;
  }
  return map;
}

/** Runs one CLI invocation and returns the exit code: 0 ok, 1 failed, 2 usage error. */
export async function run(argv: string[], io: Io): Promise<number> {
  const [command, ...rest] = argv;
  try {
    switch (command) {
      case "init": {
        const { values } = parseArgs({
          args: rest,
          options: {
            kind: { type: "string", default: "repo" },
            name: { type: "string" },
            visibility: { type: "string" },
            stack: { type: "string", multiple: true, default: [] },
            hub: { type: "string" },
          },
        });
        return initCommand(io, {
          kind: values.kind,
          name: values.name,
          visibility: values.visibility,
          stack: values.stack,
          hub: values.hub,
        });
      }
      case "build": {
        const { values } = parseArgs({
          args: rest,
          options: { adopt: { type: "boolean", default: false } },
        });
        return buildCommand(io, { adopt: values.adopt });
      }
      case "check": {
        const { values } = parseArgs({
          args: rest,
          options: { "skip-identity": { type: "boolean", default: false } },
        });
        return checkCommand(io, { skipIdentity: values["skip-identity"] });
      }
      case "diff":
        parseArgs({ args: rest, options: {} });
        return diffCommand(io);
      case "doctor":
        parseArgs({ args: rest, options: {} });
        return doctorCommand(io);
      case "ls": {
        const { values } = parseArgs({
          args: rest,
          options: { live: { type: "boolean", default: false } },
        });
        return lsCommand(io, { live: values.live });
      }
      case "explain": {
        const { positionals } = parseArgs({ args: rest, options: {}, allowPositionals: true });
        return explainCommand(io, positionals[0], positionals[1]);
      }
      case "lint": {
        const { values } = parseArgs({ args: rest, options: { layer: { type: "string" } } });
        return lintCommand(io, values.layer);
      }
      case "account": {
        const [sub, ...args] = rest;
        if (sub === "allow") {
          // Strict: `--yes`, `-y`, any other option and any positional is a usage error. There
          // is no way to answer for the person.
          const { values } = parseArgs({
            args,
            options: { remove: { type: "boolean", default: false } },
            strict: true,
            allowPositionals: false,
          });
          return accountAllowCommand(io, { remove: values.remove });
        }
        if (sub !== "which") {
          io.stderr(`unknown account command '${stripControl(sub ?? "")}'\n\n${USAGE}`);
          return 2;
        }
        const { values } = parseArgs({
          args,
          options: { use: { type: "string", multiple: true, default: [] } },
        });
        const use = parseUse(values.use);
        if (typeof use === "string") {
          io.stderr(`${use}\n\n${USAGE}`);
          return 2;
        }
        return accountWhichCommand(io, { use });
      }
      case "--version":
      case "-v":
        io.stdout(`${version}\n`);
        return 0;
      case undefined:
      case "help":
      case "--help":
      case "-h":
        io.stdout(USAGE);
        return command === undefined ? 2 : 0;
      default:
        io.stderr(`unknown command '${stripControl(command)}'\n\n${USAGE}`);
        return 2;
    }
  } catch (error) {
    if (error instanceof FatalError) {
      io.stderr(
        `${formatDiagnostic({ level: "error", code: error.code, message: error.message, where: error.where })}\n`,
      );
      return 1;
    }
    if ((error as { code?: string }).code?.startsWith("ERR_PARSE_ARGS")) {
      io.stderr(`${stripControl((error as Error).message)}\n\n${USAGE}`);
      return 2;
    }
    throw error;
  }
}
