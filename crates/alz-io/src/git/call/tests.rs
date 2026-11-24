use std::ffi::{OsStr, OsString};
use std::path::Path;

use alz_core::git::{CloneTransport, GitOperation};
use alz_core::lock::ObjectId;
use alz_core::path::SafeRelPath;
use alz_core::stack::GitRef;

use super::{GitCall, Outcome, ReadScope};

const SHA: &str = "b8b1dadfa8feded6f6d93583dd449585eaafb9f8";

fn args(call: &GitCall<'_>) -> Vec<String> {
    call.arguments()
        .iter()
        .map(|argument| argument.to_string_lossy().into_owned())
        .collect()
}

fn id() -> ObjectId {
    ObjectId::try_from(SHA).unwrap()
}

#[test]
fn a_remote_clone_allows_only_https_and_ssh_and_a_local_one_is_never_a_hard_link_copy() {
    let remote = GitCall::Clone {
        transport: CloneTransport::Remote,
        source: OsStr::new("-oProxyCommand=x"),
        destination: Path::new("/tmp/c"),
    };
    assert_eq!(
        args(&remote),
        [
            "-c",
            "protocol.allow=never",
            "-c",
            "protocol.https.allow=always",
            "-c",
            "protocol.ssh.allow=always",
            "clone",
            "--quiet",
            "--",
            "-oProxyCommand=x",
            "/tmp/c"
        ]
    );
    let local = GitCall::Clone {
        transport: CloneTransport::Local,
        source: OsStr::new("-oProxyCommand=x"),
        destination: Path::new("/tmp/c"),
    };
    assert_eq!(
        args(&local),
        [
            "clone",
            "--quiet",
            "--no-local",
            "--",
            "-oProxyCommand=x",
            "/tmp/c"
        ]
    );
}

#[test]
fn a_revision_follows_end_of_options() {
    let rev = GitRef::try_from("v1.0").unwrap();
    let repository = Path::new("/r");
    let commit = id();
    let path = SafeRelPath::try_from("layers/-x").unwrap();
    let calls = [
        GitCall::RevParseCommit {
            repository,
            scope: ReadScope::CacheEntry,
            rev: &rev,
        },
        GitCall::Checkout {
            repository,
            commit: &commit,
        },
        GitCall::LsTreeAt {
            repository,
            commit: &commit,
            path: &path,
        },
        GitCall::LsTreeRecursive {
            repository,
            tree: &commit,
        },
    ];
    for call in calls {
        let arguments = args(&call);
        let marker = arguments
            .iter()
            .position(|argument| argument == "--end-of-options")
            .unwrap();
        assert!(
            arguments
                .iter()
                .skip(marker + 1)
                .all(|a| !a.starts_with("--")),
            "{arguments:?}"
        );
        assert!(marker + 1 < arguments.len(), "{arguments:?}");
    }
}

#[test]
fn each_call_has_the_arguments_of_the_spec() {
    let rev = GitRef::try_from("main").unwrap();
    let repository = Path::new("/r");
    let commit = id();
    let path = SafeRelPath::try_from("a/b").unwrap();
    let objects = [id()];
    assert_eq!(
        args(&GitCall::RevParseCommit {
            repository,
            scope: ReadScope::PinnedTree,
            rev: &rev
        }),
        [
            "rev-parse",
            "--verify",
            "--quiet",
            "--end-of-options",
            "main^{commit}"
        ]
    );
    assert_eq!(
        args(&GitCall::Checkout {
            repository,
            commit: &commit
        }),
        [
            "-c",
            "advice.detachedHead=false",
            "checkout",
            "--quiet",
            "--end-of-options",
            SHA
        ]
    );
    assert_eq!(
        args(&GitCall::ShowToplevel {
            directory: repository
        }),
        ["rev-parse", "--show-toplevel"]
    );
    assert_eq!(
        args(&GitCall::ShowPrefix {
            directory: repository
        }),
        ["rev-parse", "--show-prefix"]
    );
    assert_eq!(
        args(&GitCall::LsTreeAt {
            repository,
            commit: &commit,
            path: &path
        }),
        ["ls-tree", "-z", "--end-of-options", SHA, "a/b"]
    );
    assert_eq!(
        args(&GitCall::LsTreeRecursive {
            repository,
            tree: &commit
        }),
        ["ls-tree", "-r", "-z", "--end-of-options", SHA]
    );
    assert_eq!(
        args(&GitCall::CatFileBatchCheck {
            repository,
            objects: &objects
        }),
        ["cat-file", "--batch-check"]
    );
    assert_eq!(
        args(&GitCall::CatFileBatch {
            repository,
            objects: &objects
        }),
        ["cat-file", "--batch"]
    );
}

