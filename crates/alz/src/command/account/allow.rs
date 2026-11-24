//! `alz account allow` and `alz account allow --remove` (spec §7.13 "Trust and `alz account
//! allow`", §10.3): the one command that records which repository may apply the account its layers
//! name, and only for a person at a terminal.
//!
//! The order is the spec's. A person at a terminal is checked first, before any file system call
//! ([`PersonAtTerminal`]), then `$HOME`, the repository, the state of `allowed.yaml`
//! ([`StateDir::check`]) and the layers' declaration, and only then the offer and the questions.
//! After the last answer the declaration is read again, and only [`StateDir`] writes.
//!
//! Standard output gets the offer and the result, standard error the refusals and the prompts.
//! Exit 0 when done, already allowed or nothing to remove; 1 when refused or not confirmed.

use std::io::{self, Write};

use alz_core::account::{AccountName, HomeFact, NotAPerson, Repository};
use alz_core::account::{
    AllowCommand, AllowDecision, AllowEntry, Confirmation, Offer, Refusal, Stack, allowance,
    allowed_lines, already_allowed, distinct_names, not_a_person, nothing_to_remove, offer_lines,
    prompt, remove_offer_lines, removed,
};
use alz_core::path::AbsolutePath;
use alz_core::{Code, Diagnostic, Diagnostics, Message};
use alz_io::{AllowChange, CachedLayers, PersonAtTerminal, StateDir, StateError, Updated};

use crate::cli::AllowArgs;
use crate::command::Surroundings;
use crate::command::account::facts::{Gathered, gather};
use crate::report::emit_diagnostics;

/// The end of a run before its normal end: the exit code, after the reason was printed.
#[derive(Debug, Clone, Copy)]
struct Exit(u8);

/// The streams a run writes to.
struct Streams<'a, O, E> {
    stdout: &'a mut O,
    stderr: &'a mut E,
}

impl<O: Write, E: Write> Streams<'_, O, E> {
    /// Each line on standard output; a stream that cannot be written ends the run with 1.
    fn say(&mut self, lines: &[Message]) -> Result<(), Exit> {
        write_through(self.stdout, &text_of(lines))
    }

    /// Each line on standard error, as it is.
    fn complain(&mut self, lines: &[Message]) -> Result<(), Exit> {
        write_through(self.stderr, &text_of(lines))
    }

    /// The diagnostics on standard error.
    fn report(&mut self, diagnostics: &Diagnostics) -> Result<(), Exit> {
        match emit_diagnostics(diagnostics, self.stderr) {
            0 => Ok(()),
            code => Err(Exit(code)),
        }
    }

    /// `error [account-allow] <text>`, and the exit that follows.
    fn refuse(&mut self, text: Message) -> Exit {
        let diagnostic = Diagnostic::error(Code::AccountAllow, None, text);
        Exit(emit_diagnostics(&Diagnostics::new().with(diagnostic), self.stderr).max(1))
    }

    /// The refusal of a run without a person at a terminal.
    fn refuse_person(&mut self, command: AllowCommand, reason: &NotAPerson) -> Exit {
        let refusal = not_a_person(command, reason);
        let exit = self.refuse(refusal.headline);
        match self.complain(&refusal.following) {
            Ok(()) => exit,
            Err(written) => written,
        }
    }

    /// The refusal of the writer's checks, worded by [`StateError::refusal`].
    fn refuse_state(&mut self, command: AllowCommand, error: &StateError, home: &HomeFact) -> Exit {
        self.refuse(error.refusal(command).message(home))
    }

    /// A diagnostic that ends the run as it is, with its own code.
    fn fail(&mut self, diagnostic: Diagnostic) -> Exit {
        Exit(emit_diagnostics(&Diagnostics::new().with(diagnostic), self.stderr).max(1))
    }

    fn not_confirmed(&mut self) -> Exit {
        match write_through(self.stderr, "not confirmed; nothing changed\n") {
            Ok(()) => Exit(1),
            Err(written) => written,
        }
    }
}

/// The lines as text, each ended by a line feed.
fn text_of(lines: &[Message]) -> String {
    lines
        .iter()
        .flat_map(|line| [line.to_string(), "\n".to_owned()])
        .collect()
}

fn write_through(stream: &mut impl Write, text: &str) -> Result<(), Exit> {
    stream
        .write_all(text.as_bytes())
        .and_then(|()| stream.flush())
        .map_err(|_unwritable: io::Error| Exit(1))
}

/// Runs `alz account allow` or, with `--remove`, `alz account allow --remove`.
pub fn run(
    args: &AllowArgs,
    surroundings: Surroundings<'_>,
    stdout: &mut impl Write,
    stderr: &mut impl Write,
) -> u8 {
    let command = if args.remove {
        AllowCommand::Remove
    } else {
        AllowCommand::Allow
    };
    let mut streams = Streams { stdout, stderr };
    match attempt(command, surroundings, &mut streams) {
        Ok(()) => 0,
        Err(Exit(code)) => code,
    }
}

