use std::ffi::{OsStr, OsString};

use proptest::collection::btree_map;
use proptest::prelude::{Just, Strategy, prop_assert, prop_assert_eq, prop_oneof, proptest};
use proptest::sample::select;

use super::{CloneTransport, GitOperation, child_environment};
use crate::env::EnvVars;

const REPOSITORY_VARIABLES: [&str; 8] = [
    "GIT_DIR",
    "GIT_WORK_TREE",
    "GIT_INDEX_FILE",
    "GIT_OBJECT_DIRECTORY",
    "GIT_ALTERNATE_OBJECT_DIRECTORIES",
    "GIT_COMMON_DIR",
    "GIT_NAMESPACE",
    "GIT_CEILING_DIRECTORIES",
];

const REMOVED_EVERYWHERE: [&str; 10] = [
    "GIT_DIR",
    "GIT_WORK_TREE",
    "GIT_INDEX_FILE",
    "GIT_OBJECT_DIRECTORY",
    "GIT_ALTERNATE_OBJECT_DIRECTORIES",
    "GIT_COMMON_DIR",
    "GIT_NAMESPACE",
    "GIT_CEILING_DIRECTORIES",
    "GIT_CONFIG_PARAMETERS",
    "GIT_CONFIG_COUNT",
];

const REMOTE_CLONE: GitOperation = GitOperation::Clone {
    transport: CloneTransport::Remote,
};
const LOCAL_CLONE: GitOperation = GitOperation::Clone {
    transport: CloneTransport::Local,
};
const OPERATIONS: [GitOperation; 4] = [
    REMOTE_CLONE,
    LOCAL_CLONE,
    GitOperation::CacheRead,
    GitOperation::TreeRead,
];

fn vars(pairs: &[(&str, &str)]) -> EnvVars {
    pairs
        .iter()
        .map(|(name, value)| (OsString::from(name), OsString::from(value)))
        .collect()
}

fn operation() -> impl Strategy<Value = GitOperation> {
    select(OPERATIONS.to_vec())
}

/// Names drawn so that the removed ones, the numbered pairs, near misses and ordinary names all
/// occur often.
fn name() -> impl Strategy<Value = String> {
    prop_oneof![
        select(REMOVED_EVERYWHERE.to_vec()).prop_map(str::to_owned),
        Just("GIT_ALLOW_PROTOCOL".to_owned()),
        Just("GIT_TERMINAL_PROMPT".to_owned()),
        Just("GIT_NO_REPLACE_OBJECTS".to_owned()),
        Just("GIT_NO_LAZY_FETCH".to_owned()),
        Just("GIT_SSH_COMMAND".to_owned()),
        "GIT_CONFIG_(KEY|VALUE)_[0-9]{1,4}",
        "GIT_CONFIG_(KEY|VALUE)_[a-z]{0,3}",
        "GIT_CONFIG_(KEY|VALUE)",
        "git_dir|git_config_count|GIT_DIR_X|XGIT_DIR",
        "[A-Z_]{1,12}",
    ]
}

fn environment() -> impl Strategy<Value = EnvVars> {
    btree_map(name(), "(?s).{0,12}", 0..16).prop_map(|map| {
        map.into_iter()
            .map(|(name, value)| (OsString::from(name), OsString::from(value)))
            .collect()
    })
}

fn is_numbered_config(name: &OsStr) -> bool {
    name.to_str().is_some_and(|name| {
        ["GIT_CONFIG_KEY_", "GIT_CONFIG_VALUE_"]
            .iter()
            .any(|prefix| {
                name.strip_prefix(prefix)
                    .is_some_and(|n| !n.is_empty() && n.chars().all(|c| c.is_ascii_digit()))
            })
    })
}

fn is_set_by(operation: GitOperation, name: &OsStr) -> bool {
    name == "GIT_TERMINAL_PROMPT"
        || (operation == GitOperation::TreeRead
            && (name == "GIT_NO_REPLACE_OBJECTS" || name == "GIT_NO_LAZY_FETCH"))
}

fn is_removed_by_spec(operation: GitOperation, name: &OsStr) -> bool {
    REMOVED_EVERYWHERE.iter().any(|removed| name == *removed)
        || is_numbered_config(name)
        || (operation == REMOTE_CLONE && name == "GIT_ALLOW_PROTOCOL")
}

proptest! {
    #[test]
    fn no_removed_name_survives(operation in operation(), env in environment()) {
        let child = child_environment(operation, &env);
        prop_assert!(child.iter().all(|(name, _)| !is_removed_by_spec(operation, name)));
    }

    #[test]
    fn terminal_prompts_are_always_off(operation in operation(), env in environment()) {
        let child = child_environment(operation, &env);
        prop_assert_eq!(child.get("GIT_TERMINAL_PROMPT"), Some(OsStr::new("0")));
    }

    #[test]
    fn every_other_variable_is_unchanged(operation in operation(), env in environment()) {
        let child = child_environment(operation, &env);
        for (name, value) in env.iter() {
            if !is_removed_by_spec(operation, name) && !is_set_by(operation, name) {
                prop_assert_eq!(child.get(name), Some(value));
            }
        }
        for (name, value) in child.iter() {
            if !is_set_by(operation, name) {
                prop_assert_eq!(env.get(name), Some(value));
            }
        }
    }

    #[test]
    fn applying_the_function_twice_gives_the_same_result(operation in operation(), env in environment()) {
        let once = child_environment(operation, &env);
        prop_assert_eq!(child_environment(operation, &once), once);
    }
}

