import { existsSync, mkdirSync, readFileSync, renameSync, rmSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import YAML from "yaml";
import { insideRoot } from "../core/files";
import { stringifyYaml } from "../core/yaml";
import { MANIFEST_FILE } from "../model/manifest";
import { pruneEmptyDirs } from "../render/writer";
import type { AdoptionPlan } from "./plan";

/** Writes an adoption plan that has no problems: layer files, moves, removals, then the manifest. */
export function applyAdoption(plan: AdoptionPlan): void {
  const { repoRoot, layerDir } = plan;
  for (const [path, content] of plan.layerFiles) {
    const abs = insideRoot(repoRoot, join(layerDir, path));
    mkdirSync(dirname(abs), { recursive: true });
    writeFileSync(abs, typeof content === "string" ? content : stringifyYaml(content.yaml));
  }
  for (const { from, to } of plan.moves) {
    const abs = insideRoot(repoRoot, join(layerDir, to));
    mkdirSync(dirname(abs), { recursive: true });
    renameSync(insideRoot(repoRoot, from), abs);
    pruneEmptyDirs(dirname(join(repoRoot, from)), repoRoot);
  }
  for (const path of plan.removes) {
    rmSync(insideRoot(repoRoot, path));
    pruneEmptyDirs(dirname(join(repoRoot, path)), repoRoot);
  }
  editManifest(repoRoot, plan.manifest);
}

/** Adds directories and MCP ids to the repo manifest, keeping its comments and layout. */
function editManifest(repoRoot: string, edits: AdoptionPlan["manifest"]): void {
  if (edits.addDirs.length === 0 && edits.mcpEnable.length === 0) return;
  const path = join(repoRoot, MANIFEST_FILE);
  if (!existsSync(path)) return;
  const doc = YAML.parseDocument(readFileSync(path, "utf8"));
  appendUnique(doc, ["workspace", "add_dirs"], edits.addDirs);
  appendUnique(doc, ["mcp", "enable"], edits.mcpEnable);
  writeFileSync(path, doc.toString({ lineWidth: 0, flowCollectionPadding: false }));
}

function appendUnique(doc: YAML.Document, keys: string[], values: string[]): void {
  if (values.length === 0) return;
  const current = doc.getIn(keys);
  const existing = YAML.isSeq(current)
    ? current.items.map((item) => String(YAML.isScalar(item) ? item.value : item))
    : [];
  const added = values.filter((value) => !existing.includes(value));
  if (YAML.isSeq(current)) for (const value of added) current.add(value);
  else doc.setIn(keys, [...existing, ...added]);
}
