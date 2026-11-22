import { existsSync, mkdirSync, writeFileSync } from "node:fs";
import { basename, join, posix } from "node:path";
import { type Diagnostic, Diagnostics, FatalError, formatDiagnostic } from "../../core/diagnostics";
import { ensureLocalIdentity } from "../../identity/setup";
import { mergeAccount } from "../../merge/account";
import { mergeCommitIdentity } from "../../merge/commit-identity";
import { NAME_PATTERN, VISIBILITIES, type Visibility } from "../../model/layer-meta";
import { readLock } from "../../model/lock";
import type { StackEntry } from "../../model/manifest";
import { readManifest } from "../../model/manifest";
import { git } from "../../resolve/git";
import { loadStack } from "../../resolve/stack";
import { buildContext, type Io } from "../io";

export interface InitOptions {
  kind: string;
  name?: string;
  visibility?: string;
  /** Layers below the repo layer, most general first. */
  stack: string[];
  hub?: string;
}

const MISE_TASKS = `[tasks."alz:build"]
run = "alz build"

[tasks."alz:check"]
run = "alz check"

[tasks."alz:diff"]
run = "alz diff"

[tasks."alz:doctor"]
run = "alz doctor"
`;

/** The repo manifest: the given layers, then the repo layer, which is always last. */
function repoManifest(stack: StackEntry[], hub: string | undefined): string {
  const entries = stack.map(
    (entry) => `  - source: ${entry.source}\n${entry.ref ? `    ref: ${entry.ref}\n` : ""}`,
  );
  const example =
    stack.length > 0
      ? ""
      : "  # Most general first, for example:\n  # - source: github:<you>/agent-layer-personal@v1.0.0\n";
  const workspace = hub
    ? `workspace:\n  hub: ${hub}             # granted to agents as an additional directory\n`
    : "# workspace:\n#   hub: ../hub             # granted to agents as an additional directory\n";
  return `stack:
${example}${entries.join("")}  - source: path:./.layer   # the repo layer, always last
targets: [claude]
${workspace}# protected_paths: []       # extra globs the build never touches
mcp:
  enable: []
budgets:
  instructions_max_lines: 400
options:
  commit_generated: true
  strict_slots: true
`;
}

/** `--stack` value → stack entry. A bare path means `path:`; `path:<dir>@<ref>` pins a ref. */
export function parseStackOption(value: string): StackEntry {
  if (/^(github|git):/.test(value)) return { source: value };
  const path = value.startsWith("path:") ? value.slice(5) : value;
  const at = path.lastIndexOf("@");
  if (at > 0 && !path.slice(at + 1).includes("/")) {
    return { source: `path:${path.slice(0, at)}`, ref: path.slice(at + 1) };
  }
  return { source: `path:${path}` };
}

/** §4.4: the hub's client layer at `<hub>/layer`, pinned to its newest `layer-v*` tag if it has one. */
function hubLayer(io: Io, hub: string): StackEntry | undefined {
  const dir = join(io.cwd, hub, "layer");
  if (!existsSync(join(dir, "layer.yaml"))) return undefined;
  let tag: string | undefined;
  try {
    tag = git(["tag", "--list", "layer-v*", "--sort=-v:refname"], io.env, dir).split("\n")[0];
  } catch {
    tag = undefined;
  }
  return { source: `path:${posix.join(hub, "layer")}`, ref: tag || undefined };
}

