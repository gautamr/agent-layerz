//! The two heuristics of the leak guard (spec §9.2), which give warnings: an email address that is
//! not at a known public domain, and a private host name.

use std::sync::LazyLock;

use regex_lite::Regex;

use crate::secret::bounded;

/// An email address.
const EMAIL: &str = r"\b[A-Za-z0-9._%+-]+@(?:[A-Za-z0-9-]+\.)+[A-Za-z]{2,}\b";

/// The domains whose addresses may be public: examples, GitHub's no-reply addresses and the
/// vendor's. Tested against one matched address.
const PUBLIC_DOMAIN: &str =
    r"(?i)@(?:example\.(?:com|org|net)|users\.noreply\.github\.com|anthropic\.com)$";

/// A host name that ends in a name that is only used inside a network.
const PRIVATE_HOST: &str =
    r"(?i)\b[a-z0-9-]+(?:\.[a-z0-9-]+)*\.(?:internal|corp|lan|intranet|local)\b";

/// The heuristics, compiled once. A pattern that failed to compile is absent, so its heuristic
/// never warns; the test `every_heuristic_compiles` makes that impossible to ship.
struct Heuristics {
    email: Option<Regex>,
    public_domain: Option<Regex>,
    private_host: Option<Regex>,
}

static HEURISTICS: LazyLock<Heuristics> = LazyLock::new(|| Heuristics {
    email: bounded(EMAIL).ok(),
    public_domain: bounded(PUBLIC_DOMAIN).ok(),
    private_host: bounded(PRIVATE_HOST).ok(),
});

/// Whether `text` holds an email address that is not at a public domain.
pub(super) fn has_unlisted_email(text: &str) -> bool {
    let Heuristics {
        email: Some(email),
        public_domain: Some(public),
        ..
    } = &*HEURISTICS
    else {
        return false;
    };
    email
        .find_iter(text)
        .any(|found| !public.is_match(found.as_str()))
}

/// Whether `text` holds a private host name.
pub(super) fn has_private_host(text: &str) -> bool {
    HEURISTICS
        .private_host
        .as_ref()
        .is_some_and(|host| host.is_match(text))
}

#[cfg(test)]
mod tests {
    use super::{HEURISTICS, has_private_host, has_unlisted_email};

    #[test]
    fn every_heuristic_compiles() {
        assert!(HEURISTICS.email.is_some());
        assert!(HEURISTICS.public_domain.is_some());
        assert!(HEURISTICS.private_host.is_some());
    }

    #[test]
    fn an_email_warns_unless_it_is_at_a_public_domain() {
        assert!(has_unlisted_email("write to dev@acme-widgets.io today"));
        assert!(has_unlisted_email("a@example.com b@acme.io"));
        assert!(!has_unlisted_email("a@example.com b@EXAMPLE.ORG"));
        assert!(!has_unlisted_email("1+me@users.noreply.github.com"));
        assert!(!has_unlisted_email("noreply@anthropic.com"));
        assert!(!has_unlisted_email("no address here, @handle or a@b"));
    }

    #[test]
    fn a_host_warns_when_it_ends_in_a_private_suffix() {
        for host in [
            "build.internal",
            "git.corp",
            "nas.lan",
            "wiki.intranet",
            "box.LOCAL",
            "a.b.c.internal:8080",
        ] {
            assert!(has_private_host(&format!("see {host}/x")), "{host}");
        }
        assert!(!has_private_host("localhost and example.com and internal"));
        assert!(!has_private_host("a.internalx"));
    }
}
