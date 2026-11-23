//! The built-in secret scan (spec §9.2): the fixed token shapes that the scan looks for in text.
//!
//! The scan answers one question, which kinds of token a text holds, and never returns the text
//! that matched: a finding is a [`SecretKind`], so no diagnostic built from one can print a secret.
//! The shapes are the reference's, compiled with `regex-lite` (decision-38) once, with the size
//! and nest limits that bound every pattern alz compiles. Matching uses `is_match`, which runs in
//! time proportional to the pattern times the text.
//!
//! `\b` and `\w` are ASCII only here, as in JavaScript without the `u` flag. `\s` is written as an
//! explicit class of the characters that JavaScript's `\s` has, because `regex-lite`'s is ASCII
//! only and a no-break space between a key and its value must not hide a credential.

use std::fmt;
use std::sync::LazyLock;

use regex_lite::{Regex, RegexBuilder};

/// The largest compiled size of a pattern, in bytes (decision-38).
pub const SIZE_LIMIT: usize = 64 * 1024;

/// The deepest nesting of a pattern (decision-38).
pub const NEST_LIMIT: u32 = 32;

/// A kind of token the scan knows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum SecretKind {
    /// A GitHub token: `ghp_`, `gho_`, `ghu_`, `ghs_`, `ghr_` or `github_pat_`.
    GitHubToken,
    /// An Anthropic API key.
    AnthropicApiKey,
    /// An OpenAI-style API key.
    OpenAiStyleApiKey,
    /// An AWS access key id.
    AwsAccessKeyId,
    /// A Google API key.
    GoogleApiKey,
    /// A Slack token.
    SlackToken,
    /// A live Stripe key.
    StripeKey,
    /// A private key block.
    PrivateKey,
    /// A long value assigned to a key named like `api_key`, `secret`, `token` or `password`.
    CredentialAssignment,
}

impl SecretKind {
    /// Every kind, in the order the scan reports them.
    pub const ALL: [Self; 9] = [
        Self::GitHubToken,
        Self::AnthropicApiKey,
        Self::OpenAiStyleApiKey,
        Self::AwsAccessKeyId,
        Self::GoogleApiKey,
        Self::SlackToken,
        Self::StripeKey,
        Self::PrivateKey,
        Self::CredentialAssignment,
    ];

    /// The kind as a diagnostic names it.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::GitHubToken => "GitHub token",
            Self::AnthropicApiKey => "Anthropic API key",
            Self::OpenAiStyleApiKey => "OpenAI-style API key",
            Self::AwsAccessKeyId => "AWS access key id",
            Self::GoogleApiKey => "Google API key",
            Self::SlackToken => "Slack token",
            Self::StripeKey => "Stripe key",
            Self::PrivateKey => "private key",
            Self::CredentialAssignment => "credential assignment",
        }
    }

    /// The pattern of the shape.
    const fn pattern(self) -> &'static str {
        match self {
            Self::GitHubToken => r"\b(?:gh[pousr]_[A-Za-z0-9]{36,}|github_pat_[A-Za-z0-9_]{50,})\b",
            Self::AnthropicApiKey => r"\bsk-ant-[A-Za-z0-9_-]{20,}",
            Self::OpenAiStyleApiKey => r"\bsk-(?:proj-)?[A-Za-z0-9_-]{32,}",
            Self::AwsAccessKeyId => r"\b(?:AKIA|ASIA)[0-9A-Z]{16}\b",
            Self::GoogleApiKey => r"\bAIza[0-9A-Za-z_-]{35}\b",
            Self::SlackToken => r"\bxox[abposr]-[A-Za-z0-9-]{10,}",
            Self::StripeKey => r"\b[rs]k_live_[A-Za-z0-9]{16,}",
            Self::PrivateKey => r"-----BEGIN [A-Z ]*PRIVATE KEY-----",
            Self::CredentialAssignment => {
                r#"(?i)\b(?:api[_-]?key|secret|token|password|passwd)\b["']?[\t\n\x0B\x0C\r \x{a0}\x{1680}\x{2000}-\x{200a}\x{2028}\x{2029}\x{202f}\x{205f}\x{3000}\x{feff}]*[:=][\t\n\x0B\x0C\r \x{a0}\x{1680}\x{2000}-\x{200a}\x{2028}\x{2029}\x{202f}\x{205f}\x{3000}\x{feff}]*["']?[A-Za-z0-9_\-/+]{20,}"#
            }
        }
    }

    /// The compiled shape, or `None` when the pattern does not compile (a test proves that none
    /// of them fails).
    fn compile(self) -> Option<Regex> {
        bounded(self.pattern()).ok()
    }
}

/// Compiles `pattern` with the limits of decision-38: a compiled size of at most [`SIZE_LIMIT`]
/// and a nesting of at most [`NEST_LIMIT`], in multi-line mode: `^` and `$` match at the start and
/// the end of each line, a line ending in a line feed, a carriage return or both (spec §9.2). The
/// token shapes use no anchor, so the mode changes nothing for them. Every pattern alz compiles, the token shapes and a
/// layer's `forbid_patterns` (§9.2), goes through here.
///
/// # Errors
///
/// The error of `regex-lite` when the pattern is not valid, is not supported or passes a limit.
pub(crate) fn bounded(pattern: &str) -> Result<Regex, regex_lite::Error> {
    RegexBuilder::new(pattern)
        .multi_line(true)
        .crlf(true)
        .size_limit(SIZE_LIMIT)
        .nest_limit(NEST_LIMIT)
        .build()
}

impl fmt::Display for SecretKind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.name())
    }
}

/// Every shape, compiled once. A shape that failed to compile would be absent, so the scan would
/// miss that kind; the test `every_token_shape_compiles` makes that impossible to ship.
static SHAPES: LazyLock<Vec<(SecretKind, Regex)>> = LazyLock::new(|| {
    SecretKind::ALL
        .into_iter()
        .filter_map(|kind| kind.compile().map(|shape| (kind, shape)))
        .collect()
});

/// The kinds of token that `text` holds, in the order of [`SecretKind::ALL`], each once. The text
/// that matched is not returned.
#[must_use]
pub fn scan(text: &str) -> Vec<SecretKind> {
    SHAPES
        .iter()
        .filter(|(_, shape)| shape.is_match(text))
        .map(|(kind, _)| *kind)
        .collect()
}

#[cfg(test)]
mod tests;