#[test]
fn only_the_batch_calls_have_input() {
    let repository = Path::new("/r");
    let objects = [id(), id()];
    let batch = GitCall::CatFileBatch {
        repository,
        objects: &objects,
    };
    assert_eq!(batch.input(), Some(format!("{SHA}\n{SHA}\n").into_bytes()));
    assert_eq!(
        GitCall::ShowPrefix {
            directory: repository
        }
        .input(),
        None
    );
    assert_eq!(GitCall::Version.input(), None);
}

#[test]
fn the_scope_decides_the_environment_of_a_rev_parse() {
    let rev = GitRef::try_from("main").unwrap();
    let repository = Path::new("/r");
    let operation = |scope| {
        GitCall::RevParseCommit {
            repository,
            scope,
            rev: &rev,
        }
        .operation()
    };
    assert_eq!(operation(ReadScope::CacheEntry), GitOperation::CacheRead);
    assert_eq!(operation(ReadScope::PinnedTree), GitOperation::TreeRead);
}

#[test]
fn only_a_clone_has_the_clone_deadline() {
    let destination = Path::new("/c");
    let clone = GitCall::Clone {
        transport: CloneTransport::Local,
        source: OsStr::new("/s"),
        destination,
    };
    assert!(clone.is_clone());
    assert!(!GitCall::Version.is_clone());
}

#[test]
fn output_is_read_into_typed_values() {
    let rev = GitRef::try_from("main").unwrap();
    let repository = Path::new("/r");
    let commit = GitCall::RevParseCommit {
        repository,
        scope: ReadScope::CacheEntry,
        rev: &rev,
    };
    assert_eq!(
        commit.read(format!("{SHA}\n").as_bytes()),
        Ok(Outcome::Object(Some(id())))
    );
    assert!(commit.read(b"not an id\n").is_err());
    assert_eq!(
        GitCall::ShowPrefix {
            directory: repository
        }
        .read(b"layers/x/\n"),
        Ok(Outcome::Prefix(OsString::from("layers/x/")))
    );
}

#[test]
fn the_tree_of_a_commit_is_asked_after_the_end_of_options_and_with_the_pinned_tree_environment() {
    let commit = id();
    let call = GitCall::RevParseTree {
        repository: Path::new("/r"),
        commit: &commit,
    };
    assert_eq!(
        args(&call),
        [
            "rev-parse",
            "--verify",
            "--quiet",
            "--end-of-options",
            &format!("{SHA}^{{tree}}")
        ]
    );
    assert_eq!(call.operation(), GitOperation::TreeRead);
    assert!(call.status_one_is_no_answer());
}

#[test]
fn the_layer_tags_are_listed_newest_version_first_and_read_line_by_line() {
    let call = GitCall::LayerTags {
        directory: Path::new("/hub/layer"),
    };
    assert_eq!(
        args(&call),
        ["tag", "--list", "layer-v*", "--sort=-v:refname"]
    );
    assert_eq!(call.directory(), Some(Path::new("/hub/layer")));
    assert_eq!(call.operation(), GitOperation::TreeRead);
    assert_eq!(
        call.read(b"layer-v0.10.0\nlayer-v0.9.0\n"),
        Ok(Outcome::Lines(vec![
            OsString::from("layer-v0.10.0"),
            OsString::from("layer-v0.9.0")
        ]))
    );
    assert_eq!(call.read(b""), Ok(Outcome::Lines(vec![])));
}
