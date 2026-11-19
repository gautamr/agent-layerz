import { InputError } from "./diagnostics";
import { isMap, parseYaml, type YamlMap } from "./yaml";

export interface MarkdownDoc {
  data: YamlMap;
  body: string;
}

const OPENING = "---\n";

/** Splits a Markdown file into YAML frontmatter and body. A file without frontmatter has `data = {}`. */
export function parseMarkdown(text: string): MarkdownDoc {
  const normalized = text.replace(/\r\n/g, "\n");
  if (!normalized.startsWith(OPENING)) return { data: {}, body: normalized };
  const close = normalized.indexOf("\n---", OPENING.length - 1);
  if (close < 0) throw new InputError("frontmatter is not closed with '---'");
  const lineEnd = normalized.indexOf("\n", close + 4);
  const yamlText = normalized.slice(OPENING.length, close + 1);
  const body = lineEnd < 0 ? "" : normalized.slice(lineEnd + 1);
  const data = parseYaml(yamlText) ?? {};
  if (!isMap(data)) throw new InputError("frontmatter must be a YAML mapping");
  return { data, body };
}

/** Inserts a YAML comment as the first frontmatter line, leaving the rest of the file byte-for-byte. */
export function insertFrontmatterComment(text: string, comment: string): string {
  const normalized = text.replace(/\r\n/g, "\n");
  if (!normalized.startsWith(OPENING)) throw new InputError("file has no frontmatter");
  return `${OPENING}# ${comment}\n${normalized.slice(OPENING.length)}`;
}
