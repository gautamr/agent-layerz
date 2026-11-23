import { appendFileSync, writeFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import { tool } from "../exec";
import { fixture, layers, readFile, readTree, type Workspace } from "../helpers";

const REPOS = ["repo-a", "repo-b"];

function git(ws: Workspace, args: string[]): void {
  tool("git", args, {
    cwd: `${ws.root}/hub`,
    env: {
      PATH: process.env.PATH ?? "",
      HOME: ws.home,
      GIT_CONFIG_GLOBAL: "/dev/null",
      GIT_CONFIG_NOSYSTEM: "1",
      GIT_AUTHOR_NAME: "test",
      GIT_AUTHOR_EMAIL: "test@example.com",
      GIT_COMMITTER_NAME: "test",
      GIT_COMMITTER_EMAIL: "test@example.com",
    },
  });
}

/** The fixture's hub as a git repo whose layer is tagged layer-v0.1.0. */
function hubWorkspace(): Workspace {
  const ws = fixture("16-hub");
  git(ws, ["init", "--quiet", "--initial-branch=main"]);
  git(ws, ["add", "."]);
  git(ws, ["commit", "--quiet", "-m", "hub: client layer and docs"]);
  git(ws, ["tag", "layer-v0.1.0"]);
  return ws;
}

const outputs = (ws: Workspace, repo: string) =>
  readTree(`${ws.root}/${repo}`, [".layer", "layer.yaml"]);

describe("§13.16 hub", () => {
  it("each repo resolves path:../hub/layer at layer-v0.1.0 and grants the hub as an additional directory", async () => {
    const ws = hubWorkspace();
    for (const repo of REPOS) {
      const result = await layers(ws, repo, ["build"]);
      expect(result.code, result.stderr).toBe(0);
      expect(readFile(ws, `${repo}/AGENTS.md`)).toContain("CLIENT-RULE-V1");
      const settings = JSON.parse(readFile(ws, `${repo}/.claude/settings.json`));
      expect(settings.permissions.additionalDirectories).toEqual(["../hub"]);
      const lock = JSON.parse(readFile(ws, `${repo}/layer.lock`));
      expect(lock.layers[1]).toMatchObject({ name: "acme", rev: "layer-v0.1.0" });
    }
  });

  it("hub commits and edits after the tag do not change either repo's build", async () => {
    const ws = hubWorkspace();
    const before: Record<string, Record<string, string>> = {};
    for (const repo of REPOS) {
      expect((await layers(ws, repo, ["build"])).code).toBe(0);
      before[repo] = outputs(ws, repo);
    }

    appendFileSync(`${ws.root}/hub/docs/product/overview.md`, "Version 2.\n");
    git(ws, ["commit", "--quiet", "-am", "docs: product overview v2"]);
    // Not yet released as a layer tag: neither a commit nor an uncommitted edit may leak in.
    appendFileSync(`${ws.root}/hub/layer/instructions/10-domain.md`, "- CLIENT-RULE-V2\n");

    for (const repo of REPOS) {
      const check = await layers(ws, repo, ["check"]);
      expect(check.code, check.stderr).toBe(0);
      expect((await layers(ws, repo, ["build"])).code).toBe(0);
      expect(outputs(ws, repo)).toEqual(before[repo]);
    }
  });

  it("doctor warns about a CLAUDE.md at the client root", async () => {
    const ws = hubWorkspace();
    expect((await layers(ws, "repo-a", ["build"])).code).toBe(0);
    const doctor = await layers(ws, "repo-a", ["doctor"]);
    expect(doctor.stdout).toContain(`warning: client root ${ws.root}/CLAUDE.md exists`);
    expect(doctor.stdout).not.toContain("is a git repo");
  });

  it("a ref the hub does not have fails with a clear error", async () => {
    const ws = fixture("16-hub");
    git(ws, ["init", "--quiet", "--initial-branch=main"]);
    git(ws, ["add", "."]);
    git(ws, ["commit", "--quiet", "-m", "hub"]);
    const result = await layers(ws, "repo-a", ["build"]);
    expect(result.code).toBe(1);
    expect(result.stderr).toContain("cannot resolve ref layer-v0.1.0 for 'path:../hub/layer'");
  });
});

// §13.28: the same cases through `alz lint`, which resolves the stack (§6) and writes nothing.
describe("§13.16 hub, through alz lint", () => {
  it("each repo resolves path:../hub/layer at layer-v0.1.0", async () => {
    const ws = hubWorkspace();
    for (const repo of REPOS) {
      const before = readTree(`${ws.root}/${repo}`);
      const result = await layers(ws, repo, ["lint"]);
      expect(result.code, result.stderr).toBe(0);
      expect(result.stdout).toContain("lint ok: ");
      expect(result.stdout).toContain("acme");
      expect(readTree(`${ws.root}/${repo}`)).toEqual(before);
    }
  });

  it("hub commits and edits after the tag do not change either repo's verdict", async () => {
    const ws = hubWorkspace();
    appendFileSync(`${ws.root}/hub/docs/product/overview.md`, "Version 2.\n");
    git(ws, ["commit", "--quiet", "-am", "docs: product overview v2"]);
    // A layer.yaml that is invalid, committed after the tag and then edited again in the work
    // tree: lint reads the tree at the tag, so neither a commit nor an edit may reach it.
    writeFileSync(`${ws.root}/hub/layer/layer.yaml`, "name: [broken\n");
    git(ws, ["commit", "--quiet", "-am", "layer: broken after the tag"]);
    appendFileSync(`${ws.root}/hub/layer/instructions/10-domain.md`, "- CLIENT-RULE-V2\n");

    for (const repo of REPOS) {
      const result = await layers(ws, repo, ["lint"]);
      expect(result.code, result.stderr).toBe(0);
      expect(result.stdout).toContain("lint ok: ");
    }
  });

  it("a ref the hub does not have is an error [source]", async () => {
    const ws = fixture("16-hub");
    git(ws, ["init", "--quiet", "--initial-branch=main"]);
    git(ws, ["add", "."]);
    git(ws, ["commit", "--quiet", "-m", "hub"]);
    const result = await layers(ws, "repo-a", ["lint"]);
    expect(result.code).toBe(1);
    expect(result.stderr).toContain(
      "error [source] cannot resolve ref layer-v0.1.0 for 'path:../hub/layer'",
    );
  });
});