/// What `$HOME`, the repository and the state of `allowed.yaml` allow, before the layers are read.
struct Ready {
    gathered: Gathered,
    root: AbsolutePath,
    state: StateDir,
    current: Option<AllowEntry>,
}

fn attempt<O: Write, E: Write>(
    command: AllowCommand,
    surroundings: Surroundings<'_>,
    streams: &mut Streams<'_, O, E>,
) -> Result<(), Exit> {
    // Before any file system call: nothing below runs without a person at a terminal.
    let mut person = PersonAtTerminal::check(surroundings.env)
        .map_err(|reason| streams.refuse_person(command, &reason))?;
    let ready = prepare(command, surroundings, streams)?;
    match command {
        AllowCommand::Allow => allow(&mut person, surroundings, streams, &ready),
        AllowCommand::Remove => remove(&mut person, streams, &ready),
    }
}

fn prepare<O: Write, E: Write>(
    command: AllowCommand,
    surroundings: Surroundings<'_>,
    streams: &mut Streams<'_, O, E>,
) -> Result<Ready, Exit> {
    let gathered = gather(surroundings.env, surroundings.directory, &[])
        .map_err(|diagnostic| streams.fail(diagnostic))?;
    streams.report(gathered.search.warnings())?;
    let home = &gathered.facts.home;
    if matches!(home, HomeFact::NotAbsolute) {
        return Err(streams.refuse(Refusal::HomeNotAbsolute.message(home)));
    }
    let (root, manifest_read) = match &gathered.facts.repository {
        Repository::None(why) => {
            return Err(streams.refuse(Refusal::NoRepository(*why).message(home)));
        }
        Repository::Root { path, manifest } => (
            path.clone(),
            matches!(manifest, alz_core::account::ManifestFact::Read),
        ),
    };
    if command == AllowCommand::Allow && !manifest_read {
        return Err(streams.refuse(Refusal::Repair(&root).message(home)));
    }
    let state = StateDir::new(gathered.files.clone());
    let current = gathered
        .facts
        .allowed
        .entries
        .iter()
        .find(|entry| entry.root() == &root)
        .cloned();
    Ok(Ready {
        gathered,
        root,
        state,
        current,
    })
}

fn allow<O: Write, E: Write>(
    person: &mut PersonAtTerminal,
    surroundings: Surroundings<'_>,
    streams: &mut Streams<'_, O, E>,
    ready: &Ready,
) -> Result<(), Exit> {
    let command = AllowCommand::Allow;
    let home = &ready.gathered.facts.home;
    ready
        .state
        .check()
        .map_err(|error| streams.refuse_state(command, &error, home))?;
    let found = allowance(
        &ready.root,
        &declaration_of(surroundings, &ready.gathered),
        ready.current.as_ref(),
    );
    streams.report(&found.warnings)?;
    let offer = match found.decision {
        AllowDecision::AlreadyAllowed => {
            return streams.say(&[already_allowed(&ready.root, home)]);
        }
        AllowDecision::NothingDeclared { has_entry } => {
            return Err(streams.refuse(Refusal::NothingDeclared { has_entry }.message(home)));
        }
        AllowDecision::CacheMiss(miss) => {
            return Err(streams.refuse(Refusal::CacheMiss(miss).message(home)));
        }
        AllowDecision::Conflict(clients) => {
            return Err(streams.refuse(Refusal::Conflict(&clients).message(home)));
        }
        AllowDecision::Reserved => return Err(streams.refuse(Refusal::Reserved.message(home))),
        AllowDecision::Ask(offer) => offer,
    };
    streams.say(&offer_lines(&offer, home, &ready.gathered.facts.folders))?;
    confirm(person, command, &offer.names(), offer.entry(), streams)?;
    // The person took time: everything is read again, from the working directory.
    if !still_declared(surroundings, &ready.root, &offer) {
        return Err(streams.refuse(Refusal::Changed.message(home)));
    }
    let entry = offer.entry();
    match ready.state.update(&AllowChange::Set(entry.clone())) {
        Ok(Updated::Written) => {
            streams.say(&allowed_lines(entry, home, &ready.gathered.facts.folders))
        }
        Ok(Updated::Unchanged) => streams.say(&[already_allowed(&ready.root, home)]),
        Err(error) => Err(streams.refuse_state(command, &error, home)),
    }
}

