import {
  chmodSync,
  lstatSync,
  mkdirSync,
  readdirSync,
  readFileSync,
  rmSync,
  symlinkSync,
  writeFileSync,
} from "node:fs";
import { relative } from "node:path";
import { describe, expect, it, vi } from "vitest";
import { hubEntry, remoteStack, type Setup, stateDir, writeAllowed } from "../account-workspace";
import {
  AGENT_SESSION_MARKERS,
  addHubV3,
  allowedPath,
  atTerminal,
  expectNoQuestion,
  HEADER,
  NAMES,
  onTerminal,
  readAllowed,
  ready,
  rebuildAt,
  removed,
  TERMINAL_CASE_MS,
  which,
} from "../allow-terminal";
import { tool } from "../exec";
import { snapshot } from "../helpers";

vi.setConfig({ testTimeout: TERMINAL_CASE_MS });

/**
 * What `alz account allow` and `--remove` refuse for a person at a terminal, and changes nothing
 * (R6.1b, TASK-55.7.17): the agent-session markers, the states of allowed.yaml and of its folder,
 * what the layers say, and where the file may live. Each refusal happens before any question, so
 * the run is never answered: the cases compare the files before and after. The counterparts
 * without a terminal are in file 32; the reference copies are in test/reference/32.
 */

/** Runs a command at a terminal and expects a refusal, with no question and nothing changed. */
async function refused(s: Setup, args: string[], extra: Record<string, string> = {}, dir = "repo") {
  const before = { home: snapshot(s.ws.home), repo: snapshot(`${s.real}/repo`) };
  const result = await atTerminal(s, args, extra, dir);
  expect(result.code, result.stderr).toBe(1);
  expectNoQuestion(result);
  expect(snapshot(s.ws.home)).toEqual(before.home);
  expect(snapshot(`${s.real}/repo`)).toEqual(before.repo);
  return result;
}

const ALLOW = ["account", "allow"];
const REMOVE = ["account", "allow", "--remove"];

describe("§13.20 who may run alz account allow, at a terminal (AC 1)", () => {
  const MARKER_VALUE = "marker-value-4f1c9";
  const cases = AGENT_SESSION_MARKERS.flatMap((marker) =>
    [false, true].flatMap((isRemove) =>
      ["", MARKER_VALUE].map((value) => [marker, isRemove, value] as const),
    ),
  );
  it.each(cases)(
    "refuses with %s set (remove: %s, value %j) even on a terminal, without asking",
    async (marker, isRemove, value) => {
      const s = await ready();
      const result = await refused(s, isRemove ? REMOVE : ALLOW, { [marker]: value });
      expect(result.stderr).toContain(marker);
      expect(result.stderr).toContain("an agent session is running here");
      expect(result.stderr + result.stdout).not.toContain(MARKER_VALUE);
      const ide = "run alz account allow in a terminal window outside the IDE";
      if (marker === "CLAUDECODE") expect(result.stderr).toContain(ide);
      else expect(result.stderr).not.toContain(ide);
    },
  );

  it.each([["--yes"], ["-y"], ["--force"], ["extra"]])(
    "has no way to answer for the person: %s is a usage error, even on a terminal",
    async (arg) => {
      const s = await ready();
      const before = snapshot(s.ws.home);
      const result = await atTerminal(s, [...ALLOW, arg]);
      expect(result.code).toBe(2);
      expect(result.stderr).toContain("usage: alz");
      expectNoQuestion(result);
      expect(snapshot(s.ws.home)).toEqual(before);
    },
  );
});

