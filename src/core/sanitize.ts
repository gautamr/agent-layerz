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
