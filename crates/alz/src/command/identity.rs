//! The commit identity step of `init`, `build` and `check` (spec §9.4, §10.3).
//!
//! `alz-core` decides what is wrong and words every line; `alz-io`'s [`RepositoryGit`] runs the
//! git calls. This module joins them: it asks git, turns the answers into the lines of the report
//! and, for the setup commands, the keys that were written. Without a declared identity nothing
//! here runs, so no git command is started and no line is printed.

use alz_core::identity::{
    Command as IdentityCommand, DeclaredIdentity, Field, Finding, Home, IdentityCheck, KeyAction,
    Presence, RepoState, Report, Slot, locate,
};
use alz_core::path::AbsolutePath;
use alz_core::stack::ResolvedLayer;
use alz_core::{Code, Diagnostic, Diagnostics, Message};
use alz_io::{
    Changed, Deadlines, EnvSnapshot, GitRunner, KeyWrite, RepoRoot, RepositoryGit, SetupResult,
    Stopped,
};

use crate::command::pipeline::Computed;

/// What a setup printed: the line for each key written, which goes to standard output, and every
/// other line, which goes to standard error.
#[derive(Debug, Default)]
pub struct Outcome {
    pub stdout: String,
    pub diagnostics: Diagnostics,
}

/// A line of a setup.
enum Line {
    Written(Message),
    Problem(Diagnostic),
}

/// The identity step of one run: the declared identity and what reaches git for it.
#[derive(Debug)]
pub struct Step<'a> {
    declared: DeclaredIdentity,
    home: Option<Home>,
    env: &'a EnvSnapshot,
    root: &'a RepoRoot,
    directory: &'a AbsolutePath,
}

/// The declared identity of a merged build: the winning declarer and the source it was read from.
/// `None` when no layer declares one, or the declarations conflict.
pub fn declared_of(computed: &Computed) -> Option<DeclaredIdentity> {
    DeclaredIdentity::from_winner(computed.merged.commit_identity().declared(), |layer| {
        computed
            .stack
            .layers()
            .iter()
            .find(|resolved| resolved.meta().name() == layer)
            .map(ResolvedLayer::source)
    })
}

impl<'a> Step<'a> {
    /// The step for `declared`, in the repository at `directory` which `root` is the capability of.
    pub fn new(
        declared: DeclaredIdentity,
        env: &'a EnvSnapshot,
        root: &'a RepoRoot,
        directory: &'a AbsolutePath,
    ) -> Self {
        Self {
            declared,
            home: env.home().and_then(|home| home.to_str()).map(Home::new),
            env,
            root,
            directory,
        }
    }