describe("§13.20 states alz account allow refuses at a terminal, and changes nothing (decision-20)", () => {
  const abs = (s: Setup) => `${s.real}/repo-b`;

  /** A state that changes files only; the run then uses the normal environment. */
  const setUp =
    (prepare: (s: Setup) => void) =>
    (s: Setup): Record<string, string> => {
      prepare(s);
      return {};
    };
  const entryOf = (s: Setup, account: Record<string, string>, root = abs(s)) => ({
    root,
    account,
    source: "s",
  });

  const states: [string, (s: Setup) => Record<string, string>, string][] = [
    [
      "a file that group can write",
      setUp((s) => {
        writeAllowed(s, []);
        chmodSync(allowedPath(s), 0o660);
      }),
      "group or others can write it",
    ],
    [
      "a file that others can write",
      setUp((s) => {
        writeAllowed(s, []);
        chmodSync(allowedPath(s), 0o606);
      }),
      "group or others can write it",
    ],
    [
      "a directory that group can write",
      setUp((s) => {
        writeAllowed(s, []);
        chmodSync(stateDir(s), 0o770);
      }),
      "can be written by group or others",
    ],
    [
      "a parent below HOME that group can write",
      setUp((s) => {
        writeAllowed(s, []);
        chmodSync(`${s.ws.home}/.local`, 0o775);
      }),
      "can be written by group or others",
    ],
    [
      "a parent below HOME that group can write, with no state folder yet",
      setUp((s) => {
        mkdirSync(`${s.ws.home}/.local/state`, { recursive: true });
        chmodSync(`${s.ws.home}/.local/state`, 0o775);
      }),
      "can be written by group or others",
    ],
    [
      "XDG_STATE_HOME moved while mise is active",
      (s) => ({ XDG_STATE_HOME: `${s.ws.home}/elsewhere`, __MISE_DIFF: "x" }),
      "XDG_STATE_HOME points away from its default and mise or direnv is active here",
    ],
    [
      "XDG_STATE_HOME moved while direnv is active",
      (s) => ({ XDG_STATE_HOME: `${s.ws.home}/elsewhere`, DIRENV_DIFF: "x" }),
      "XDG_STATE_HOME points away from its default and mise or direnv is active here",
    ],
    [
      "XDG_STATE_HOME inside the repository",
      (s) => ({ XDG_STATE_HOME: `${s.real}/repo/state` }),
      "it lies inside the current repository",
    ],
    [
      "XDG_STATE_HOME inside an account folder",
      (s) => ({ XDG_STATE_HOME: `${s.ws.home}/.claude-x/state` }),
      "it lies inside an account folder",
    ],
    [
      "a folder in place of the file",
      setUp((s) => {
        mkdirSync(`${stateDir(s)}/allowed.yaml`, { recursive: true, mode: 0o700 });
      }),
      "not a regular file",
    ],
    [
      "a FIFO in place of the file",
      setUp((s) => {
        mkdirSync(stateDir(s), { recursive: true, mode: 0o700 });
        tool("mkfifo", [allowedPath(s)]);
      }),
      "not a regular file",
    ],
    [
      "a file over 1 MiB",
      setUp((s) => {
        writeAllowed(s, [], `#${"x".repeat(1024 * 1024)}\n`);
      }),
      "at most 1 MiB",
    ],
    [
      "invalid YAML",
      setUp((s) => {
        writeAllowed(s, [], "root: [\n");
      }),
      "not valid YAML",
    ],
    [
      "a mapping at the top",
      setUp((s) => {
        writeAllowed(s, [], "a: 1\n");
      }),
      "is not a list",
    ],
    [
      "an entry with a relative root",
      setUp((s) => {
        writeAllowed(s, [entryOf(s, { claude: "x" }, "relative/path")]);
      }),
      "has entry 1 that alz ignores",
    ],
    [
      "an entry with a bad account name",
      setUp((s) => {
        writeAllowed(s, [entryOf(s, { claude: "Bad Name" })]);
      }),
      "has entry 1 that alz ignores",
    ],
    [
      "an entry whose root is not a real path",
      setUp((s) => {
        symlinkSync(abs(s), `${s.real}/link-b`);
        writeAllowed(s, [entryOf(s, { claude: "x" }, `${s.real}/link-b`)]);
      }),
      "has entry 1 that alz ignores",
    ],
    [
      "two entries for one root",
      setUp((s) => {
        writeAllowed(s, [entryOf(s, { claude: "x" }), entryOf(s, { claude: "x" })]);
      }),
      "has entries 1, 2 that alz ignores",
    ],
  ];

  it.each(states)(
    "refuses %s, with the reason, before any prompt",
    async (_label, prepare, reason) => {
      const s = await ready();
      const result = await refused(s, ALLOW, prepare(s));
      expect(result.stderr).toContain(reason);
    },
  );

  it.each(states.slice(0, 4).concat(states.slice(11)))(
    "refuses `--remove` the same way for %s",
    async (_label, prepare, reason) => {
      const s = await ready();
      const result = await refused(s, REMOVE, prepare(s));
      expect(result.stderr).toContain(reason);
    },
  );

  it("refuses a relative HOME", async () => {
    const s = await ready();
    const result = await atTerminal(s, ALLOW, { HOME: "relative/home" });
    expect(result.code).toBe(1);
    expect(result.stderr).toContain("HOME is not an absolute path");
    expectNoQuestion(result);
  });
});

