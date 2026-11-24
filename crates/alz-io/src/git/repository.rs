//! Git calls on the repository being built (spec §9.4): reading the identity git would use and
//! adding the declared one to `.git/config`. Every call is a value with an argument array, no
//! shell is involved, a key is one of a closed set and a value is a validated name or email, so
//! no caller writes an argument. git's answers are reduced to typed values of `alz-core` as they
//! arrive; the value of a config key is never kept.

use std::ffi::OsString;
use std::path::Path;

use alz_core::git::RepositoryMessages;
use alz_core::identity::{
    Blocker, CommitIdentity, ConfigAnswer, ConfigKey, Effective, Field, GitConfigPlace, KeyState,
    RepoState, Role, SetupDecision, SetupPlan, Verdicts,
};

use super::runner::GitRunner;
use crate::fs::{Changed, RepoRoot};

/// One call. The directory is the repository root, and the file `.git/config` below it.
#[derive(Debug, Clone, Copy)]
pub(super) enum RepositoryCall<'a> {
    /// `git rev-parse --git-dir`, in the C locale because its message is read.
    GitDir { root: &'a Path },
    /// `git rev-parse --show-toplevel`.
    ShowToplevel { root: &'a Path },
    /// `git var GIT_<ROLE>_IDENT`.
    Var { root: &'a Path, role: Role },
    /// `git config --show-scope --show-origin --get --end-of-options <key>`.
    Origin { root: &'a Path, key: ConfigKey },
    /// `git config --file <root>/.git/config --get --end-of-options <key>`.
    FileGet { root: &'a Path, field: Field },
    /// `git config --file <root>/.git/config --end-of-options <key> <value>`.
    FileSet {
        root: &'a Path,
        field: Field,
        declared: &'a CommitIdentity,
    },
}

fn os(text: &str) -> OsString {
    OsString::from(text)
}

fn config_file(root: &Path) -> OsString {
    root.join(".git").join("config").into_os_string()
}

impl RepositoryCall<'_> {
    pub(super) const fn messages(&self) -> RepositoryMessages {
        match self {
            Self::GitDir { .. } => RepositoryMessages::ReadByAlz,
            Self::ShowToplevel { .. }
            | Self::Var { .. }
            | Self::Origin { .. }
            | Self::FileGet { .. }
            | Self::FileSet { .. } => RepositoryMessages::Ignored,
        }
    }

    pub(super) fn directory(&self) -> &Path {
        match self {
            Self::GitDir { root }
            | Self::ShowToplevel { root }
            | Self::Var { root, .. }
            | Self::Origin { root, .. }
            | Self::FileGet { root, .. }
            | Self::FileSet { root, .. } => root,
        }
    }

    pub(super) fn arguments(&self) -> Vec<OsString> {
        match self {
            Self::GitDir { .. } => vec![os("rev-parse"), os("--git-dir")],
            Self::ShowToplevel { .. } => vec![os("rev-parse"), os("--show-toplevel")],
            Self::Var { role, .. } => vec![
                os("var"),
                os(match role {
                    Role::Author => "GIT_AUTHOR_IDENT",
                    Role::Committer => "GIT_COMMITTER_IDENT",
                }),
            ],
            Self::Origin { key, .. } => vec![
                os("config"),
                os("--show-scope"),
                os("--show-origin"),
                os("--get"),
                os("--end-of-options"),
                os(key.as_str()),
            ],
            Self::FileGet { root, field } => vec![
                os("config"),
                os("--file"),
                config_file(root),
                os("--get"),
                os("--end-of-options"),
                os(field.user_key().as_str()),
            ],
            Self::FileSet {
                root,
                field,
                declared,
            } => vec![
                os("config"),
                os("--file"),
                config_file(root),
                os("--end-of-options"),
                os(field.user_key().as_str()),
                os(declared_value(declared, *field)),
            ],
        }
    }
}

/// The declared value of `field`.
fn declared_value(declared: &CommitIdentity, field: Field) -> &str {
    match field {
        Field::Name => declared.name().as_str(),
        Field::Email => declared.email().as_str(),
    }
}

/// What a call gave: git's exit status, `None` when it did not run or did not finish, and its
/// output. Untrusted bytes, reduced by the callers and never printed from here.
#[derive(Debug)]
pub(super) struct Answer {
    pub(super) code: Option<i32>,
    pub(super) stdout: Vec<u8>,
    pub(super) stderr: Vec<u8>,
}

impl Answer {
    pub(super) const fn not_run() -> Self {
        Self {
            code: None,
            stdout: Vec::new(),
            stderr: Vec::new(),
        }
    }
}

/// How one key write ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyWrite {
    /// git wrote the key and the check after it found nothing changed.
    Written,
    /// git failed or did not run.
    Failed,
}

/// Why the writes stopped before the plan was done.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stopped {
    /// The check right before a write found the place no longer writable: nothing was written for
    /// it and nothing more is written.
    Refused(Blocker),
    /// The check right after the write of `field` found `.git` or `.git/config` changed: git may
    /// have written through a link. Nothing more is written.
    Tampered {
        /// The key whose write the check follows.
        field: Field,
        /// What changed.
        changed: Changed,
    },
}

/// What setting the identity did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SetupResult {
    /// Nothing was read or written: the place is blocked.
    Blocked(Blocker),
    /// The place was writable.
    Planned {
        /// What `.git/config` held, as the plan.
        plan: SetupPlan,
        /// The keys alz tried to write, in order, with how each ended.
        written: Vec<(Field, KeyWrite)>,
        /// Why the writes stopped early, when they did.
        stopped: Option<Stopped>,
    },
}

