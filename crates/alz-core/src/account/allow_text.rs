//! The words of `alz account allow` and `--remove` (spec §10.3). Every line is a [`Message`]:
//! repository text is quoted (class 2) or stripped, account names are validated, and no line can
//! hold a control or format character.

use std::fmt;

use crate::account::allow::Offer;
use crate::account::person::NotAPerson;
use crate::account::resolution::{ExistingFolders, HomeFact, NoRepository};
use crate::account::shower::Shower;
use crate::account::which_report::repository_none;
use crate::account::{AccountName, AllowEntry, AllowFileProblem, Miss};
use crate::diagnostic::{Message, MessagePart};
use crate::layer::LayerName;
use crate::path::AbsolutePath;
use crate::target::TargetName;
use crate::untrusted_text::{Escaped, Stripped};

/// Which of the two commands runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AllowCommand {
    /// `alz account allow`.
    Allow,
    /// `alz account allow --remove`.
    Remove,
}

impl AllowCommand {
    /// The command as a person types it.
    #[must_use]
    pub const fn text(self) -> &'static str {
        match self {
            Self::Allow => "alz account allow",
            Self::Remove => "alz account allow --remove",
        }
    }

    const fn verb(self) -> &'static str {
        match self {
            Self::Allow => "allow",
            Self::Remove => "remove",
        }
    }
}

impl fmt::Display for AllowCommand {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.text())
    }
}

/// Why nothing was asked or changed, before any prompt. Each is `error [account-allow] <text>`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Refusal<'a> {
    /// `HOME` is not an absolute path.
    HomeNotAbsolute,
    /// There is no repository here.
    NoRepository(NoRepository),
    /// `layer.yaml` cannot be used.
    Repair(&'a AbsolutePath),
    /// `allowed.yaml` is in a state that alz does not change: `<state>; nothing was changed`.
    State(&'a AllowFileProblem),
    /// The writer's checks of the place of `allowed.yaml` failed: `allowed.yaml cannot be written:
    /// <problem>`, and for `--remove` `allowed.yaml cannot be changed: <problem>`.
    Unwritable {
        /// The command that was refused.
        command: AllowCommand,
        /// What the writer found, worded by alz.
        problem: Message,
    },
    /// The writer refused in its own words, which end the sentence.
    Writer(Message),
    /// The layer cache could not be verified.
    CacheMiss(Miss),
    /// Several client layers declare.
    Conflict(&'a [LayerName]),
    /// No layer names an account here.
    NothingDeclared {
        /// Whether this root has an entry to withdraw.
        has_entry: bool,
    },
    /// The reserved name `alz-blocked`.
    Reserved,
    /// The declaration changed while the person confirmed.
    Changed,
}

impl Refusal<'_> {
    /// The text after `error [account-allow] `.
    #[must_use]
    pub fn message(&self, home: &HomeFact) -> Message {
        match self {
            Self::HomeNotAbsolute => {
                Message::new("HOME is not an absolute path; nothing was changed")
            }
            Self::NoRepository(why) => repository_none(*why).then("; run it inside a repository"),
            Self::Repair(root) => Message::new("repair ")
                .then(Shower::of(home).path(root.as_str()))
                .then("/layer.yaml first; it cannot be used"),
            Self::State(problem) => problem.state().then("; nothing was changed"),
            Self::Unwritable { command, problem } => unwritable(*command, problem),
            Self::Writer(words) => words.clone(),
            Self::CacheMiss(miss) => Message::new("the layer cache cannot be verified (")
                .then(miss.reason().phrase())
                .then("); run alz build; nothing was changed"),
            Self::Conflict(clients) => clients
                .iter()
                .enumerate()
                .fold(
                    Message::new("several client layers declare an account ("),
                    |message, (index, client)| {
                        let separated = if index == 0 {
                            message
                        } else {
                            message.then(", ")
                        };
                        separated.then(Escaped::from_untrusted(&client.to_string()))
                    },
                )
                .then("); remove one declaration, then run alz build; nothing was changed"),
            Self::NothingDeclared { has_entry } => Message::new(
                "the layers name no account here; nothing was changed",
            )
            .then(if *has_entry {
                "; to withdraw the earlier approval run: alz account allow --remove"
            } else {
                ""
            }),
            Self::Reserved => Message::new(
                "the name alz-blocked is reserved for the blocked folders; the layer must name another account; nothing was changed",
            ),
            Self::Changed => Message::new(
                "the declaration changed while you were confirming; run alz account allow again",
            ),
        }
    }
}

/// `allowed.yaml cannot be written: <problem>`, and for `--remove` `allowed.yaml cannot be
/// changed: <problem>` (§10.3).
#[must_use]
pub fn unwritable(command: AllowCommand, problem: &Message) -> Message {
    Message::new(match command {
        AllowCommand::Allow => "allowed.yaml cannot be written: ",
        AllowCommand::Remove => "allowed.yaml cannot be changed: ",
    })
    .and(problem.clone())
}

/// The refusal of a run without a person at a terminal: the headline is
/// `error [account-allow] <text>`, the lines after it are printed as they are.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PersonRefusal {
    /// The text after `error [account-allow] `.
    pub headline: Message,
    /// The lines that follow, the last being `nothing was changed`.
    pub following: Vec<Message>,
}

