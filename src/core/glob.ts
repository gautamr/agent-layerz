/**
 * Matches a repo-relative POSIX path against a glob: `**` spans directories, `*` and `?` do
 * not. A pattern without `/` matches the file name at any depth (gitignore style).
 */
export function matchesGlob(path: string, pattern: string): boolean {
  const target = pattern.includes("/") ? path : path.slice(path.lastIndexOf("/") + 1);
  return globToRegExp(pattern.replace(/^\/+/, "")).test(target);
}

export function matchesAny(path: string, patterns: readonly string[]): boolean {
  return patterns.some((pattern) => matchesGlob(path, pattern));
}

function globToRegExp(pattern: string): RegExp {
  let source = "";
  for (let i = 0; i < pattern.length; i++) {
    const char = pattern.charAt(i);
    if (char === "*" && pattern[i + 1] === "*") {
      const slash = pattern[i + 2] === "/";
      source += slash ? "(?:.*/)?" : ".*";
      i += slash ? 2 : 1;
    } else if (char === "*") source += "[^/]*";
    else if (char === "?") source += "[^/]";
    else source += char.replace(/[.+^${}()|[\]\\]/g, "\\$&");
  }
  return new RegExp(`^${source}$`);
}
