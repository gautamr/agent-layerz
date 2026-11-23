//! Tests of the source forms of spec §5.2: each parses to its variant and prints back byte for
//! byte, and credentials are refused as §5.2 states without the error holding the source.

use proptest::prelude::{prop_assert_eq, proptest};

use super::{GitHubName, GitUrl, Source, SourceDir, SourceError};
use crate::stack::git_ref::{GitRef, GitRefError};

fn parsed(text: &str) -> Source {
    Source::try_from(text).unwrap_or_else(|error| panic!("{text:?} is refused: {error}"))
}

fn git_ref(text: &str) -> GitRef {
    GitRef::try_from(text).expect("a ref")
}

#[test]
fn every_source_form_parses_to_its_variant() {
    assert_eq!(
        parsed("path:../hub/layer"),
        Source::Path {
            dir: SourceDir("../hub/layer".to_owned())
        }
    );
    assert_eq!(
        parsed("github:me/agent-layerz-personal@v1.4.0"),
        Source::GitHub {
            owner: GitHubName("me".to_owned()),
            repo: GitHubName("agent-layerz-personal".to_owned()),
            git_ref: git_ref("v1.4.0"),
        }
    );
    assert_eq!(
        parsed("git:https://example.com/o/r.git@v1"),
        Source::Git {
            url: GitUrl("https://example.com/o/r.git".to_owned()),
            git_ref: git_ref("v1"),
        }
    );
}

#[test]
fn every_source_prints_back_byte_for_byte_and_parses_again_to_the_same_value() {
    let texts = [
        "path:./.layer",
        "path:/abs/dir",
        "path:with space/ü",
        "github:o/r@main",
        "github:o.x/r_y-z@feature/a+b",
        "git:https://example.com/o/r.git@v1",
        "git:ssh://git@example.com/o/r.git@v1",
        "git:git@github.com:o/r.git@v1.0",
        "git:/local/repo@abc123",
        "git:../sibling@v2",
        "git:file:///srv/repo@v1",
        "git:ssh://git@host:2222/o/r.git@v1",
    ];
    for text in texts {
        let source = parsed(text);
        assert_eq!(source.to_string(), text);
        assert_eq!(parsed(&source.to_string()), source);
    }
}

#[test]
fn a_user_name_alone_is_accepted_in_ssh_and_scp_like_urls() {
    for text in [
        "git:ssh://git@host/x.git@v1",
        "git:git@github.com:o/x.git@v1",
        "git:ssh://git@host:22/x.git@v1",
        "git:https://host/path@part/x.git@v1",
        "git:ssh://host/x.git@v1",
    ] {
        assert!(Source::try_from(text).is_ok(), "{text}");
    }
}

#[test]
fn an_at_that_starts_the_ref_is_not_the_user_of_a_scp_like_address() {
    for text in [
        "git:git@host:repo.git@v1",
        "git:host.invalid:client.git@v1",
        "git:x:y@v1",
        "git:git@host:o/r.git@feature/x",
    ] {
        assert!(Source::try_from(text).is_ok(), "{text}");
    }
}

#[test]
fn a_password_in_a_scp_like_address_is_refused_whatever_follows_it() {
    for text in [
        "git:me:TOKEN@github.com/o/r",
        "git:user:tok@host:repo@v1",
        "git:user:tok@host:repo",
        "git:user:tok@host@v1",
    ] {
        assert_eq!(
            Source::try_from(text),
            Err(SourceError::Credentials),
            "{text}"
        );
    }
    assert_eq!(
        GitUrl::try_from("user:tok@host"),
        Err(SourceError::Credentials)
    );
}

#[test]
fn a_transport_helper_url_is_refused_before_anything_else() {
    for text in [
        "git:ext::sh -c touch% /tmp/x@v1",
        "git:ext::sh@v1",
        "git:fd::17@v1",
        "git:https::https://TOKEN@host/x.git@v1",
        "git:https::https://host/x.git@v1",
        "git:a+b-c.d::x@v1",
        "git:::x@v1",
        "git:ext::sh -c x",
    ] {
        assert_eq!(
            Source::try_from(text),
            Err(SourceError::TransportHelper),
            "{text}"
        );
    }
    assert_eq!(
        GitUrl::try_from("ext::sh"),
        Err(SourceError::TransportHelper)
    );
}

