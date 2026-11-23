//! The environment of a git child process (spec §6 "Git subprocesses", decision-24, TASK-63).

use std::ffi::OsStr;

use crate::env::EnvVars;

/// Variables that point git at another repository or object store.
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

/// Variables that carry the `-c` options of a parent git, apart from the numbered pairs.
const CONFIG_VARIABLES: [&str; 2] = ["GIT_CONFIG_PARAMETERS", "GIT_CONFIG_COUNT"];

/// The names that begin the numbered pairs `GIT_CONFIG_KEY_<n>` and `GIT_CONFIG_VALUE_<n>`.
const NUMBERED_CONFIG_PREFIXES: [&str; 2] = ["GIT_CONFIG_KEY_", "GIT_CONFIG_VALUE_"];

/// The variable that restricts which transports git may use, which a remote clone must not
/// inherit (spec §7.12).
const ALLOW_PROTOCOL: &str = "GIT_ALLOW_PROTOCOL";

/// How a clone reaches its source.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CloneTransport {
    /// A network transport (`https:`, `ssh:`, a `github:` source): the clone is guarded, so it
    /// is given no `GIT_ALLOW_PROTOCOL`.
    Remote,
    /// A path or `file:` URL on this machine.
    Local,
}

/// A git call that resolves a source. Calls on the repository being built (§9.4) are not
/// among these: their environment is that section's.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GitOperation {
    /// `git clone` of a `git:` or `github:` source.
    Clone {
        /// How the source is reached.
        transport: CloneTransport,
    },
    /// `rev-parse` and `checkout` in a temporary clone or a cache entry.
    CacheRead,
    /// Every call that reads a pinned layer's tree: `rev-parse`, `ls-tree` and `cat-file`.
    TreeRead,
}

/// Whether `name` is `GIT_CONFIG_KEY_<n>` or `GIT_CONFIG_VALUE_<n>`, with `n` one or more ASCII
/// digits.
fn is_numbered_config(name: &str) -> bool {
    NUMBERED_CONFIG_PREFIXES.iter().any(|prefix| {
        name.strip_prefix(prefix).is_some_and(|number| {
            !number.is_empty() && number.bytes().all(|byte| byte.is_ascii_digit())
        })
    })
}

/// Whether the child of `operation` must not inherit the variable `name`. A name that is not
/// UTF-8 matches none of them, because every one is ASCII.
fn is_removed(operation: GitOperation, name: &OsStr) -> bool {
    name.to_str().is_some_and(|name| {
        REPOSITORY_VARIABLES.contains(&name)
            || CONFIG_VARIABLES.contains(&name)
            || is_numbered_config(name)
            || (name == ALLOW_PROTOCOL
                && matches!(
                    operation,
                    GitOperation::Clone {
                        transport: CloneTransport::Remote
                    }
                ))
    })
}

/// The variables `operation` sets, whatever the parent had.
const fn assignments(operation: GitOperation) -> &'static [(&'static str, &'static str)] {
    match operation {
        GitOperation::Clone { transport: _ } | GitOperation::CacheRead => {
            &[("GIT_TERMINAL_PROMPT", "0")]
        }
        GitOperation::TreeRead => &[
            ("GIT_TERMINAL_PROMPT", "0"),
            ("GIT_NO_REPLACE_OBJECTS", "1"),
            ("GIT_NO_LAZY_FETCH", "1"),
        ],
    }
}

/// The whole environment of the git child that carries out `operation`, given alz's own.
///
/// - Removed, for every operation: the eight variables that point git at another repository
///   or object store, `GIT_CONFIG_PARAMETERS`, `GIT_CONFIG_COUNT` and every
///   `GIT_CONFIG_KEY_<n>` and `GIT_CONFIG_VALUE_<n>`. A git hook receives some of them, so
///   without this a call from a hook would read or write the user's index or repository.
/// - Removed, for a remote clone: `GIT_ALLOW_PROTOCOL`.
/// - Set: `GIT_TERMINAL_PROMPT=0` always; `GIT_NO_REPLACE_OBJECTS=1` and `GIT_NO_LAZY_FETCH=1`
///   for a tree read.
/// - Every other variable passes unchanged, so agents, proxies, `GIT_SSH_COMMAND` and
///   credential configuration keep working.
///
/// The result is a function of `operation` and `env` only, and applying it to its own result
/// gives that result.
#[must_use]
pub fn child_environment(operation: GitOperation, env: &EnvVars) -> EnvVars {
    assignments(operation).iter().fold(
        env.clone().keeping(|name| !is_removed(operation, name)),
        |vars, (name, value)| vars.with(*name, *value),
    )
}

#[cfg(test)]
mod tests;