describe("§13.20 what the layers say decides whether alz account allow can run, at a terminal (AC 1)", () => {
  it("says only to run alz build on a cache miss: a lock that is missing", async () => {
    const s = await ready();
    rmSync(`${s.real}/repo/layer.lock`);
    const result = await refused(s, ALLOW);
    expect(result.stderr).toContain(
      "the layer cache cannot be verified (layer.lock is missing); run alz build",
    );
    expect(result.stdout).toBe("");
  });

  it("says only to run alz build on a cache miss: a cached layer that no longer matches the lock", async () => {
    const s = await ready();
    const entry = readdirSync(s.cache).find((name) => name.startsWith("tree2@"));
    expect(entry).toBeDefined();
    writeFileSync(`${s.cache}/${entry}/instructions/10-client.md`, "- Tampered.\n");
    const result = await refused(s, ALLOW);
    expect(result.stderr).toContain("(a layer no longer matches layer.lock); run alz build");
  });

  it("says only to run alz build on a cache miss: a cache that XDG_CACHE_HOME moved under mise", async () => {
    const s = await ready();
    const result = await refused(s, ALLOW, {
      XDG_CACHE_HOME: `${s.ws.home}/othercache`,
      __MISE_DIFF: "x",
    });
    expect(result.stderr).toContain("XDG_CACHE_HOME points away from ~/.cache");
    expect(result.stderr).toContain("run alz build");
  });

  it("refuses when the layers name no account, and names --remove when this root has an entry", async () => {
    const s = await ready();
    addHubV3(s, "");
    await rebuildAt(s, "v3");
    const plain = await refused(s, ALLOW);
    expect(plain.stderr).toContain("the layers name no account here");
    expect(plain.stderr).not.toContain("--remove");

    writeAllowed(s, [hubEntry(s)]);
    const withEntry = await refused(s, ALLOW);
    expect(withEntry.stderr).toContain("the layers name no account here");
    expect(withEntry.stderr).toContain("alz account allow --remove");
  });

  it("refuses the reserved name alz-blocked", async () => {
    const s = await ready();
    addHubV3(s, "account: alz-blocked\n");
    await rebuildAt(s, "v3");
    const result = await refused(s, ALLOW);
    expect(result.stderr).toContain("alz-blocked is reserved");
  });

  it("refuses when several client layers declare", async () => {
    const s = await ready();
    const files = (name: string) => ({
      "layer.yaml": `name: ${name}\nkind: client\nvisibility: private\nversion: 0.1.0\naccount: client-a\n`,
      "instructions/10-x.md": `- ${name}.\n`,
    });
    remoteStack(s, [
      {
        url: "https://example.invalid/first.git",
        ref: "v1",
        sha: "3".repeat(40),
        files: {
          "layer.yaml": "name: first\nkind: personal\nvisibility: public\nversion: 1.0.0\n",
          "instructions/10-x.md": "- first.\n",
        },
      },
      {
        url: "https://example.invalid/one.git",
        ref: "v1",
        sha: "1".repeat(40),
        files: files("client-one"),
      },
      {
        url: "https://example.invalid/two.git",
        ref: "v1",
        sha: "2".repeat(40),
        files: files("client-two"),
      },
    ]);
    const result = await refused(s, ALLOW);
    expect(result.stderr).toContain(
      'several client layers declare an account ("client-one", "client-two")',
    );
    expect(result.stderr).toContain("remove one declaration, then run alz build");
  });

  it("refuses when layer.yaml cannot be read, but --remove still works", async () => {
    const s = await ready();
    writeAllowed(s, [hubEntry(s)]);
    writeFileSync(`${s.real}/repo/layer.yaml`, "stack: [\n");
    const result = await refused(s, ALLOW);
    expect(result.stderr).toContain(`repair ${s.real}/repo/layer.yaml`);

    const done = await removed(s);
    expect(done.code, done.stderr).toBe(0);
    expect(readAllowed(s)).toBe(`${HEADER}[]\n`);
  });

  it("refuses outside a repository, inside an account folder and inside a configured folder", async () => {
    const s = await ready();
    const none = await refused(s, ALLOW, {}, "personal");
    expect(none.stderr).toContain("repository: none (no repository here)");
    mkdirSync(`${s.ws.home}/.claude-x`);
    const folder = await refused(s, ALLOW, {}, relative(s.ws.root, `${s.ws.home}/.claude-x`));
    expect(folder.stderr).toContain("the working directory is inside an account folder");
    const configured = await refused(s, ALLOW, { CLAUDE_CONFIG_DIR: `${s.real}/repo` });
    expect(configured.stderr).toContain("inside a folder CLAUDE_CONFIG_DIR or CODEX_HOME names");
  });
});

