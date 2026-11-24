// Regenerates the golden trees of `alz build` (TASK-55.6.8): what the TypeScript reference writes
// for every repository fixture of the acceptance suite that builds. The Rust `alz` must write the
// same paths, bytes and executable bits (crates/alz/tests/golden_trees.rs).
//
// Run from the repository root after `npm ci --ignore-scripts && npm run build`, and commit the
// output:
//
//   node crates/alz/tools/golden-trees.mjs
//
// The script reads test/fixtures and dist/cli.js and changes neither. For each repository folder
// of a fixture (a folder whose `layer.yaml` has a `stack`) it copies the whole fixture to a
// scratch folder, runs `alz build` there with an isolated HOME, and, when the build exits 0,
// writes to crates/alz/tests/fixtures/golden/<fixture>/<repo>/:
//   files/   every file the build created, with its path and bytes (git keeps the executable bit);
//   stdout   what the build printed on standard output;
//   stderr   what it printed on standard error.
// A repository whose build fails is listed on standard output and has no golden: its failure is
// what an acceptance case asserts, not a tree.
//
// The scratch copies hold only paths relative to the scratch folder in what is stored: the
// script refuses an output that names the scratch folder, so no machine path is committed.

import { execFileSync, spawnSync } from "node:child_process";
import {
  chmodSync,
  cpSync,
  existsSync,
  mkdirSync,
  mkdtempSync,
  readdirSync,
  readFileSync,
  rmSync,
  statSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, relative } from "node:path";

const fixtures = "test/fixtures";
const out = "crates/alz/tests/fixtures/golden";
const cli = join(process.cwd(), "dist/cli.js");
const scratch = mkdtempSync(join(tmpdir(), "alz-golden-"));

/** The files of a folder as relative paths, `.git` left out. */
function listFiles(dir, prefix = "") {
  return readdirSync(dir, { withFileTypes: true }).flatMap((entry) => {
    if (entry.name === ".git") return [];
    const path = prefix === "" ? entry.name : `${prefix}/${entry.name}`;
    if (entry.isDirectory()) return listFiles(join(dir, entry.name), path);
    return entry.isFile() ? [path] : [];
  });
}

const gitEnv = (home) => ({
  PATH: process.env.PATH ?? "",
  HOME: home,
  GIT_CONFIG_GLOBAL: "/dev/null",
  GIT_CONFIG_NOSYSTEM: "1",
  GIT_AUTHOR_NAME: "test",
  GIT_AUTHOR_EMAIL: "test@example.com",
  GIT_COMMITTER_NAME: "test",
  GIT_COMMITTER_EMAIL: "test@example.com",
  GIT_AUTHOR_DATE: "2000-01-01T00:00:00Z",
  GIT_COMMITTER_DATE: "2000-01-01T00:00:00Z",
});

/** The hub of `16-hub` as a git repository tagged `layer-v0.1.0`, as test 16 makes it. */
function tagHub(root, home) {
  const git = (...args) =>
    execFileSync("git", args, { cwd: join(root, "hub"), env: gitEnv(home), stdio: "pipe" });
  git("init", "--quiet", "--initial-branch=main");
  git("add", ".");
  git("commit", "--quiet", "-m", "hub: client layer and docs");
  git("tag", "layer-v0.1.0");
}

// Repositories the reference builds and the Rust `alz` refuses, which the specification states
// (reference drift): an overlay that widens an agent's tools by a command operator or a path
// scope (§7.2, test 04). A golden of the reference's tree would pin the drift.
const referenceDrift = new Set([
  "04-tool-narrowing/repo-widen-command-operator",
  "04-tool-narrowing/repo-widen-path-scope",
]);

rmSync(out, { recursive: true, force: true });
const failed = [];
for (const fixture of readdirSync(fixtures).sort()) {
  const source = join(fixtures, fixture);
  const repos = readdirSync(source, { withFileTypes: true })
    .filter((entry) => entry.isDirectory())
    .map((entry) => entry.name)
    .filter((name) => {
      const manifest = join(source, name, "layer.yaml");
      return existsSync(manifest) && /^stack:/m.test(readFileSync(manifest, "utf8"));
    })
    .sort();
  for (const repo of repos) {
    if (referenceDrift.has(`${fixture}/${repo}`)) continue;
    const root = join(scratch, fixture, repo);
    const home = join(root, "home");
    mkdirSync(home, { recursive: true });
    const copy = join(root, "work");
    cpSync(source, copy, { recursive: true, verbatimSymlinks: true });
    if (fixture === "16-hub") tagHub(copy, home);
    const repoDir = join(copy, repo);
    const before = new Set(listFiles(repoDir));
    const run = spawnSync("node", [cli, "build"], {
      cwd: repoDir,
      env: { ...gitEnv(home), XDG_CACHE_HOME: join(root, "cache") },
      encoding: "utf8",
    });
    if (run.status !== 0) {
      failed.push(`${fixture}/${repo} (exit ${run.status})`);
      continue;
    }
    for (const text of [run.stdout, run.stderr]) {
      if (text.includes(scratch))
        throw new Error(`${fixture}/${repo}: output names the scratch folder`);
    }
    const target = join(out, fixture, repo);
    for (const path of listFiles(repoDir).filter((path) => !before.has(path))) {
      const bytes = readFileSync(join(repoDir, path));
      if (bytes.includes(scratch))
        throw new Error(`${fixture}/${repo}/${path}: names the scratch folder`);
      const file = join(target, "files", path);
      mkdirSync(dirname(file), { recursive: true });
      writeFileSync(file, bytes);
      chmodSync(file, statSync(join(repoDir, path)).mode & 0o100 ? 0o755 : 0o644);
    }
    writeFileSync(join(target, "stdout"), run.stdout);
    writeFileSync(join(target, "stderr"), run.stderr);
    console.log(`${fixture}/${repo}: ${relative(target, join(target, "files"))} written`);
  }
}
rmSync(scratch, { recursive: true, force: true });
console.log(`no golden for a build that fails: ${failed.join(", ")}`);