fn remove<O: Write, E: Write>(
    person: &mut PersonAtTerminal,
    streams: &mut Streams<'_, O, E>,
    ready: &Ready,
) -> Result<(), Exit> {
    let command = AllowCommand::Remove;
    let home = &ready.gathered.facts.home;
    ready
        .state
        .check_state()
        .map_err(|error| streams.refuse_state(command, &error, home))?;
    let Some(entry) = &ready.current else {
        return streams.say(&[nothing_to_remove(&ready.root, home)]);
    };
    ready
        .state
        .check()
        .map_err(|error| streams.refuse_state(command, &error, home))?;
    streams.say(&remove_offer_lines(entry, home))?;
    confirm(
        person,
        command,
        &distinct_names(entry.account()),
        entry,
        streams,
    )?;
    match ready.state.update(&AllowChange::Remove(ready.root.clone())) {
        Ok(Updated::Written) => streams.say(&[removed(&ready.root, home)]),
        Ok(Updated::Unchanged) => streams.say(&[nothing_to_remove(&ready.root, home)]),
        Err(error) => Err(streams.refuse_state(command, &error, home)),
    }
}

/// One prompt for each name, in order; the first answer that is not the name, the end of input or
/// a terminal that cannot be set up ends the run with `not confirmed; nothing changed`.
fn confirm<O: Write, E: Write>(
    person: &mut PersonAtTerminal,
    command: AllowCommand,
    names: &[AccountName],
    entry: &AllowEntry,
    streams: &mut Streams<'_, O, E>,
) -> Result<(), Exit> {
    for name in names {
        let question = prompt(command, name, entry).to_string();
        let confirmed = person
            .ask(&question)
            .is_ok_and(|answer| Confirmation::of(name, &answer).is_confirmed());
        if !confirmed {
            return Err(streams.not_confirmed());
        }
    }
    Ok(())
}

/// The layers' declaration, read from the cache for the repository `gathered` found.
fn declaration_of(
    surroundings: Surroundings<'_>,
    gathered: &Gathered,
) -> alz_core::account::CachedDeclaration {
    CachedLayers::of(
        gathered.search.repository(),
        &gathered.folders,
        &gathered.home,
        surroundings.env,
    )
    .declaration()
}

/// Whether the repository, found again from the working directory, still has the declaration that
/// was shown. A repository that is another one, a `layer.yaml` that cannot be read or a
/// declaration that differs is not.
fn still_declared(surroundings: Surroundings<'_>, root: &AbsolutePath, offer: &Offer) -> bool {
    gather(surroundings.env, surroundings.directory, &[]).is_ok_and(|again| {
        match &again.facts.repository {
            Repository::Root { path, manifest }
                if path == root && matches!(manifest, alz_core::account::ManifestFact::Read) =>
            {
                offer.is_still_declared_by(root, &declaration_of(surroundings, &again))
            }
            Repository::Root { .. } | Repository::None(_) => false,
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use alz_core::account::AllowFileProblem;
    use alz_io::IgnoreReason;

    fn home() -> HomeFact {
        HomeFact::Absolute {
            given: AbsolutePath::try_from("/home/p").expect("absolute"),
            real: None,
        }
    }

    /// What `error` leaves on standard error when `command` is refused, and the exit code.
    fn refused(command: AllowCommand, error: &StateError) -> (u8, String) {
        let (mut stdout, mut stderr) = (Vec::new(), Vec::new());
        let exit = Streams {
            stdout: &mut stdout,
            stderr: &mut stderr,
        }
        .refuse_state(command, error, &home());
        (
            exit.0,
            String::from_utf8(stderr).expect("a refusal is text"),
        )
    }

    #[test]
    fn a_refused_place_is_worded_as_cannot_be_written_for_allow() {
        assert_eq!(
            refused(
                AllowCommand::Allow,
                &StateError::Place(IgnoreReason::PathUnresolvable)
            ),
            (
                1,
                "error [account-allow] allowed.yaml cannot be written: its path cannot be resolved\n"
                    .to_owned()
            )
        );
    }

    #[test]
    fn a_refused_place_is_worded_as_cannot_be_changed_for_remove() {
        assert_eq!(
            refused(
                AllowCommand::Remove,
                &StateError::Place(IgnoreReason::PathUnresolvable)
            ),
            (
                1,
                "error [account-allow] allowed.yaml cannot be changed: its path cannot be resolved\n"
                    .to_owned()
            )
        );
    }

    #[test]
    fn a_file_state_alz_does_not_change_is_followed_by_nothing_was_changed() {
        for command in [AllowCommand::Allow, AllowCommand::Remove] {
            assert_eq!(
                refused(command, &StateError::State(AllowFileProblem::NotAList)),
                (
                    1,
                    "error [account-allow] allowed.yaml is not a list; fix it or delete it, alz does not repair it; nothing was changed\n"
                        .to_owned()
                )
            );
        }
    }

    #[test]
    fn a_failure_to_gather_is_one_error_with_its_own_code() {
        let (mut stdout, mut stderr) = (Vec::new(), Vec::new());
        let exit = Streams {
            stdout: &mut stdout,
            stderr: &mut stderr,
        }
        .fail(Diagnostic::error(
            Code::Account,
            None,
            Message::new("cannot read the file system root"),
        ));
        assert_eq!(exit.0, 1);
        assert_eq!(
            String::from_utf8(stderr).expect("text"),
            "error [account] cannot read the file system root\n"
        );
    }
}