#[test]
fn the_eight_repository_variables_are_removed_for_every_operation() {
    let env: EnvVars = REPOSITORY_VARIABLES
        .iter()
        .map(|name| (OsString::from(name), OsString::from("/elsewhere")))
        .collect();
    for operation in OPERATIONS {
        let child = child_environment(operation, &env);
        assert!(
            REPOSITORY_VARIABLES
                .iter()
                .all(|name| child.get(name).is_none()),
            "{operation:?}"
        );
    }
}

#[test]
fn the_config_variables_of_a_parent_git_are_removed_for_every_operation() {
    let env = vars(&[
        ("GIT_CONFIG_PARAMETERS", "'core.x=1'"),
        ("GIT_CONFIG_COUNT", "2"),
        ("GIT_CONFIG_KEY_0", "protocol.file.allow"),
        ("GIT_CONFIG_VALUE_0", "always"),
        ("GIT_CONFIG_KEY_10", "k"),
        ("GIT_CONFIG_VALUE_10", "v"),
        ("GIT_CONFIG_GLOBAL", "/home/me/.gitconfig"),
        ("GIT_CONFIG_KEY_", "kept"),
        ("GIT_CONFIG_KEY_x", "kept"),
    ]);
    let removed = [
        "GIT_CONFIG_PARAMETERS",
        "GIT_CONFIG_COUNT",
        "GIT_CONFIG_KEY_0",
        "GIT_CONFIG_VALUE_0",
        "GIT_CONFIG_KEY_10",
        "GIT_CONFIG_VALUE_10",
    ];
    let kept = ["GIT_CONFIG_GLOBAL", "GIT_CONFIG_KEY_", "GIT_CONFIG_KEY_x"];
    for operation in OPERATIONS {
        let child = child_environment(operation, &env);
        assert!(
            removed.iter().all(|name| child.get(name).is_none()),
            "{operation:?}"
        );
        assert!(
            kept.iter().all(|name| child.get(name).is_some()),
            "{operation:?}"
        );
    }
}

#[test]
fn a_remote_clone_has_no_allowed_protocol_list() {
    let env = vars(&[("GIT_ALLOW_PROTOCOL", "file:https")]);
    assert_eq!(
        child_environment(REMOTE_CLONE, &env).get("GIT_ALLOW_PROTOCOL"),
        None
    );
}

#[test]
fn the_other_operations_keep_the_allowed_protocol_list() {
    let env = vars(&[("GIT_ALLOW_PROTOCOL", "file:https")]);
    for operation in [LOCAL_CLONE, GitOperation::CacheRead, GitOperation::TreeRead] {
        assert_eq!(
            child_environment(operation, &env).get("GIT_ALLOW_PROTOCOL"),
            Some(OsStr::new("file:https")),
            "{operation:?}"
        );
    }
}

#[test]
fn every_operation_turns_terminal_prompts_off_over_any_inherited_value() {
    let env = vars(&[("GIT_TERMINAL_PROMPT", "1")]);
    for operation in OPERATIONS {
        assert_eq!(
            child_environment(operation, &env).get("GIT_TERMINAL_PROMPT"),
            Some(OsStr::new("0")),
            "{operation:?}"
        );
    }
}

#[test]
fn only_a_tree_read_ignores_replace_refs_and_lazy_fetch() {
    let env = vars(&[("GIT_NO_LAZY_FETCH", "0")]);
    let tree = child_environment(GitOperation::TreeRead, &env);
    assert_eq!(tree.get("GIT_NO_REPLACE_OBJECTS"), Some(OsStr::new("1")));
    assert_eq!(tree.get("GIT_NO_LAZY_FETCH"), Some(OsStr::new("1")));
    for operation in [REMOTE_CLONE, LOCAL_CLONE, GitOperation::CacheRead] {
        let child = child_environment(operation, &EnvVars::default());
        assert_eq!(child.get("GIT_NO_REPLACE_OBJECTS"), None, "{operation:?}");
        assert_eq!(child.get("GIT_NO_LAZY_FETCH"), None, "{operation:?}");
    }
}

#[test]
fn the_variables_that_keep_ssh_proxies_and_credentials_working_pass_unchanged() {
    let env = vars(&[
        ("HOME", "/home/me"),
        ("PATH", "/usr/bin"),
        ("SSH_AUTH_SOCK", "/run/agent"),
        ("GIT_SSH_COMMAND", "ssh -i key"),
        ("HTTPS_PROXY", "http://proxy"),
        ("GIT_ASKPASS", "/bin/askpass"),
    ]);
    for operation in OPERATIONS {
        let child = child_environment(operation, &env);
        assert!(
            env.iter()
                .all(|(name, value)| child.get(name) == Some(value))
        );
    }
}

#[cfg(unix)]
#[test]
fn a_value_that_is_not_utf8_passes_byte_for_byte() {
    use std::os::unix::ffi::OsStringExt;

    let value = OsString::from_vec(vec![0xFF, b'=', 0x80]);
    let env: EnvVars = [(OsString::from("SSH_ASKPASS"), value.clone())]
        .into_iter()
        .collect();
    assert_eq!(
        child_environment(GitOperation::TreeRead, &env).get("SSH_ASKPASS"),
        Some(value.as_os_str())
    );
}