    fn report(&self) -> Report<'_> {
        Report::new(&self.declared, self.home.as_ref())
    }

    /// The lines of `alz build` and `alz check`: what differs, and nothing is written.
    pub fn check(&self, command: IdentityCommand) -> Diagnostics {
        let check = self.runner().map_or(IdentityCheck::Unavailable, |runner| {
            self.read(&self.git(&runner))
        });
        self.report()
            .check_lines(command, &check, true)
            .into_iter()
            .collect()
    }

    /// The step of `alz init` and `alz build --adopt` (§9.4 "Writing"): the missing keys of
    /// `.git/config` are written, then the identity git would use is read again.
    pub fn set_up(&self, command: IdentityCommand) -> Outcome {
        let Some(runner) = self.runner() else {
            return self.lines_of(command, &IdentityCheck::Unavailable, |_| true, true);
        };
        let git = self.git(&runner);
        match git.state() {
            RepoState::NotARepository => {
                self.lines_of(command, &IdentityCheck::NotARepository, |_| true, true)
            }
            RepoState::Unavailable => {
                self.lines_of(command, &IdentityCheck::Unavailable, |_| true, true)
            }
            RepoState::Repository => {
                self.write_and_report(command, &git, git.set_up(self.declared.identity()))
            }
        }
    }

    fn write_and_report(
        &self,
        command: IdentityCommand,
        git: &RepositoryGit<'_>,
        result: SetupResult,
    ) -> Outcome {
        let report = self.report();
        match result {
            SetupResult::Blocked(blocker) => {
                let refusal = Outcome {
                    stdout: String::new(),
                    diagnostics: Diagnostics::new().with(report.blocked_line(blocker)),
                };
                self.with_findings(
                    refusal,
                    command,
                    git,
                    |_| true,
                    !blocker.would_reach_another_repository(),
                )
            }
            SetupResult::Planned {
                plan,
                written,
                stopped,
            } => {
                let lines = plan
                    .actions()
                    .into_iter()
                    .filter_map(|(field, action)| self.key_line(field, action, &written))
                    .fold(Outcome::default(), |outcome, line| match line {
                        Line::Written(message) => Outcome {
                            stdout: format!("{}{message}\n", outcome.stdout),
                            ..outcome
                        },
                        Line::Problem(problem) => Outcome {
                            diagnostics: outcome.diagnostics.with(problem),
                            ..outcome
                        },
                    });
                match stopped {
                    Some(Stopped::Tampered { field, changed }) => Outcome {
                        diagnostics: lines.diagnostics.with(tampered_line(field, changed)),
                        ..lines
                    },
                    Some(Stopped::Refused(blocker)) => {
                        let lines = Outcome {
                            diagnostics: lines.diagnostics.with(report.blocked_line(blocker)),
                            ..lines
                        };
                        self.with_findings(
                            lines,
                            command,
                            git,
                            |_| true,
                            !blocker.would_reach_another_repository(),
                        )
                    }
                    None => {
                        let kept: Vec<Field> = plan.kept().collect();
                        self.with_findings(
                            lines,
                            command,
                            git,
                            move |finding| {
                                !kept.iter().any(|field| finding.is_local_user_key(*field))
                            },
                            true,
                        )
                    }
                }
            }
        }
    }

    /// The line for one key of the plan, `None` when there is nothing to say about it.
    fn key_line(
        &self,
        field: Field,
        action: KeyAction,
        written: &[(Field, KeyWrite)],
    ) -> Option<Line> {
        match action {
            KeyAction::Add => written
                .iter()
                .find(|(written_field, _)| *written_field == field)
                .map(|(_, how)| match how {
                    KeyWrite::Written => Line::Written(self.report().written_line(field)),
                    KeyWrite::Failed => Line::Problem(Report::write_failed_line(field)),
                }),
            KeyAction::LeaveEqual => None,
            KeyAction::ReportDifferent => Some(Line::Problem(self.report().kept_line(field))),
            KeyAction::ReportUnreadable => Some(Line::Problem(Report::unreadable_line(field))),
        }
    }

    /// `before` followed by what git would use now, less the findings `keep` refuses.
    fn with_findings(
        &self,
        before: Outcome,
        command: IdentityCommand,
        git: &RepositoryGit<'_>,
        keep: impl Fn(&Finding) -> bool,
        with_fix: bool,
    ) -> Outcome {
        let after = self.lines_of(command, &self.read(git), keep, with_fix);
        Outcome {
            stdout: before.stdout,
            diagnostics: before.diagnostics.merge(after.diagnostics),
        }
    }

    /// The report lines of `check`, without the findings `keep` refuses.
    fn lines_of(
        &self,
        command: IdentityCommand,
        check: &IdentityCheck,
        keep: impl Fn(&Finding) -> bool,
        with_fix: bool,
    ) -> Outcome {
        let report = self.report();
        let lines = match check {
            IdentityCheck::Differs(findings) => match findings.clone().retain(keep) {
                Some(kept) => report.finding_lines(command.level(), &kept, with_fix),
                None => Vec::new(),
            },
            IdentityCheck::Matches | IdentityCheck::NotARepository | IdentityCheck::Unavailable => {
                report.check_lines(command, check, with_fix)
            }
        };
        Outcome {
            stdout: String::new(),
            diagnostics: lines.into_iter().collect(),
        }
    }

    /// What git would use for a commit here, compared with the declaration.
    fn read(&self, git: &RepositoryGit<'_>) -> IdentityCheck {
        let presence = self.presence();
        IdentityCheck::decide(
            git.state(),
            || git.verdicts(self.declared.identity()),
            |slot| locate(slot, &presence, |key| git.config_answer(key)),
        )
    }

    /// Which variables git reads an identity from are set, by presence only.
    fn presence(&self) -> Presence {
        let set = |name: &str| self.env.vars().get(name).is_some();
        Presence::new(
            Slot::all().into_iter().filter(|slot| set(slot.variable())),
            set("EMAIL"),
        )
    }

    fn runner(&self) -> Option<GitRunner> {
        GitRunner::new(self.env, Deadlines::STANDARD).ok()
    }

    fn git<'r>(&'r self, runner: &'r GitRunner) -> RepositoryGit<'r> {
        RepositoryGit::new(
            runner,
            self.root,
            std::path::Path::new(self.directory.as_str()),
        )
    }
}

/// The error of a check after a write that found the place changed (§9.4 "Writing"): git may have
/// written through a link, so nothing more is written and the command fails. No value is named.
fn tampered_line(field: Field, changed: Changed) -> Diagnostic {
    let what = match changed {
        Changed::Config => ".git/config is not the regular file alz checked before git wrote ",
        Changed::Git => ".git is not the directory alz checked before git wrote ",
    };
    Diagnostic::error(
        Code::Identity,
        None,
        Message::new(what)
            .then(field.user_key().as_str())
            .then(", so git may have written through a link; alz wrote nothing more; check .git/config by hand"),
    )
}
