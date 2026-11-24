//! The lines the identity steps print (spec §9.4). A line is built from the finding's source and
//! the declared identity, never from what git reported: a finding holds no reported value, and
//! the only values that reach a line are the declared ones, in a fix command only when
//! [`ShellSafe`] allows it. Origins are stripped, with the home folder shown as `~`.

use std::collections::BTreeSet;

use crate::diagnostic::{Code, Diagnostic, Level, Message};
use crate::identity::check::{Command, Findings, IdentityCheck};
use crate::identity::declared::{DeclaredBy, DeclaredIdentity};
use crate::identity::finding::{Finding, Source};
use crate::identity::origin::GitScope;
use crate::identity::setup::Blocker;
use crate::identity::shell_safe::{FixCommand, FixScope, ShellSafe};
use crate::identity::slot::{ConfigKey, Field};
use crate::untrusted_text::Stripped;

/// The home folder as the environment gives it, so an origin below it prints as `~/...`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Home(Stripped);

impl Home {
    /// The home folder, stripped of control and format characters.
    #[must_use]
    pub fn new(path: &str) -> Self {
        Self(Stripped::from_untrusted(path))
    }

    pub(super) fn display(&self, path: &Stripped) -> Stripped {
        let base = self.0.as_str();
        let shown = (!base.is_empty() && base != "/")
            .then(|| path.as_str().strip_prefix(base))
            .flatten()
            .filter(|rest| rest.is_empty() || rest.starts_with('/'))
            .map(|rest| format!("~{rest}"));
        shown.map_or_else(|| path.clone(), |text| Stripped::from_untrusted(&text))
    }
}

/// What to do about one wrong part.
enum Fix {
    /// `unset <words>`.
    Unset(&'static str),
    /// `unset GIT_CONFIG_COUNT GIT_CONFIG_PARAMETERS`: the value comes from the command scope.
    UnsetCommandConfig,
    /// Set the key to the declared value.
    Set(ConfigKey, FixScope),
}

impl Fix {
    fn of(source: &Source) -> Self {
        match source {
            Source::Environment(slot) => Self::Unset(slot.variable()),
            Source::Config { key, origin } => match origin.scope() {
                GitScope::Command => Self::UnsetCommandConfig,
                GitScope::Worktree => Self::Set(*key, FixScope::Worktree),
                GitScope::System | GitScope::Global | GitScope::Local | GitScope::Unknown => {
                    Self::Set(*key, FixScope::Local)
                }
            },
            Source::EmailVariable => Self::Set(Field::Email.user_key(), FixScope::Local),
            Source::Guessed(field) => Self::Set(field.user_key(), FixScope::Local),
            Source::Unknown(key) => Self::Set(*key, FixScope::Local),
        }
    }
}

/// The report of one identity step: the declaration it checks against and the home folder.
#[derive(Debug, Clone, Copy)]
pub struct Report<'a> {
    declared: &'a DeclaredIdentity,
    home: Option<&'a Home>,
}

impl<'a> Report<'a> {
    /// A report for `declared`; `home` is the home folder when it is known.
    #[must_use]
    pub const fn new(declared: &'a DeclaredIdentity, home: Option<&'a Home>) -> Self {
        Self { declared, home }
    }

    /// The lines of a check, for `command`: nothing when the identity matches, a note outside a
    /// repository, one line when git cannot be used, and one line per distinct source of a wrong
    /// part otherwise, in a fixed order. `with_fix` is false where a command run by hand would
    /// write to another repository.
    #[must_use]
    pub fn check_lines(
        &self,
        command: Command,
        check: &IdentityCheck,
        with_fix: bool,
    ) -> Vec<Diagnostic> {
        match check {
            IdentityCheck::Matches => Vec::new(),
            IdentityCheck::NotARepository => vec![Diagnostic::note(
                Code::Identity,
                None,
                Message::new(
                    "this is not a git repository, so the commit identity was not checked",
                ),
            )],
            IdentityCheck::Unavailable => vec![line(
                command.level(),
                Message::new(
                    "git is not available or failed, so the commit identity could not be checked",
                ),
            )],
            IdentityCheck::Differs(findings) => {
                self.finding_lines(command.level(), findings, with_fix)
            }
        }
    }

    /// One line per distinct source key, name before email and author before committer, whatever
    /// order the findings came in.
    #[must_use]
    pub fn finding_lines(
        &self,
        level: Level,
        findings: &Findings,
        with_fix: bool,
    ) -> Vec<Diagnostic> {
        distinct(findings.as_slice())
            .into_iter()
            .map(|finding| line(level, self.finding_message(finding, with_fix)))
            .collect()
    }

    fn finding_message(&self, finding: &Finding, with_fix: bool) -> Message {
        let intro = Message::new("git takes the ")
            .then(finding.slot().field().word())
            .then(" (")
            .then(finding.source().key().as_str())
            .then(") from ");
        let message = self
            .source_text(intro, finding.source())
            .then(", which differs from the commit identity declared by ");
        let message = with_by(message, &self.declared.by);
        if with_fix {
            self.fix_text(
                message.then("; fix: "),
                &Fix::of(finding.source()),
                finding.slot().field(),
            )
        } else {
            message
        }
    }