/// The refusal of `command` for `reason`: every reason that applies, the markers by name only.
#[must_use]
pub fn not_a_person(command: AllowCommand, reason: &NotAPerson) -> PersonRefusal {
    let terminal = reason
        .lacks_terminal()
        .then(|| Message::new("standard input is not a terminal"));
    let session = reason.agent_session().map(|session| {
        session
            .markers()
            .enumerate()
            .fold(
                Message::new("an agent session is running here ("),
                |message, (index, marker)| {
                    let separated = if index == 0 {
                        message
                    } else {
                        message.then(", ")
                    };
                    separated.then(marker.name())
                },
            )
            .then(" set)")
    });
    let headline = terminal.into_iter().chain(session).enumerate().fold(
        Message::new(command.text()).then(" must be run by a person at a terminal: "),
        |message, (index, why)| {
            let separated = if index == 0 {
                message
            } else {
                message.then("; ")
            };
            separated.and(why)
        },
    );
    let ide = reason
        .agent_session()
        .is_some_and(crate::account::AgentSession::includes_claude_code)
        .then(|| {
            Message::new(
                "Claude Code also sets CLAUDECODE in an IDE's integrated terminal; run alz account allow in a terminal window outside the IDE",
            )
        });
    PersonRefusal {
        headline,
        following: ide
            .into_iter()
            .chain([Message::new("nothing was changed")])
            .collect(),
    }
}

/// `<target>=<name>` for each target, joined by `, `.
fn map_text(entry: &AllowEntry) -> Message {
    entry
        .account()
        .iter()
        .enumerate()
        .fold(Message::new(""), |message, (index, (target, name))| {
            let separated = if index == 0 {
                message
            } else {
                message.then(", ")
            };
            separated
                .then(MessagePart::printable(&target))
                .then("=")
                .then(MessagePart::printable(name))
        })
}

fn shown_root(root: &AbsolutePath, home: &HomeFact) -> Stripped {
    Shower::of(home).path(root.as_str())
}

/// `already allowed: <root>`.
#[must_use]
pub fn already_allowed(root: &AbsolutePath, home: &HomeFact) -> Message {
    Message::new("already allowed: ").then(shown_root(root, home))
}

/// `nothing to remove: <root> has no entry in allowed.yaml`.
#[must_use]
pub fn nothing_to_remove(root: &AbsolutePath, home: &HomeFact) -> Message {
    Message::new("nothing to remove: ")
        .then(shown_root(root, home))
        .then(" has no entry in allowed.yaml")
}

/// `removed the entry for <root> from allowed.yaml`.
#[must_use]
pub fn removed(root: &AbsolutePath, home: &HomeFact) -> Message {
    Message::new("removed the entry for ")
        .then(shown_root(root, home))
        .then(" from allowed.yaml")
}

fn entry_source(entry: &AllowEntry) -> Escaped {
    Escaped::from_untrusted(entry.source().as_str())
}

