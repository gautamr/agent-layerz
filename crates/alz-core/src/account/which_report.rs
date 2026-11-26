//! The text of `alz account which` (spec §10.3), built from a [`Resolution`] and the [`Facts`] it
//! was made from. Every line is a [`Message`], so repository text and environment values reach it
//! only stripped or quoted, and a name only as a validated [`AccountName`]; no line can hold a
//! control or format character.

use std::fmt;

use crate::account::AccountFamily;
use crate::account::AccountName;
use crate::account::resolution::{
    Applied, BLOCKED, Facts, FailsClosed, FileFact, FileOutcome, Fix, HookApplies, ManifestFact,
    ManifestProblem, NoRepository, Recorded, Repository, Resolution, SkipReason, Skipped,
    TargetResolution, Verdict, variable,
};
use crate::account::shower::Shower;
use crate::diagnostic::{Diagnostics, Message, MessagePart};
use crate::untrusted_text::{Quoted, Stripped};

/// The report: standard output as lines, the warnings for standard error, and the exit status.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WhichReport {
    lines: Vec<Message>,
    warnings: Diagnostics,
    resolves: bool,
}

impl WhichReport {
    /// The lines of standard output, in order.
    #[must_use]
    pub fn lines(&self) -> &[Message] {
        &self.lines
    }

    /// The warnings of standard error, after the report: those the files and the walk raised,
    /// then resolution's.
    #[must_use]
    pub const fn warnings(&self) -> &Diagnostics {
        &self.warnings
    }

    /// 0 when every target resolves, 1 when one fails closed.
    #[must_use]
    pub const fn exit_code(&self) -> u8 {
        if self.resolves { 0 } else { 1 }
    }
}

impl fmt::Display for WhichReport {
    /// Standard output: each line, then a line feed.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.lines
            .iter()
            .try_for_each(|line| writeln!(formatter, "{line}"))
    }
}

/// The report of `resolution`. `earlier_warnings` are those found before resolution (`$HOME`, the
/// root walk, the two files); the resolution's own follow them.
#[must_use]
pub fn which_report(
    facts: &Facts,
    resolution: &Resolution,
    earlier_warnings: Diagnostics,
) -> WhichReport {
    let shower = Shower::of(&facts.home);
    let lines = [repository_line(facts, &shower)]
        .into_iter()
        .chain([
            file_line("accounts.yaml", &facts.config.file, false),
            file_line("allowed.yaml", &facts.allowed.file, resolution.is_allowed()),
        ])
        .chain(stale_lines(facts, &shower))
        .chain(
            resolution
                .targets()
                .flat_map(|(target, found)| target_lines(facts, target, found, &shower)),
        )
        .collect();
    WhichReport {
        lines,
        warnings: earlier_warnings.merge(resolution.warnings().clone()),
        resolves: resolution.all_resolve(),
    }
}

fn repository_line(facts: &Facts, shower: &Shower) -> Message {
    match &facts.repository {
        Repository::Root { path, manifest } => {
            let line = Message::new("repository: ").then(shower.path(path.as_str()));
            match manifest {
                ManifestFact::Read => line,
                ManifestFact::Unreadable => line.then(" (layer.yaml cannot be read)"),
                ManifestFact::Invalid => line.then(" (layer.yaml is not valid)"),
            }
        }
        Repository::None(why) => repository_none(*why),
    }
}

/// `repository: none (<why>)`, the line `alz account which` and `alz account allow` share.
pub(crate) fn repository_none(why: NoRepository) -> Message {
    Message::new("repository: none (")
        .then(match why {
            NoRepository::NotFound => "no repository here",
            NoRepository::AccountFolder => "the working directory is inside an account folder",
            NoRepository::ConfiguredFolder => {
                "the working directory is inside a folder CLAUDE_CONFIG_DIR or CODEX_HOME names"
            }
            NoRepository::Unresolvable => "the working directory cannot be resolved",
        })
        .then(")")
}

fn file_line(name: &'static str, file: &FileFact, allowed: bool) -> Message {
    let head = Message::new(name).then(": ").then(file.shown.clone());
    match &file.outcome {
        FileOutcome::NotFound => head.then(" (not found)"),
        FileOutcome::Ignored(reason) => head.then(" (ignored: ").and(reason.clone()).then(")"),
        FileOutcome::Read if allowed => head.then(" (read; this repository is allowed)"),
        FileOutcome::Read => head.then(" (read)"),
    }
}

fn stale_lines(facts: &Facts, shower: &Shower) -> Vec<Message> {
    let mut roots: Vec<&str> = facts
        .allowed
        .unresolved
        .iter()
        .map(crate::path::AbsolutePath::as_str)
        .collect();
    roots.sort_unstable();
    roots
        .into_iter()
        .map(|root| {
            Message::new("  stale approval: ")
                .then(Quoted::from_untrusted(&shower.text(root)))
                .then(" does not exist here")
        })
        .collect()
}

