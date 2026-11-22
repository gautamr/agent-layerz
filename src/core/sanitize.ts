/** Control characters: C0, DEL, C1, and the Unicode line and paragraph separators. */
export function isControl(code: number): boolean {
  return code <= 0x1f || (code >= 0x7f && code <= 0x9f) || code === 0x2028 || code === 0x2029;
}

const FORMAT = /^\p{Cf}$/u;

/** Unicode format characters: zero-width, bidi controls, BOM, soft hyphen and so on. */
export function isFormat(char: string): boolean {
  return FORMAT.test(char);
}

/** Text from an untrusted source, safe to print: no control or format characters. */
export function stripControl(text: string): string {
  return [...text]
    .filter((char) => !isControl(char.codePointAt(0) ?? 0) && !isFormat(char))
    .join("");
}

/** Text from an untrusted source for an error message: no control or format characters, quoted. */
export function quote(text: string): string {
  return JSON.stringify(stripControl(text));
}

/**
 * A path from the environment or the file system, as a report may show it: control and format
 * characters removed, and `home` written as `~`.
 */
export function displayPath(path: string, home: string | undefined): string {
  const text = stripControl(path);
  const base = home === undefined ? "" : stripControl(home);
  if (base && base !== "/" && (text === base || text.startsWith(`${base}/`))) {
    return `~${text.slice(base.length)}`;
  }
  return text;
}

/**
 * Text from an untrusted source for a prompt or a decision a person makes on it: quoted, with
 * every character other than printable ASCII shown as `\u{XXXX}` instead of being removed, so a
 * zero-width character, a bidirectional control or a look-alike letter is visible and two
 * sources that differ only by such a character never print alike.
 */
export function quoteEscaped(text: string): string {
  const body = [...text]
    .map((char) => {
      if (char === '"' || char === "\\") return `\\${char}`;
      const code = char.codePointAt(0) ?? 0;
      if (code >= 0x20 && code <= 0x7e) return char;
      return `\\u{${code.toString(16).toUpperCase().padStart(4, "0")}}`;
    })
    .join("");
  return `"${body}"`;
}