describe("§13.20 where allowed.yaml may live, and what a link may point at, at a terminal (decision-20 amendment)", () => {
  const target = (s: Setup) => `${s.ws.home}/store/target.yaml`;
  const linked = (s: Setup, content: string | undefined) => {
    mkdirSync(`${s.ws.home}/store`, { recursive: true, mode: 0o700 });
    chmodSync(`${s.ws.home}/store`, 0o700);
    if (content !== undefined) writeFileSync(target(s), content, { mode: 0o600 });
    else mkdirSync(target(s), { mode: 0o700 });
    mkdirSync(stateDir(s), { recursive: true, mode: 0o700 });
    symlinkSync(target(s), allowedPath(s));
  };

  it("refuses a state directory inside a git work tree, for a missing file and an existing one", async () => {
    const missing = await ready();
    mkdirSync(`${missing.ws.home}/.local/state/.git`, { recursive: true });
    const result = await refused(missing, ALLOW);
    expect(result.stderr).toContain("it lies inside a git work tree (~/.local/state/.git)");

    const existing = await ready();
    writeAllowed(existing, []);
    mkdirSync(`${existing.ws.home}/.local/state/.git`);
    for (const args of [ALLOW, REMOVE]) {
      const second = await refused(existing, args);
      expect(second.stderr).toContain(
        "allowed.yaml is ignored (it lies inside a git work tree (~/.local/state/.git))",
      );
    }
  });

  it("refuses a link whose target directory is inside a git work tree", async () => {
    const s = await ready();
    const store = `${s.ws.home}/store`;
    mkdirSync(`${store}/.git`, { recursive: true, mode: 0o700 });
    chmodSync(store, 0o700);
    writeFileSync(`${store}/target.yaml`, "[]\n", { mode: 0o600 });
    mkdirSync(stateDir(s), { recursive: true, mode: 0o700 });
    symlinkSync(`${store}/target.yaml`, allowedPath(s));
    const result = await refused(s, ALLOW);
    expect(result.stderr).toContain("git work tree");
  });

  it.each([
    ["an empty file", ""],
    ["a file that holds only []", "[]\n"],
    ["a file that starts with alz's own header", `${HEADER}[]\n`],
  ])("follows a link to %s", async (_label, content) => {
    const s = await ready();
    linked(s, content);
    const session = onTerminal(s, ALLOW);
    for (const name of NAMES) {
      await session.waitForStderr(`Type ${name} to allow`);
      session.type(`${name}\n`);
    }
    const result = await session.done();
    expect(result.code, result.stderr).toBe(0);
    expect(lstatSync(allowedPath(s)).isSymbolicLink()).toBe(true);
    expect(readFileSync(target(s), "utf8")).toContain(`- root: ${s.real}/repo\n`);
  });

  it.each([
    ["a file somebody else wrote", "- root: /somewhere\n  account:\n    claude: x\n  source: /y\n"],
    ["a file with a comment of another program", "# another tool's file\n[]\n"],
  ])("refuses a link to %s, and changes nothing", async (_label, content) => {
    const s = await ready();
    linked(s, content);
    const before = snapshot(s.ws.home);
    const result = await refused(s, ALLOW);
    expect(result.stderr).toContain("not empty, [] or written by alz account allow");
    // --remove reads the same file: this root has no entry in it, so there is nothing to remove.
    const none = await atTerminal(s, REMOVE);
    expect(none.code, none.stderr).toBe(0);
    expect(none.stdout).toContain("nothing to remove");
    expect(snapshot(s.ws.home)).toEqual(before);
  });

  /** The state folder's parent is a link: the file's real path differs from where it is looked for. */
  const linkedParent = (s: Setup, content: string) => {
    const real = `${s.ws.home}/realstate`;
    mkdirSync(`${real}/agent-layerz`, { recursive: true, mode: 0o700 });
    chmodSync(real, 0o700);
    chmodSync(`${real}/agent-layerz`, 0o700);
    writeFileSync(`${real}/agent-layerz/allowed.yaml`, content, { mode: 0o600 });
    mkdirSync(`${s.ws.home}/.local`, { recursive: true, mode: 0o700 });
    chmodSync(`${s.ws.home}/.local`, 0o700);
    symlinkSync(real, `${s.ws.home}/.local/state`);
    return `${real}/agent-layerz/allowed.yaml`;
  };

  it("refuses a file reached through a linked folder when it is not alz's, before any prompt", async () => {
    const s = await ready();
    linkedParent(s, "- root: /somewhere\n  account:\n    claude: x\n  source: /y\n");
    const result = await refused(s, ALLOW);
    expect(result.stderr).toContain("not empty, [] or written by alz account allow");
  });

  it("writes a file reached through a linked folder when it is empty, and keeps the link", async () => {
    const s = await ready();
    const file = linkedParent(s, "");
    const session = onTerminal(s, ALLOW);
    for (const name of NAMES) {
      await session.waitForStderr(`Type ${name} to allow`);
      session.type(`${name}\n`);
    }
    const result = await session.done();
    expect(result.code, result.stderr).toBe(0);
    expect(lstatSync(`${s.ws.home}/.local/state`).isSymbolicLink()).toBe(true);
    expect(readFileSync(file, "utf8")).toContain(`- root: ${s.real}/repo\n`);
    expect(readdirSync(`${s.ws.home}/realstate/agent-layerz`)).toEqual(["allowed.yaml"]);
  });

  it("refuses a link placed inside a git work tree even when its target is outside every work tree", async () => {
    const s = await ready();
    const tree = `${s.ws.home}/tree`;
    const outside = `${s.ws.home}/outside/state`;
    mkdirSync(`${tree}/.git`, { recursive: true, mode: 0o700 });
    chmodSync(tree, 0o700);
    mkdirSync(`${outside}/agent-layerz`, { recursive: true, mode: 0o700 });
    chmodSync(`${s.ws.home}/outside`, 0o700);
    chmodSync(outside, 0o700);
    chmodSync(`${outside}/agent-layerz`, 0o700);
    writeFileSync(`${outside}/agent-layerz/allowed.yaml`, "[]\n", { mode: 0o600 });
    symlinkSync(outside, `${tree}/state`);
    const extra = { XDG_STATE_HOME: `${tree}/state` };
    for (const args of [ALLOW, REMOVE]) {
      const result = await refused(s, args, extra);
      expect(result.stderr).toContain("git work tree (~/tree/.git)");
    }
    const listed = await which(s, extra);
    expect(listed.stdout).toContain("ignored: it lies inside a git work tree (~/tree/.git)");
  });

  it("refuses a link to a directory", async () => {
    const s = await ready();
    linked(s, undefined);
    const result = await refused(s, ALLOW);
    expect(result.stderr).toContain("not a regular file");
  });
});
