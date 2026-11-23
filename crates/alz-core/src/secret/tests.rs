use super::{SHAPES, SecretKind, scan};

/// A token of `kind`, assembled at run time so that no file of the repository holds a string that
/// looks like a token.
fn sample(kind: SecretKind) -> String {
    match kind {
        SecretKind::GitHubToken => format!("{}_{}", "ghp", "aB3".repeat(13)),
        SecretKind::AnthropicApiKey => format!("{}-{}", "sk-ant", "x1_".repeat(8)),
        SecretKind::OpenAiStyleApiKey => format!("{}-{}", "sk-proj", "Ab1".repeat(12)),
        SecretKind::AwsAccessKeyId => format!("{}{}", "AKIA", "Q7".repeat(8)),
        SecretKind::GoogleApiKey => format!("{}{}{}", "AIza", "Zz9_".repeat(8), "Zz9"),
        SecretKind::SlackToken => format!("{}-{}", "xoxb", "1234567890"),
        SecretKind::StripeKey => format!("{}_{}", "sk_live", "a1".repeat(10)),
        SecretKind::PrivateKey => format!("{}{}{}", "-----BEGIN ", "RSA PRIVATE KEY", "-----"),
        SecretKind::CredentialAssignment => format!("{}: {}", "api_key", "k9".repeat(12)),
    }
}

#[test]
fn every_token_shape_compiles() {
    for kind in SecretKind::ALL {
        assert!(kind.compile().is_some(), "{kind}");
    }
    assert_eq!(SHAPES.len(), SecretKind::ALL.len());
}

#[test]
fn each_shape_is_found_by_its_own_kind() {
    for kind in SecretKind::ALL {
        assert!(scan(&sample(kind)).contains(&kind), "{kind}");
    }
}

#[test]
fn a_github_token_is_found_in_each_of_its_prefixes() {
    for prefix in ["ghp", "gho", "ghu", "ghs", "ghr"] {
        let token = format!("{prefix}_{}", "a".repeat(36));
        assert_eq!(
            scan(&format!("use {token} here")),
            [SecretKind::GitHubToken]
        );
    }
    let fine_grained = format!("{}_{}", "github_pat", "a".repeat(50));
    assert_eq!(scan(&fine_grained), [SecretKind::GitHubToken]);
}

#[test]
fn text_without_a_token_has_no_finding() {
    for text in [
        "",
        "https://mcp.context7.com/mcp",
        "npx",
        "@playwright/mcp@latest",
        "ghp_short",
        "sk-short",
        "the token is stored elsewhere",
        "password: short",
        "AKIA0123",
    ] {
        assert_eq!(scan(text), Vec::<SecretKind>::new(), "{text}");
    }
}

#[test]
fn a_shape_needs_a_word_boundary_where_the_reference_has_one() {
    let aws = sample(SecretKind::AwsAccessKeyId);
    assert_eq!(scan(&format!("x{aws}")), Vec::<SecretKind>::new());
    assert_eq!(scan(&format!("{aws}x")), Vec::<SecretKind>::new());
    assert_eq!(scan(&format!("[{aws}]")), [SecretKind::AwsAccessKeyId]);
}

#[test]
fn a_credential_assignment_is_matched_ignoring_case_and_across_unicode_space() {
    let value = "k9".repeat(12);
    for text in [
        format!("API-KEY = \"{value}\""),
        format!("Password:{value}"),
        format!("secret\u{a0}={value}"),
        format!("\"token\": '{value}'"),
    ] {
        assert!(
            scan(&text).contains(&SecretKind::CredentialAssignment),
            "{text}"
        );
    }
}

#[test]
fn a_kind_is_reported_once_however_often_it_occurs() {
    let token = sample(SecretKind::AwsAccessKeyId);
    assert_eq!(
        scan(&format!("{token} and {token}")),
        [SecretKind::AwsAccessKeyId]
    );
}

#[test]
fn a_finding_holds_the_kind_and_never_the_text() {
    let token = sample(SecretKind::GitHubToken);
    let named: Vec<String> = scan(&token).iter().map(ToString::to_string).collect();
    assert_eq!(named, ["GitHub token"]);
    assert!(named.iter().all(|name| !name.contains(&token)));
}

#[test]
fn a_long_text_is_scanned_in_bounded_time() {
    let text = "a".repeat(2_000_000);
    assert_eq!(scan(&text), Vec::<SecretKind>::new());
}