const fn skip_phrase(reason: SkipReason, target: AccountFamily) -> SkipText {
    match reason {
        SkipReason::NoUseChoice => SkipText::Fixed("no alz use choice"),
        SkipReason::NoUserConfig => SkipText::Fixed("no accounts.yaml"),
        SkipReason::UserConfigIgnored => SkipText::Fixed("accounts.yaml is ignored"),
        SkipReason::NoFoldersMatch => SkipText::Fixed("no folders entry for this directory"),
        SkipReason::EntryNamesNoTarget => {
            SkipText::NoAccount("the folders entry names no ", target)
        }
        SkipReason::NoRepository => SkipText::Fixed("no repository here"),
        SkipReason::CwdInAccountFolder => {
            SkipText::Fixed("the working directory is inside an account folder")
        }
        SkipReason::CwdUnresolvable => SkipText::Fixed("the working directory cannot be resolved"),
        SkipReason::CwdInConfiguredFolder => SkipText::Fixed(
            "the working directory is inside a folder CLAUDE_CONFIG_DIR or CODEX_HOME names",
        ),
        SkipReason::AllowedMissing => SkipText::Fixed("no allowed.yaml"),
        SkipReason::AllowedIgnored => SkipText::Fixed("allowed.yaml is ignored"),
        SkipReason::NotAllowed => SkipText::Fixed("this repository is not allowed"),
        SkipReason::LayersNameNoTarget => SkipText::NoAccount("the layers name no ", target),
    }
}

