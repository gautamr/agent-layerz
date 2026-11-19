import { type Diagnostics, InputError, UNRESOLVED } from "../core/diagnostics";
import { asStringList, unknownKeys } from "../core/yaml";
import { kindTitle, type Layer, sourceRef, where } from "../model/layer";
import { SKILL_NAME } from "../model/vocabulary";
import type { Contribution } from "./agents";
import { filesIn, readLayerMarkdown, stem } from "./layer-files";

export interface RuleSpec {
  name: string;
  /** Globs relative to the repo root; the rule loads only for matching files. */
  paths: string[];
  body: string;
  sources: Contribution[];
}

const FIELDS = ["paths", "mode"];

/**
 * §7.10: identity is the filename. A more specific layer replaces the whole rule, or with
 * `mode: extend` appends a section and adds its `paths`.
 */
export function mergeRules(layers: Layer[], diags: Diagnostics): Map<string, RuleSpec> {
  const rules = new Map<string, RuleSpec>();
  for (const layer of layers) {
    for (const path of filesIn(layer, "rules", ".md")) {
      const doc = readLayerMarkdown(layer, path, diags);
      if (!doc) continue;
      const loc = where(layer, path);
      const name = stem(path, ".md");
      const extra = unknownKeys(doc.data, FIELDS);
      if (extra.length > 0) {
        diags.error("rule", `unknown field(s): ${extra.join(", ")}`, loc);
        continue;
      }
      if (!SKILL_NAME.test(name)) {
        diags.error("rule", `rule name '${name}' must be kebab-case`, loc);
        continue;
      }
      const mode = doc.data.mode ?? "replace";
      if (mode !== "replace" && mode !== "extend") {
        diags.error("rule", "'mode' must be replace or extend", loc);
        continue;
      }
      let paths: string[];
      try {
        paths = doc.data.paths === undefined ? [] : asStringList(doc.data.paths, "paths");
      } catch (error) {
        if (!(error instanceof InputError)) throw error;
        diags.error("rule", error.message, loc);
        continue;
      }
      if (paths.some((glob) => glob.trim() === "")) {
        diags.error("rule", "'paths' entries must be non-empty globs", loc);
        continue;
      }
      const body = doc.body.trim();
      const ref = sourceRef(layer, path);
      const previous = rules.get(name);

      if (mode === "extend") {
        if (!previous) {
          diags.error(
            UNRESOLVED,
            `mode: extend, but no more general layer has rules/${name}.md`,
            loc,
          );
          continue;
        }
        previous.paths = [...new Set([...previous.paths, ...paths])];
        if (body) {
          previous.body = `${previous.body}\n\n## ${kindTitle(layer)}: ${layer.meta.name}\n\n${body}`;
        }
        previous.sources.push({ ref, action: `extend: paths +[${paths.join(", ")}]` });
        continue;
      }

      // Without `paths` Claude loads the rule in every session, outside the instructions budget.
      if (paths.length === 0) {
        diags.error(
          "rule",
          "'paths' is required; always-loaded text belongs in instructions/",
          loc,
        );
        continue;
      }
      const sources = (previous?.sources ?? []).map((source) => ({
        ...source,
        action: `${source.action} (overridden)`,
      }));
      rules.set(name, { name, paths, body, sources: [...sources, { ref, action: "rule" }] });
    }
  }
  return rules;
}