/// The git calls on one repository root, the only ones the identity step makes. It holds the
/// runner, which runs git, and the root, which looks at `.git` and `.git/config` without
/// following a link.
#[derive(Debug, Clone, Copy)]
pub struct RepositoryGit<'a> {
    runner: &'a GitRunner,
    root: &'a RepoRoot,
    directory: &'a Path,
}

impl<'a> RepositoryGit<'a> {
    /// The calls on the repository at `directory`, which `root` is the capability of.
    #[must_use]
    pub const fn new(runner: &'a GitRunner, root: &'a RepoRoot, directory: &'a Path) -> Self {
        Self {
            runner,
            root,
            directory,
        }
    }

    /// Whether `directory` is a git repository, from `git rev-parse --git-dir`.
    #[must_use]
    pub fn state(&self) -> RepoState {
        let answer = self.run(&RepositoryCall::GitDir {
            root: self.directory,
        });
        RepoState::classify(answer.code, &answer.stderr)
    }

    /// Compares the identity git would use for a commit with `declared`, for author and
    /// committer. A failed `git var` differs.
    #[must_use]
    pub fn verdicts(&self, declared: &CommitIdentity) -> Verdicts {
        let verdict = |role| {
            let answer = self.run(&RepositoryCall::Var {
                root: self.directory,
                role,
            });
            let output = (answer.code == Some(0)).then_some(answer.stdout.as_slice());
            Effective::compare(output, declared)
        };
        Verdicts {
            author: verdict(Role::Author),
            committer: verdict(Role::Committer),
        }
    }

    /// Where git reads `key` from: scope and origin, without the value.
    #[must_use]
    pub fn config_answer(&self, key: ConfigKey) -> ConfigAnswer {
        let answer = self.run(&RepositoryCall::Origin {
            root: self.directory,
            key,
        });
        ConfigAnswer::from_git(answer.code, &answer.stdout)
    }

    /// Whether `.git/config` may be written: the probes of the root, and git's own answer that
    /// the root is the top level of its work tree.
    #[must_use]
    pub fn place(&self) -> GitConfigPlace {
        match self.root.git_config_place() {
            GitConfigPlace::Blocked(blocker) => GitConfigPlace::Blocked(blocker),
            GitConfigPlace::Writable => self.top_level_place(),
        }
    }

    fn top_level_place(&self) -> GitConfigPlace {
        let answer = self.run(&RepositoryCall::ShowToplevel {
            root: self.directory,
        });
        let line = answer.stdout.strip_suffix(b"\n").unwrap_or(&answer.stdout);
        let blocker = match (answer.code, std::str::from_utf8(line)) {
            (Some(0), Ok(top)) => match self.root.is_this_folder(Path::new(top)) {
                Ok(true) => return GitConfigPlace::Writable,
                Ok(false) => Blocker::NotTopLevel,
                Err(_) => Blocker::Uninspectable,
            },
            (Some(_) | None, Ok(_) | Err(_)) => Blocker::Uninspectable,
        };
        GitConfigPlace::Blocked(blocker)
    }

    /// What `.git/config` holds for `user.name` and `user.email`, with the values dropped.
    #[must_use]
    pub fn key_states(&self, declared: &CommitIdentity) -> (KeyState, KeyState) {
        let state = |field| {
            let answer = self.run(&RepositoryCall::FileGet {
                root: self.directory,
                field,
            });
            KeyState::from_get(answer.code, &answer.stdout, declared_value(declared, field))
        };
        (state(Field::Name), state(Field::Email))
    }

    /// Sets the declared identity in `.git/config` (spec §9.4 "Writing"): when the place is
    /// writable, each key `.git/config` has no value for is added, the place is checked right
    /// before and right after each write, and a failed check stops everything. Never
    /// `--global` or `--system`, and a different value is never replaced.
    #[must_use]
    pub fn set_up(&self, declared: &CommitIdentity) -> SetupResult {
        match SetupDecision::decide(self.place(), || self.key_states(declared)) {
            SetupDecision::Blocked(blocker) => SetupResult::Blocked(blocker),
            SetupDecision::Write(plan) => {
                let (written, stopped) = self.write_additions(plan, declared);
                SetupResult::Planned {
                    plan,
                    written,
                    stopped,
                }
            }
        }
    }

    /// The writes are effects, so they are a loop that ends at the first failed check.
    fn write_additions(
        &self,
        plan: SetupPlan,
        declared: &CommitIdentity,
    ) -> (Vec<(Field, KeyWrite)>, Option<Stopped>) {
        let mut written = Vec::new();
        for field in plan.additions() {
            let seal = match self.root.seal_git_config() {
                Ok(seal) => seal,
                Err(blocker) => return (written, Some(Stopped::Refused(blocker))),
            };
            let answer = self.run(&RepositoryCall::FileSet {
                root: self.directory,
                field,
                declared,
            });
            if let Err(changed) = self.root.verify_git_config(seal) {
                return (written, Some(Stopped::Tampered { field, changed }));
            }
            written.push((
                field,
                if answer.code == Some(0) {
                    KeyWrite::Written
                } else {
                    KeyWrite::Failed
                },
            ));
        }
        (written, None)
    }

    fn run(&self, call: &RepositoryCall<'_>) -> Answer {
        self.runner.run_repository(call)
    }
}