enum SkipText {
    Fixed(&'static str),
    NoAccount(&'static str, AccountFamily),
}

impl SkipText {
    fn message(self) -> Message {
        match self {
            Self::Fixed(text) => Message::new(text),
            Self::NoAccount(lead, target) => Message::new(lead)
                .then(MessagePart::printable(&target))
                .then(" account"),
        }
    }
}

fn skipped_lines(skipped: &[Skipped], target: AccountFamily) -> impl Iterator<Item = Message> + '_ {
    skipped.iter().map(move |skip| {
        Message::new("  ")
            .then(skip.step.as_str())
            .then(" step skipped (")
            .and(skip_phrase(skip.reason, target).message())
            .then(")")
    })
}

fn target_lines(
    facts: &Facts,
    target: AccountFamily,
    found: &TargetResolution,
    shower: &Shower,
) -> Vec<Message> {
    let head = Message::new(MessagePart::printable(&target)).then(": ");
    match &found.verdict {
        Verdict::Applied(applied) => {
            let account = match applied {
                Applied::Kept(_) => Message::new("set outside alz"),
                Applied::Folder(name) => Message::new(MessagePart::printable(name)),
                Applied::Default => facts.config.default_name.as_ref().map_or_else(
                    || Message::new("default"),
                    |name| {
                        Message::new("default, called ")
                            .then(MessagePart::printable(name))
                            .then(" here")
                    },
                ),
            };
            let what = match applied {
                Applied::Folder(name) => Message::new(variable(target))
                    .then("=")
                    .then(shower.folder(&facts.home, target, name)),
                Applied::Default => Message::new(variable(target)).then(" unset"),
                Applied::Kept(value) => Message::new(variable(target))
                    .then("=\"")
                    .then(shower.path(value.as_str()))
                    .then("\" kept"),
            };
            let line = head
                .and(account)
                .then(" (source: ")
                .then(found.source.as_str())
                .then(") ")
                .and(what);
            std::iter::once(line)
                .chain(skipped_lines(&found.skipped, target))
                .collect()
        }
        Verdict::FailsClosed(failure) => {
            let line = head
                .then("fails closed (source: ")
                .then(found.source.as_str())
                .then(")");
            std::iter::once(line)
                .chain(skipped_lines(&found.skipped, target))
                .chain(failure_lines(facts, target, found, failure, shower))
                .collect()
        }
    }
}

fn failure_lines(
    facts: &Facts,
    target: AccountFamily,
    found: &TargetResolution,
    failure: &FailsClosed,
    shower: &Shower,
) -> Vec<Message> {
    let folder = |name: &AccountName| shower.folder(&facts.home, target, name);
    vec![
        failure_headline(found.source.as_str(), failure, &folder),
        hook_line(
            target,
            &failure.hook(),
            &folder,
            &shower.folder(&facts.home, target, &BLOCKED),
        ),
        fix_line(facts, failure, shower),
    ]
}

fn failure_headline(
    step: &'static str,
    failure: &FailsClosed,
    folder: &impl Fn(&AccountName) -> Stripped,
) -> Message {
    match failure {
        FailsClosed::InvalidName => Message::new("  ")
            .then(step)
            .then(" step names something that is not an account name; fails closed"),
        FailsClosed::HomeNotAbsolute => Message::new("  ")
            .then(step)
            .then(" step names an account, but HOME is not an absolute path; fails closed"),
        FailsClosed::MissingFolder { name, .. } => Message::new("  ")
            .then(step)
            .then(" step names ")
            .then(MessagePart::printable(name))
            .then(", but ")
            .then(folder(name))
            .then(" does not exist; fails closed"),
        FailsClosed::CacheMiss { miss, .. } => Message::new("  layer step skipped (cache miss: ")
            .then(miss.phrase())
            .then("); fails closed"),
        FailsClosed::Changed { still_declared } => Message::new("  layer step skipped (")
            .then(if *still_declared {
                "the layers' account declaration changed since it was allowed"
            } else {
                "the layers no longer declare an account"
            })
            .then("); fails closed"),
        FailsClosed::SourceGone => Message::new(
            "  layer step skipped (the allowed source is no longer in the stack); fails closed",
        ),
        FailsClosed::Conflict => Message::new(
            "  layer step skipped (several client layers declare an account); fails closed",
        ),
        FailsClosed::Manifest(problem) => Message::new("  layer step skipped (")
            .then(match problem {
                ManifestProblem::Unreadable => "layer.yaml cannot be read",
                ManifestProblem::Invalid => "layer.yaml is not valid",
            })
            .then("); fails closed"),
        FailsClosed::ConfiguredFolder => Message::new(
            "  layer step skipped (an allowed repository lies in a folder CLAUDE_CONFIG_DIR or CODEX_HOME names, so its layers are not read); fails closed",
        ),
    }
}

fn hook_line(
    target: AccountFamily,
    hook: &HookApplies,
    folder: &impl Fn(&AccountName) -> Stripped,
    blocked: &Stripped,
) -> Message {
    let variable = variable(target);
    match hook {
        HookApplies::NoFolder => Message::new("  the hook sets no folder"),
        HookApplies::MissingFolder(name) => Message::new("  the hook applies ")
            .then(variable)
            .then("=")
            .then(folder(name))
            .then(" (the missing folder)"),
        HookApplies::Recorded(Recorded::Default) => Message::new("  the hook applies ")
            .then(variable)
            .then(" unset (the recorded account default)"),
        HookApplies::Recorded(Recorded::Folder { name, exists }) => {
            Message::new("  the hook applies ")
                .then(variable)
                .then("=")
                .then(folder(name))
                .then(" (the recorded account ")
                .then(MessagePart::printable(name))
                .then(if *exists {
                    ")"
                } else {
                    "; that folder does not exist)"
                })
        }
        HookApplies::Blocked => Message::new("  the hook applies ")
            .then(variable)
            .then("=")
            .then(blocked.clone())
            .then(" (the blocked folder)"),
    }
}

fn fix_line(facts: &Facts, failure: &FailsClosed, shower: &Shower) -> Message {
    Message::new("  fix: ").and(match failure.fix() {
        Fix::UseAValidName => {
            Message::new("use a name of lower-case letters, digits and '-' (or default)")
        }
        Fix::SetHome => Message::new("set HOME to an absolute path"),
        Fix::AccountInit(name) => {
            let existing = match failure {
                FailsClosed::MissingFolder { existing, .. } => existing.as_slice(),
                FailsClosed::InvalidName
                | FailsClosed::HomeNotAbsolute
                | FailsClosed::CacheMiss { .. }
                | FailsClosed::Changed { .. }
                | FailsClosed::SourceGone
                | FailsClosed::Conflict
                | FailsClosed::Manifest(_)
                | FailsClosed::ConfiguredFolder => &[],
            };
            Message::new("alz account init ")
                .then(MessagePart::printable(&name))
                .then(" (")
                .and(folders_here(existing))
                .then(")")
        }
        Fix::ReservedName => Message::new(
            "the name alz-blocked is reserved for the blocked folders; choose another account name",
        ),
        Fix::Build => Message::new("alz build"),
        Fix::Allow => Message::new("alz account allow"),
        Fix::AllowRemove => Message::new("alz account allow --remove"),
        Fix::UnsetConfiguredFolders => Message::new(
            "unset CLAUDE_CONFIG_DIR and CODEX_HOME here, or point them outside the repository",
        ),
        Fix::RemoveDeclarations => {
            Message::new("remove one of the declarations, then run alz build")
        }
        Fix::RepairManifest => {
            let root = match &facts.repository {
                Repository::Root { path, .. } => shower.path(path.as_str()),
                Repository::None(_) => Stripped::from_untrusted(""),
            };
            Message::new("repair ").then(root).then("/layer.yaml")
        }
    })
}

fn folders_here(existing: &[AccountName]) -> Message {
    if existing.is_empty() {
        Message::new("no account folders here")
    } else {
        existing.iter().enumerate().fold(
            Message::new("account folders here: "),
            |message, (index, name)| {
                let separated = if index == 0 {
                    message
                } else {
                    message.then(", ")
                };
                separated.then(MessagePart::printable(name))
            },
        )
    }
}

#[cfg(test)]
mod tests;