    fn source_text(&self, message: Message, source: &Source) -> Message {
        match source {
            Source::Environment(_) | Source::EmailVariable => message
                .then("the environment (")
                .then(source.key().as_str())
                .then(")"),
            Source::Guessed(_) => message.then("no setting (git guessed it from the host)"),
            Source::Unknown(_) => message.then("a source git could not report"),
            Source::Config { origin, .. } => {
                let scope = match origin.scope() {
                    GitScope::Command => "command-line",
                    GitScope::System => "system",
                    GitScope::Global => "global",
                    GitScope::Local => "local",
                    GitScope::Worktree => "worktree",
                    GitScope::Unknown => "unknown",
                };
                message
                    .then(scope)
                    .then(" config (")
                    .then(self.home.map_or_else(
                        || origin.origin().clone(),
                        |home| home.display(origin.origin()),
                    ))
                    .then(")")
            }
        }
    }

    fn declared_value(&self, field: Field) -> &str {
        match field {
            Field::Name => self.declared.identity.name().as_str(),
            Field::Email => self.declared.identity.email().as_str(),
        }
    }

    /// The fix: a command when the declared value is shell-safe, else a sentence.
    fn fix_text(&self, message: Message, fix: &Fix, field: Field) -> Message {
        match fix {
            Fix::Unset(variable) => message.then("unset ").then(*variable),
            Fix::UnsetCommandConfig => message.then("unset GIT_CONFIG_COUNT GIT_CONFIG_PARAMETERS"),
            Fix::Set(key, scope) => {
                self.set_text(message, *key, *scope, self.declared_value(field))
            }
        }
    }

    fn set_text(&self, message: Message, key: ConfigKey, scope: FixScope, value: &str) -> Message {
        match ShellSafe::try_from(value) {
            Ok(safe) => message.then(Stripped::from_untrusted(
                &FixCommand::new(scope, key, safe).text(),
            )),
            Err(_) => with_by(
                message
                    .then("set ")
                    .then(key.as_str())
                    .then(" to the value declared in "),
                &self.declared.by,
            )
            .then(" (`alz build --adopt` adds it when unset)"),
        }
    }

    /// The warning for a place alz may not write: the reason, the commands to run by hand (none
    /// where they would write to another repository), and what to do instead.
    #[must_use]
    pub fn blocked_line(&self, blocker: Blocker) -> Diagnostic {
        let message = with_by(
            Message::new(blocker.reason())
                .then(", so alz did not write the git identity declared by "),
            &self.declared.by,
        );
        let message = if blocker.would_reach_another_repository() {
            message.then(". Run `git init` here first, or run alz from the repository's top level.")
        } else {
            Field::ALL.into_iter().enumerate().fold(
                message.then(". Run by hand: "),
                |message, (index, field)| {
                    let message = if index == 0 {
                        message
                    } else {
                        message.then("; ")
                    };
                    self.set_text(
                        message,
                        field.user_key(),
                        FixScope::Local,
                        self.declared_value(field),
                    )
                },
            )
        };
        line(Level::Warning, message)
    }

    /// The line printed when a key was written: `set user.email in .git/config to '...' (declared
    /// by layer ...)`, with the declared value.
    #[must_use]
    pub fn written_line(&self, field: Field) -> Message {
        with_by(
            Message::new("set ")
                .then(field.user_key().as_str())
                .then(" in .git/config to '")
                .then(Stripped::from_untrusted(self.declared_value(field)))
                .then("' (declared by "),
            &self.declared.by,
        )
        .then(")")
    }

    /// The warning for a local key that holds another value, which alz never replaces.
    #[must_use]
    pub fn kept_line(&self, field: Field) -> Diagnostic {
        let message = with_by(
            Message::new(field.user_key().as_str())
                .then(" in .git/config differs from the identity declared by "),
            &self.declared.by,
        )
        .then("; alz never replaces it. Fix: ");
        line(
            Level::Warning,
            self.set_text(
                message,
                field.user_key(),
                FixScope::Local,
                self.declared_value(field),
            ),
        )
    }

    /// The warning for a key whose value could not be read.
    #[must_use]
    pub fn unreadable_line(field: Field) -> Diagnostic {
        line(
            Level::Warning,
            Message::new("could not read .git/config; set ")
                .then(field.user_key().as_str())
                .then(" by hand"),
        )
    }

    /// The warning for a key that could not be written.
    #[must_use]
    pub fn write_failed_line(field: Field) -> Diagnostic {
        line(
            Level::Warning,
            Message::new("could not write ")
                .then(field.user_key().as_str())
                .then(" to .git/config; set it by hand"),
        )
    }
}

fn line(level: Level, message: Message) -> Diagnostic {
    match level {
        Level::Error => Diagnostic::error(Code::Identity, None, message),
        Level::Warning => Diagnostic::warning(Code::Identity, None, message),
        Level::Note => Diagnostic::note(Code::Identity, None, message),
    }
}

/// `layer 'client' (path:../client)`, appended to `message`.
fn with_by(message: Message, by: &DeclaredBy) -> Message {
    message
        .then("layer '")
        .then(crate::diagnostic::MessagePart::printable(&by.layer))
        .then("' (")
        .then(by.source.clone())
        .then(")")
}

/// The findings in report order, one per source key.
fn distinct(findings: &[Finding]) -> Vec<&Finding> {
    let mut sorted: Vec<&Finding> = findings.iter().collect();
    sorted.sort_by_key(|finding| finding.slot());
    let mut seen = BTreeSet::new();
    sorted
        .into_iter()
        .filter(|finding| seen.insert(finding.source().key()))
        .collect()
}
