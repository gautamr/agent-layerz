/** Built-in token shapes (§9.2 "a built-in secret scanner"). Names only; matches are never echoed. */
const SECRET_PATTERNS: { name: string; pattern: RegExp }[] = [
  {
    name: "GitHub token",
    pattern: /\b(?:gh[pousr]_[A-Za-z0-9]{36,}|github_pat_[A-Za-z0-9_]{50,})\b/,
  },
  { name: "Anthropic API key", pattern: /\bsk-ant-[A-Za-z0-9_-]{20,}/ },
  { name: "OpenAI-style API key", pattern: /\bsk-(?:proj-)?[A-Za-z0-9_-]{32,}/ },
  { name: "AWS access key id", pattern: /\b(?:AKIA|ASIA)[0-9A-Z]{16}\b/ },
  { name: "Google API key", pattern: /\bAIza[0-9A-Za-z_-]{35}\b/ },
  { name: "Slack token", pattern: /\bxox[abposr]-[A-Za-z0-9-]{10,}/ },
  { name: "Stripe key", pattern: /\b[rs]k_live_[A-Za-z0-9]{16,}/ },
  { name: "private key", pattern: /-----BEGIN [A-Z ]*PRIVATE KEY-----/ },
  {
    name: "credential assignment",
    pattern:
      /\b(?:api[_-]?key|secret|token|password|passwd)\b["']?\s*[:=]\s*["']?[A-Za-z0-9_\-/+]{20,}/i,
  },
];

export function findSecrets(text: string): string[] {
  return SECRET_PATTERNS.filter(({ pattern }) => pattern.test(text)).map(({ name }) => name);
}
