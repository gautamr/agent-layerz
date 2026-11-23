import { mkdirSync, rmSync, symlinkSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { afterAll, describe, expect, it } from "vitest";
import YAML from "yaml";
import { parseYamlLimited } from "../../src/core/yaml";
import { ENV_NAMES as srcEnvNames } from "../../src/resolve/account-resolution";
import * as srcCache from "../../src/resolve/cache-paths";
import { hashLayerDir } from "../../src/resolve/layer-hash";
import {
  MAX_LAYER_BYTES as srcMaxLayerBytes,
  MAX_LAYER_ENTRIES as srcMaxLayerEntries,
  MAX_LAYER_YAML_BYTES as srcMaxYamlBytes,
} from "../../src/resolve/reader-load";
import { githubCloneUrl as srcGithubCloneUrl } from "../../src/resolve/source-location";
import { tempDir } from "../run-root";
import * as spec from "../spec/cache";
import { contentHash, hashedFiles } from "../spec/content-hash";
import {
  ENV_NAMES,
  MAX_LAYER_BYTES,
  MAX_LAYER_ENTRIES,
  MAX_READ_FILE_BYTES,
  YAML_ALIAS_LIMIT,
} from "../spec/limits";

/**
 * The acceptance suite writes the spec's cache names, hashes and limits in test code
 * (test/spec/*), from docs/LAYERING-SPEC.md. While the TypeScript implementation exists, this
 * reference test keeps each test-side formula equal to its src/ counterpart, so a drift on
 * either side shows up here. It is deleted with src/ at the cutover.
 */

const SHA1 = "0123456789abcdef0123456789abcdef01234567";
const SHA256 = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

const CLONE_ARGS = [
  "https://github.com/acme-corp/layer.git",
  "https://github.com-acme-corp/layer",
  "https://example.invalid/personal.git",
  "ssh://git@example.invalid/a/b.git",
  "/private/var/folders/xy/T/alz-1/hub/.git",
  "/work/répertoire/日本語/.git",
  "",
];

const REAL_DIRS = ["/work/hub/layer", "/private/var/x/hub/layer", "/home/you/é/layer", "/"];
const TREES = [SHA1, SHA256];
const SHAS = [SHA1, SHA256];

describe("cache names equal the implementation's", () => {
  it.each(CLONE_ARGS)("git cache key of %j", (cloneArg) => {
    expect(spec.gitCacheKey(cloneArg)).toBe(srcCache.gitCacheKey(cloneArg));
    for (const sha of SHAS) {
      expect(spec.gitEntryName(cloneArg, sha)).toBe(srcCache.gitEntryName(cloneArg, sha));
    }
  });

  it.each(TREES)("tree entry of %s", (tree) => {
    expect(spec.treeEntryName(tree)).toBe(srcCache.treeEntryName(tree));
  });

  it.each(REAL_DIRS)("path index entry of %j", (realDir) => {
    for (const sha of SHAS) {
      expect(spec.pathIndexName(realDir, sha)).toBe(srcCache.pathIndexName(realDir, sha));
    }
  });

  it.each([
    ["/home/you", {}],
    ["/home/you", { XDG_CACHE_HOME: "/var/cache-x" }],
    ["/home/you", { XDG_CACHE_HOME: "/private/var/folders/xy/T/alz-1/cache" }],
    ["/Users/a b/é", {}],
  ] as const)("cache root of HOME %j with %j", (home, env) => {
    expect(spec.cacheRoot(home, env)).toBe(srcCache.cacheRoot(home, env));
  });

  it.each(["acme-corp/layer", "a/b", "owner/repo.name"])("GitHub clone URL of %s", (ownerRepo) => {
    expect(spec.githubCloneUrl(ownerRepo)).toBe(srcGithubCloneUrl(ownerRepo));
    expect(spec.githubCloneUrl(ownerRepo)).toBe(`https://github.com/${ownerRepo}.git`);
  });
});

describe("content hash equals the implementation's", () => {
  const roots: string[] = [];
  const layer = (
    files: Record<string, string | Uint8Array>,
    links: Record<string, string> = {},
  ) => {
    const dir = tempDir("alz-spec-formulas-");
    roots.push(dir);
    for (const [path, content] of Object.entries(files)) {
      mkdirSync(join(dir, path, ".."), { recursive: true });
      writeFileSync(join(dir, path), content);
    }
    for (const [path, target] of Object.entries(links)) symlinkSync(target, join(dir, path));
    return dir;
  };
  afterAll(() => {
    for (const dir of roots) rmSync(dir, { recursive: true, force: true });
  });

  const cases: [string, () => string][] = [
    ["an empty layer", () => layer({})],
    [
      "a layer with nested folders",
      () =>
        layer({
          "layer.yaml": "name: x\n",
          "instructions/10-a.md": "- a\n",
          "instructions/sub/20-b.md": "- b\n",
          "a.b": "dot",
          "a/b": "slash",
        }),
    ],
    [
      "the files the hash leaves out",
      () =>
        layer({
          "layer.yaml": "name: x\n",
          ".env": "SECRET=1\n",
          ".env.dev": "SECRET=2\n",
          "sub/.env.prod": "SECRET=3\n",
          ".DS_Store": "x",
          "sub/.DS_Store": "x",
          ".git/config": "x",
          "node_modules/pkg/index.js": "x",
          "sub/node_modules/pkg/index.js": "x",
          "kept.md": "kept\n",
        }),
    ],
    ["a link, which is not listed", () => layer({ "real.md": "r\n" }, { "link.md": "real.md" })],
    [
      "binary content",
      () => layer({ "bin/data.bin": Uint8Array.from([0, 255, 1, 128, 10, 13, 0]) }),
    ],
    [
      "names in uppercase, lowercase and punctuation",
      () => layer({ B: "1", a: "2", _c: "3", "-d": "4" }),
    ],
    [
      "non-ASCII names in the basic plane",
      () => layer({ "é.md": "e", "z.md": "z", "日本.md": "j" }),
    ],
    ["an empty file and a file with a newline", () => layer({ "empty.md": "", "nl.md": "\n" })],
  ];

  it.each(cases)("of %s", (_name, make) => {
    const dir = make();
    expect(contentHash(dir)).toBe(hashLayerDir(dir).contentHash);
    expect(hashedFiles(dir)).toEqual(hashLayerDir(dir).files);
  });

  const astralLayer = () => layer({ "\u{FF5E}.md": "bmp", "\u{1F600}.md": "astral" });

  it("orders by code point, as §5.3 says, not by UTF-16 unit", () => {
    // U+FF5E is a larger code point than U+1F600's high surrogate U+D83D in UTF-16 but a smaller
    // one than U+1F600, so the two orders differ.
    expect(hashedFiles(astralLayer())).toEqual(["\u{FF5E}.md", "\u{1F600}.md"]);
  });

  // KNOWN DRIFT (reported in TASK-55.2.2): src/core/files.ts `compareCodePoints` compares with
  // `<`, which orders by UTF-16 unit, so a layer with a file name outside the basic plane gets a
  // different content hash from the one §5.3 describes. `it.fails` turns green-to-red when src/
  // is fixed, so this marker is removed with the fix. The Rust port sorts by code point.
  it.fails("hashes a name outside the basic plane as the implementation does", () => {
    const dir = astralLayer();
    expect(contentHash(dir)).toBe(hashLayerDir(dir).contentHash);
  });
});

describe("limits equal the implementation's", () => {
  it("has the same layer entry and byte limits and the 1 MiB reader limit", () => {
    expect(MAX_LAYER_ENTRIES).toBe(srcMaxLayerEntries);
    expect(MAX_LAYER_BYTES).toBe(srcMaxLayerBytes);
    expect(MAX_READ_FILE_BYTES).toBe(srcMaxYamlBytes);
  });

  it("reads the same environment variables", () => {
    expect([...ENV_NAMES].sort()).toEqual([...srcEnvNames].sort());
  });

  it("refuses the same number of YAML aliases", () => {
    const aliases = (count: number) =>
      `a: &a [x]\nrefs: [${Array.from({ length: count }, () => "*a").join(", ")}]\n`;
    const test = (text: string) => YAML.parse(text, { maxAliasCount: YAML_ALIAS_LIMIT });
    for (const count of [1, YAML_ALIAS_LIMIT - 1, YAML_ALIAS_LIMIT, YAML_ALIAS_LIMIT + 1, 150]) {
      const outcome = (parse: (text: string) => unknown) => {
        try {
          parse(aliases(count));
          return "parsed";
        } catch {
          return "refused";
        }
      };
      expect(outcome(test), `${count} aliases`).toBe(outcome(parseYamlLimited));
    }
  });
});
