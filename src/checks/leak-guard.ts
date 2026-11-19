import { readFileSync } from "node:fs";
import { join } from "node:path";
import type { Diagnostics } from "../core/diagnostics";
import { isBinary } from "../core/files";
import { parseYaml, stringifyYaml } from "../core/yaml";
import { type Layer, where } from "../model/layer";
import { findSecrets } from "./secrets";

const EMAIL = /\b[A-Za-z0-9._%+-]+@([A-Za-z0-9-]+\.)+[A-Za-z]{2,}\b/g;
const PUBLIC_EMAIL_DOMAINS =
  /@(?:example\.(?:com|org|net)|users\.noreply\.github\.com|anthropic\.com)$/i;
const PRIVATE_HOST = /\b[a-z0-9-]+(?:\.[a-z0-9-]+)*\.(?:internal|corp|lan|intranet|local)\b/i;

/** §9.2 leak guard for public layers: forbid_patterns and secrets are errors; emails and private hosts warn. */
export function scanPublicLayer(layer: Layer, diags: Diagnostics): void {
  if (layer.meta.visibility !== "public") return;
  const forbid = compilePatterns(layer, diags);
  for (const file of layer.files) {
    const data = readFileSync(join(layer.dir, file));
    if (isBinary(data)) continue;
    let text = data.toString("utf8");
    if (file === "layer.yaml") text = withoutForbidPatterns(text);
    text.split("\n").forEach((line, index) => {
      const loc = `${where(layer, file)}:${index + 1}`;
      for (const { source, regex } of forbid) {
        if (regex.test(line))
          diags.error("leak-forbid", `matches forbid_patterns entry ${source}`, loc);
      }
      for (const name of findSecrets(line)) {
        diags.error(
          "leak-secret",
          `looks like a secret (${name}); remove it from the public layer`,
          loc,
        );
      }
      for (const email of line.match(EMAIL) ?? []) {
        if (!PUBLIC_EMAIL_DOMAINS.test(email)) {
          diags.warn("leak-email", "contains an email address; check that it may be public", loc);
        }
      }
      if (PRIVATE_HOST.test(line)) {
        diags.warn("leak-host", "contains what looks like a private hostname", loc);
      }
    });
  }
}

function compilePatterns(layer: Layer, diags: Diagnostics): { source: string; regex: RegExp }[] {
  return layer.meta.forbidPatterns.flatMap((source, index) => {
    // Accept the PCRE-style leading (?i) the spec uses; JS needs it as a flag.
    const caseInsensitive = source.startsWith("(?i)");
    try {
      const regex = new RegExp(
        caseInsensitive ? source.slice(4) : source,
        caseInsensitive ? "i" : "",
      );
      return [{ source: `#${index + 1}`, regex }];
    } catch {
      diags.error(
        "leak-forbid",
        `forbid_patterns #${index + 1} is not a valid regex`,
        where(layer, "layer.yaml"),
      );
      return [];
    }
  });
}

/** The patterns themselves live in layer.yaml; scan the rest of it. */
function withoutForbidPatterns(text: string): string {
  const raw = parseYaml(text) as Record<string, unknown>;
  const { forbid_patterns: _ignored, ...rest } = raw;
  return stringifyYaml(rest);
}