#[test]
fn a_colon_that_is_not_a_double_colon_at_the_start_is_not_a_transport_helper() {
    for text in [
        "git:host:o/x.git@v1",
        "git:./a::b/x.git@v1",
        "git:https://host/a::b.git@v1",
        "git:git@host:o::x@v1",
    ] {
        assert!(Source::try_from(text).is_ok(), "{text}");
    }
}

#[test]
fn credentials_in_a_git_url_are_refused() {
    for text in [
        "git:https://user:token@host/x.git@v1",
        "git:https://token@host/x.git@v1",
        "git:https://@host/x.git@v1",
        "git:HTTPS://token@host/x.git@v1",
        "git:http://user@host/x.git@v1",
        "git:https://host@evil/x.git@v1",
        "git:https://token@host?q@v1",
        "git:ssh://git:secret@host/x.git@v1",
        "git:ftp://u:p@host/x.git@v1",
        "git:user:secret@host:o/x.git@v1",
        // No ref: the last `@` is the credentials' own and must not hide them.
        "git:https://user:FAKETOKEN@github.com/org/repo.git",
        "git:https://FAKETOKEN@github.com/org/repo",
        "git:https://@host/x.git",
        "git:ssh://git:secret@host/x.git",
        "git:user:secret@host:o/x.git",
    ] {
        assert_eq!(
            Source::try_from(text),
            Err(SourceError::Credentials),
            "{text}"
        );
    }
}

#[test]
fn a_refused_source_error_never_holds_the_source() {
    let token = "ghp_secret_token";
    let error = Source::try_from(format!("git:https://user:{token}@host/x.git@v1").as_str())
        .expect_err("credentials");
    assert!(!format!("{error:?} {error} {}", error.rule()).contains(token));
}

#[test]
fn other_malformed_sources_are_refused_with_their_own_reason() {
    let cases = [
        ("", SourceError::UnknownScheme),
        ("./.layer", SourceError::UnknownScheme),
        ("Path:x", SourceError::UnknownScheme),
        ("file:x", SourceError::UnknownScheme),
        ("path:", SourceError::EmptyPath),
        ("github:o/r", SourceError::MissingRef),
        ("github:o/r@", SourceError::MissingRef),
        ("github:@v1", SourceError::MissingRef),
        ("git:git@host:o/r.git", SourceError::MissingRef),
        (
            "github:owner/repo@--upload-pack=touch",
            SourceError::InvalidRef(GitRefError),
        ),
        ("git:https://h/r@a..b", SourceError::InvalidRef(GitRefError)),
        ("git:-oProxy=x@v1", SourceError::UrlStartsWithDash),
        ("github:o@v1", SourceError::GitHubShape),
        ("github:o/r/x@v1", SourceError::GitHubShape),
        ("github:/r@v1", SourceError::GitHubShape),
        ("github:../r@v1", SourceError::GitHubShape),
        ("github:-o/r@v1", SourceError::GitHubShape),
    ];
    for (text, expected) in cases {
        assert_eq!(Source::try_from(text), Err(expected), "{text:?}");
    }
}

proptest! {
    #[test]
    fn a_valid_source_prints_back_to_its_input(
        kind in 0..3_u8,
        owner in "[a-zA-Z0-9][a-zA-Z0-9._-]{0,8}",
        repo in "[a-zA-Z0-9][a-zA-Z0-9._-]{0,8}",
        host in "[a-z]{1,8}",
        dir in "[a-zA-Z0-9 ./_-]{1,12}",
        git_ref in "[A-Za-z0-9_+][A-Za-z0-9_+/-]{0,8}",
    ) {
        let text = match kind {
            0 => format!("path:{dir}"),
            1 => format!("github:{owner}/{repo}@{git_ref}"),
            _ => format!("git:ssh://git@{host}/{owner}/{repo}.git@{git_ref}"),
        };
        let source = Source::try_from(text.as_str()).expect("a valid source");
        prop_assert_eq!(source.to_string(), text);
        prop_assert_eq!(Source::try_from(source.to_string().as_str()), Ok(source));
    }
}