/** `alz init [--kind repo|layer]`: scaffold a repo manifest + `.layer/`, or a new layer. */
export function initCommand(io: Io, options: InitOptions): number {
  if (options.kind !== "repo" && options.kind !== "layer") {
    io.stderr(
      "usage: alz init [--kind repo|layer] [--name <name>] [--visibility public|private|confidential]\n",
    );
    return 2;
  }
  if (existsSync(join(io.cwd, "layer.yaml"))) {
    io.stderr("error [init] layer.yaml already exists here\n");
    return 1;
  }
  const name = options.name ?? toName(basename(io.cwd));
  if (!NAME_PATTERN.test(name)) {
    io.stderr(`error [init] '${name}' is not a valid layer name; pass --name\n`);
    return 1;
  }
  const visibility = (options.visibility ??
    (options.kind === "repo" ? "private" : "public")) as Visibility;
  if (!VISIBILITIES.includes(visibility)) {
    io.stderr(`error [init] --visibility must be one of ${VISIBILITIES.join(", ")}\n`);
    return 1;
  }

  const written: string[] = [];
  const write = (path: string, content: string) => {
    mkdirSync(join(io.cwd, path, ".."), { recursive: true });
    writeFileSync(join(io.cwd, path), content);
    written.push(path);
  };

  if (options.kind === "repo") {
    const stack = options.stack.map(parseStackOption);
    const client = options.hub ? hubLayer(io, options.hub) : undefined;
    if (client && !stack.some((entry) => entry.source === client.source)) {
      stack.push(client);
      io.stdout(
        `using the hub's client layer ${client.source}${client.ref ? ` at ${client.ref}` : ""}\n`,
      );
    }
    write("layer.yaml", repoManifest(stack, options.hub));
    if (!existsSync(join(io.cwd, ".layer", "layer.yaml"))) {
      write(".layer/layer.yaml", layerMeta(name, "repo", visibility));
      write(
        ".layer/instructions/50-repo.md",
        "- Describe this repo's commands, paths and contracts here.\n",
      );
    }
    if (!existsSync(join(io.cwd, "mise.toml")) && !existsSync(join(io.cwd, ".mise.toml"))) {
      write("mise.toml", MISE_TASKS);
    } else {
      io.stdout(`add these tasks to your mise config:\n\n${MISE_TASKS}\n`);
    }
  } else {
    write("layer.yaml", layerMeta(name, "layer", visibility));
    write("instructions/00-core.md", "- Write the standards of this layer here.\n");
  }
  for (const path of written) io.stdout(`created ${path}\n`);
  io.stdout(
    options.kind === "repo"
      ? "next: run `alz build --adopt` to take over existing agent config, or `alz build`\n"
      : "next: run `alz lint`\n",
  );
  if (options.kind === "repo") setUpIdentity(io);
  return 0;
}

/**
 * §9.4: with a declared commit identity, sets the repo-local git identity. Problems with the
 * stack or with a declaration are warnings here (init still succeeds; `alz build` fails on
 * them), and nothing is written then. Only a source that cannot be resolved yet gets the note
 * that `alz build --adopt` will set the identity.
 */
function setUpIdentity(io: Io): void {
  const ctx = buildContext(io);
  const diags = new Diagnostics();
  const warn = (item: Diagnostic) =>
    io.stderr(`${formatDiagnostic({ ...item, level: "warning" })}\n`);
  try {
    const layers = loadStack(
      readManifest(io.cwd),
      {
        repoRoot: io.cwd,
        home: ctx.home,
        env: io.env,
        locked: readLock(io.cwd)?.layers ?? [],
      },
      diags,
    );
    const declared = mergeCommitIdentity(layers, io.cwd, diags);
    mergeAccount(layers, io.cwd, diags);
    for (const item of diags.items) warn(item);
    if (declared && !diags.hasErrors) ensureLocalIdentity(io.cwd, declared, io.env, io);
  } catch (error) {
    if (!(error instanceof FatalError)) throw error;
    if (error.code === "source") {
      io.stderr(
        "note [identity] the stack could not be resolved yet, so no commit identity was set; `alz build --adopt` will set it\n",
      );
    } else {
      warn({ level: "error", code: error.code, message: error.message, where: error.where });
    }
  }
}

function layerMeta(name: string, kind: string, visibility: Visibility): string {
  return `name: ${name}\nkind: ${kind}\nvisibility: ${visibility}\nversion: 0.1.0\n`;
}

function toName(dir: string): string {
  return dir
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-+|-+$/g, "");
}
