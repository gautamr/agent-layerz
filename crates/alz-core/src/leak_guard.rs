//! The leak guard for public layers (spec §9.2): what `alz lint` and `alz build` check before a
//! public layer is published.
//!
//! Every file of a public layer is read as text and checked against three things: the layer's
//! `forbid_patterns` (errors, `leak-forbid`), the built-in secret scan (errors, `leak-secret`) and
//! the email and private-host heuristics (warnings, `leak-email` and `leak-host`). A diagnostic
//! names the layer, the file and, for a pattern, its number; it never holds the text that matched
//! or the text of a pattern, so a guard that finds a secret does not print it.
//!
//! The guard is a pure function of a layer's metadata and the bytes the walk read. A layer that is
//! not public is not checked.

mod forbid;
mod heuristics;
mod layer_yaml;
mod text;

use crate::diagnostic::{Code, Diagnostic, Diagnostics, Message, Where};
use crate::layer::{LayerContents, LayerMeta, Visibility};
use crate::path::SafeRelPath;
use crate::secret;
use crate::untrusted_text::Stripped;

use forbid::ForbidPatterns;

/// The file that holds `forbid_patterns`.
const LAYER_YAML: &str = "layer.yaml";

/// Checks a layer against the leak guard. A layer whose visibility is not `public` has nothing to
/// report. Diagnostics come in this order: the refused `forbid_patterns` entries, then for each
/// file in name order its pattern matches, its secrets and its warnings.
#[must_use]
pub fn check(meta: &LayerMeta, contents: &LayerContents) -> Diagnostics {
    match meta.visibility() {
        Visibility::Public => guard(meta, contents),
        Visibility::Private | Visibility::Confidential => Diagnostics::new(),
    }
}

fn guard(meta: &LayerMeta, contents: &LayerContents) -> Diagnostics {
    let layer = Stripped::from_untrusted(meta.name().as_str());
    let patterns = ForbidPatterns::compile(meta.forbid_patterns());
    let refused = patterns.refusals(&layer_yaml_place(&layer));
    contents.iter().fold(refused, |found, (path, bytes)| {
        found.merge(file_findings(&layer, &patterns, path, bytes))
    })
}

fn layer_yaml_place(layer: &Stripped) -> Where {
    Where::layer_file(layer.clone(), Stripped::from_untrusted(LAYER_YAML))
}

/// What one file holds: the text is read lossily, so a file that is not UTF-8, or is binary,
/// is checked like any other and nothing can hide in it.
fn file_findings(
    layer: &Stripped,
    patterns: &ForbidPatterns,
    path: &SafeRelPath,
    bytes: &[u8],
) -> Diagnostics {
    let at = Where::layer_file(layer.clone(), Stripped::from_untrusted(path.as_str()));
    let text = text::decode(bytes);
    // The patterns are written in `layer.yaml`, so only there are they left out of what they are
    // matched against. The secret scan and the heuristics read the whole file: a token or an
    // address in a `forbid_patterns` line, or in a comment after it, is still reported.
    let matched_text = if path.as_str() == LAYER_YAML {
        layer_yaml::without_forbid_patterns(&text)
    } else {
        text.clone()
    };
    let matched = patterns.matching(&matched_text).map(|number| {
        Diagnostic::error(
            Code::LeakForbid,
            Some(at.clone()),
            Message::new("matches forbid_patterns entry #").then(number.get()),
        )
    });
    let secrets = secret::scan(&text).into_iter().map(|kind| {
        Diagnostic::error(
            Code::LeakSecret,
            Some(at.clone()),
            Message::new("looks like a secret (")
                .then(kind.name())
                .then("); remove it from the public layer"),
        )
    });
    let email = heuristics::has_unlisted_email(&text).then(|| {
        Diagnostic::warning(
            Code::LeakEmail,
            Some(at.clone()),
            Message::new("contains an email address; check that it may be public"),
        )
    });
    let host = heuristics::has_private_host(&text).then(|| {
        Diagnostic::warning(
            Code::LeakHost,
            Some(at.clone()),
            Message::new(
                "contains what looks like a private host name; check that it may be public",
            ),
        )
    });
    matched.chain(secrets).chain(email).chain(host).collect()
}

#[cfg(test)]
mod tests;
