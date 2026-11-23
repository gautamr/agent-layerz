import { appendFileSync, mkdirSync } from "node:fs";
import { describe, expect, it } from "vitest";
import { tool } from "../exec";
import { fixture, layers, readFile, type Workspace } from "../helpers";

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

describe("§10 alz init --stack --hub", () => {
  it("writes a buildable stack and pins the hub's client layer to its newest layer-v* tag", async () => {
    const ws = fixture("16-hub");
    git(ws, ["init", "--quiet", "--initial-branch=main"]);
    git(ws, ["add", "."]);
    git(ws, ["commit", "--quiet", "-m", "client layer"]);
    git(ws, ["tag", "layer-v0.1.0"]);
    appendFileSync(`${ws.root}/hub/layer/instructions/10-domain.md`, "- A later rule.\n");
    git(ws, ["commit", "--quiet", "-am", "client layer 0.10"]);
    git(ws, ["tag", "layer-v0.10.0"]);
    mkdirSync(`${ws.root}/new-repo`);

    const init = await layers(ws, "new-repo", [
      "init",
      "--stack",
      "../personal",
      "--hub",
      "../hub",
    ]);
    expect(init.code).toBe(0);
    expect(init.stdout).toContain("path:../hub/layer at layer-v0.10.0");
    const manifest = readFile(ws, "new-repo/layer.yaml");
    expect(manifest).toContain(
      "  - source: path:../personal\n  - source: path:../hub/layer\n    ref: layer-v0.10.0\n  - source: path:./.layer",
    );
    expect(manifest).toContain("  hub: ../hub");

    const build = await layers(ws, "new-repo", ["build"]);
    expect(build.code).toBe(0);
    expect(readFile(ws, "new-repo/AGENTS.md")).toContain("A later rule.");
    const settings = JSON.parse(readFile(ws, "new-repo/.claude/settings.json"));
    expect(settings.permissions.additionalDirectories).toEqual(["../hub"]);
  });

  it("writes --stack values as path, pinned path and github sources into layer.yaml", async () => {
    const ws = fixture("16-hub");
    mkdirSync(`${ws.root}/new-repo`);

    // The unresolvable first entry stops resolution before the github entry, so no clone is tried.
    const init = await layers(ws, "new-repo", [
      "init",
      "--stack",
      "../nowhere",
      "--stack",
      "path:../hub/layer@layer-v1.0.0",
      "--stack",
      "github:me/agent-layerz-personal@v1.2.0",
    ]);
    expect(init.code).toBe(0);
    expect(readFile(ws, "new-repo/layer.yaml")).toContain(
      [
        "stack:",
        "  - source: path:../nowhere",
        "  - source: path:../hub/layer",
        "    ref: layer-v1.0.0",
        "  - source: github:me/agent-layerz-personal@v1.2.0",
        "  - source: path:./.layer",
      ].join("\n"),
    );
  });
});