/// The offer on standard output (§10.3), for a root that is not allowed yet or differently.
#[must_use]
pub fn offer_lines(offer: &Offer, home: &HomeFact, folders: &ExistingFolders) -> Vec<Message> {
    let entry = offer.entry();
    let accounts = TargetName::ALL.into_iter().map(|target| {
        let head = Message::new("  ")
            .then(MessagePart::printable(&target))
            .then(": ");
        match entry.account().get(target) {
            None => head.then("not named by the layers (the other sources decide)"),
            Some(name) => head.then(MessagePart::printable(name)).then(
                if name == &AccountName::default_account() {
                    " (the default account)"
                } else if folders.exists(target, name) {
                    " (the account folder exists)"
                } else {
                    " (the account folder does not exist yet)"
                },
            ),
        }
    });
    let revision = offer.revision().map_or_else(
        || Message::new("  revision: none (unpinned path: layer)"),
        |commit| Message::new("  revision: ").then(Stripped::from_untrusted(&commit.to_string())),
    );
    [
        Message::new("repository: ").then(shown_root(entry.root(), home)),
        Message::new("accounts the layers name:"),
    ]
    .into_iter()
    .chain(accounts)
    .chain([
        Message::new("declared by the layer ")
            .then(Escaped::from_untrusted(&offer.layer().to_string()))
            .then(", stack entry ")
            .then(offer.position().saturating_add(1)),
        Message::new("  source as written: ")
            .then(Escaped::from_untrusted(&offer.source().to_string())),
    ])
    .chain(
        offer.git_ref().map(|git_ref| {
            Message::new("  ref: ").then(Escaped::from_untrusted(&git_ref.to_string()))
        }),
    )
    .chain([
        revision,
        Message::new("  recorded source: ").then(entry_source(entry)),
    ])
    .chain(offer.current().map(|current| {
        Message::new("currently allowed: ")
            .and(map_text(current))
            .then(" from ")
            .then(entry_source(current))
    }))
    .chain([Message::new(
        "Claude Code and Codex started here will use these accounts' logins and MCP connections.",
    )])
    .collect()
}

/// What `--remove` shows of the entry it would remove.
#[must_use]
pub fn remove_offer_lines(entry: &AllowEntry, home: &HomeFact) -> Vec<Message> {
    vec![
        Message::new("repository: ").then(shown_root(entry.root(), home)),
        Message::new("allowed now: ")
            .and(map_text(entry))
            .then(" from ")
            .then(entry_source(entry)),
        Message::new(
            "removing it sends this repository back to the default account unless accounts.yaml names one",
        ),
    ]
}

/// The prompt for `name` on standard error: the real root and the source are repeated, because the
/// details go to standard output and the prompt to standard error.
#[must_use]
pub fn prompt(command: AllowCommand, name: &AccountName, entry: &AllowEntry) -> Message {
    Message::new("Type ")
        .then(MessagePart::printable(name))
        .then(" to ")
        .then(command.verb())
        .then(" it for ")
        .then(Escaped::from_untrusted(entry.root().as_str()))
        .then(" (source ")
        .then(entry_source(entry))
        .then("): ")
}

/// After a write: the entry, and a note for each named account whose folder does not exist.
#[must_use]
pub fn allowed_lines(
    entry: &AllowEntry,
    home: &HomeFact,
    folders: &ExistingFolders,
) -> Vec<Message> {
    let notes = entry
        .account()
        .iter()
        .filter(|(target, name)| {
            **name != AccountName::default_account() && !folders.exists(*target, name)
        })
        .map(|(target, name)| {
            Message::new("note: the ")
                .then(MessagePart::printable(&target))
                .then(" account folder for ")
                .then(MessagePart::printable(name))
                .then(" does not exist yet; create it with: alz account init ")
                .then(MessagePart::printable(name))
        });
    std::iter::once(
        Message::new("allowed: ")
            .then(shown_root(entry.root(), home))
            .then(" uses ")
            .and(map_text(entry))
            .then(" from ")
            .then(entry_source(entry)),
    )
    .chain(notes)
    .collect()
}

#[cfg(test)]
mod tests;
