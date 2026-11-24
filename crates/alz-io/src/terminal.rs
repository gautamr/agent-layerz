//! A person at the terminal (spec §7.13 "Trust and `alz account allow`", decision-27's
//! amendment of 2026-11-24).
//!
//! [`PersonAtTerminal`] is the proof that standard input is a terminal and that no agent session
//! is running. It is made only by [`PersonAtTerminal::check`], which makes no file system call,
//! and it is the only code in alz that reads standard input (`std::io::stdin` is banned in every
//! crate's `clippy.toml` but this one's, and a test scans the sources too).
//!
//! While one answer is read, the terminal keeps its line editing and echo, but its interrupt key
//! sends no signal and ends the line instead, so Ctrl-C arrives as an answer that is not the
//! name. The terminal's settings are saved before and restored on every path: a [`Drop`] guard
//! covers an unwinding panic, and the normal path restores explicitly so a failure is reported.

use std::io::{self, BufRead, Stdin, Write};

use alz_core::account::{Answer, NotAPerson, decide_person_at_terminal};
use rustix::termios::{
    LocalModes, OptionalActions, SpecialCodeIndex, Termios, isatty, tcgetattr, tcsetattr,
};
use thiserror::Error;

use crate::env::EnvSnapshot;

/// Why a question could not be asked or answered. The caller treats each as "not confirmed".
#[derive(Debug, Error)]
pub enum TerminalError {
    /// The terminal's settings could not be read, so nothing was asked.
    #[error("cannot read the terminal's settings: {0}")]
    ReadSettings(#[source] io::Error),
    /// The terminal's settings could not be changed, so nothing was asked.
    #[error("cannot change the terminal's settings: {0}")]
    ChangeSettings(#[source] io::Error),
    /// The prompt could not be written to standard error.
    #[error("cannot write the prompt: {0}")]
    Prompt(#[source] io::Error),
    /// The answer could not be read.
    #[error("cannot read the answer: {0}")]
    Read(#[source] io::Error),
    /// The terminal's settings could not be put back.
    #[error("cannot restore the terminal's settings: {0}")]
    RestoreSettings(#[source] io::Error),
}

/// Proof that a person is at a terminal: standard input is one, and no agent-session marker is
/// set. The capability to ask a question and read the answer.
#[derive(Debug)]
pub struct PersonAtTerminal {
    stdin: Stdin,
}

impl PersonAtTerminal {
    /// Checks that standard input is a terminal and that none of the six agent-session markers
    /// is set in `env`, even to an empty value. Makes no file system call and reads nothing.
    ///
    /// # Errors
    ///
    /// [`NotAPerson`], naming every reason that applies.
    #[expect(
        clippy::disallowed_methods,
        reason = "the one place that touches standard input (spec §7.13, TASK-55.7.11)"
    )]
    pub fn check(env: &EnvSnapshot) -> Result<Self, NotAPerson> {
        let stdin = io::stdin();
        decide_person_at_terminal(isatty(&stdin), env.vars()).map(|()| Self { stdin })
    }

    /// Writes `prompt` to standard error and reads one answer from standard input. The answer is
    /// never printed. The terminal's interrupt key ends the line during the read, and its
    /// settings are as they were when this returns, whatever it returns.
    ///
    /// # Errors
    ///
    /// [`TerminalError`] when the settings cannot be read or changed (nothing is asked then),
    /// when the prompt or the answer cannot be written or read, or when the settings cannot be
    /// restored.
    pub fn ask(&mut self, prompt: &str) -> Result<Answer, TerminalError> {
        let settings = InterruptEndsTheLine::set(&self.stdin)?;
        let answer = prompt_and_read(&self.stdin, prompt);
        let restored = settings.restore();
        let answer = answer?;
        restored.map(|()| answer)
    }
}

/// Writes the prompt, reads one answer, and ends the echoed line when the answer did not.
fn prompt_and_read(stdin: &Stdin, prompt: &str) -> Result<Answer, TerminalError> {
    let mut standard_error = io::stderr().lock();
    standard_error
        .write_all(prompt.as_bytes())
        .and_then(|()| standard_error.flush())
        .map_err(TerminalError::Prompt)?;
    let answer = read_answer(stdin).map_err(TerminalError::Read)?;
    // An answer that ended without a newline (the interrupt key, the end of input) leaves the
    // cursor on the prompt's line; the next message starts on a line of its own. A failure to
    // write it changes nothing.
    if answer == Answer::Ended {
        standard_error.write_all(b"\n").unwrap_or_default();
    }
    Ok(answer)
}

/// One answer, from what one read of the terminal delivered.
fn read_answer(stdin: &Stdin) -> io::Result<Answer> {
    let mut input = stdin.lock();
    let (answer, taken) = loop {
        match input.fill_buf() {
            Ok(available) => break answer_in(available),
            Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
            Err(error) => return Err(error),
        }
    };
    input.consume(taken);
    Ok(answer)
}

/// The answer at the start of `delivered` and how many bytes it used. In line mode a read of the
/// terminal returns one line, or the part of it typed before the end-of-file or interrupt
/// character. Only a line that ends with a newline is an answer: bytes without one are what the
/// person ended with Ctrl-D or Ctrl-C, which is the end of input, and nothing is empty input.
/// Bytes that are not UTF-8 become U+FFFD, which is never part of a name.
fn answer_in(delivered: &[u8]) -> (Answer, usize) {
    delivered
        .iter()
        .position(|byte| *byte == b'\n')
        .and_then(|at| delivered.get(..at).map(|line| (line, at + 1)))
        .map_or((Answer::Ended, delivered.len()), |(line, used)| {
            (
                Answer::Line(String::from_utf8_lossy(line).into_owned()),
                used,
            )
        })
}

/// The terminal's settings as they were, held while the interrupt key is made to end the line.
/// Dropping it puts them back, so no path leaves the terminal changed.
struct InterruptEndsTheLine<'a> {
    stdin: &'a Stdin,
    saved: Option<Termios>,
}

impl<'a> InterruptEndsTheLine<'a> {
    /// Saves the settings, then turns signal generation off and makes the interrupt character
    /// also the end-of-line character, so it ends the line it is typed in. Line editing and
    /// echo are untouched, and no input already typed is discarded.
    fn set(stdin: &'a Stdin) -> Result<Self, TerminalError> {
        let saved = tcgetattr(stdin)
            .map_err(io::Error::from)
            .map_err(TerminalError::ReadSettings)?;
        let mut changed = saved.clone();
        changed.local_modes.remove(LocalModes::ISIG);
        let interrupt = changed.special_codes[SpecialCodeIndex::VINTR];
        changed.special_codes[SpecialCodeIndex::VEOL] = interrupt;
        // From here the guard exists, so a change that half-succeeded is undone too.
        let guard = Self {
            stdin,
            saved: Some(saved),
        };
        tcsetattr(stdin, OptionalActions::Now, &changed)
            .map_err(io::Error::from)
            .map_err(TerminalError::ChangeSettings)?;
        Ok(guard)
    }

    /// Puts the settings back and says whether that worked.
    fn restore(mut self) -> Result<(), TerminalError> {
        self.saved.take().map_or(Ok(()), |saved| {
            tcsetattr(self.stdin, OptionalActions::Now, &saved)
                .map_err(io::Error::from)
                .map_err(TerminalError::RestoreSettings)
        })
    }
}

impl Drop for InterruptEndsTheLine<'_> {
    fn drop(&mut self) {
        // A destructor cannot report; the explicit `restore` is the path that does.
        if let Some(saved) = self.saved.take() {
            tcsetattr(self.stdin, OptionalActions::Now, &saved).unwrap_or_default();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_line_ends_at_its_newline_and_leaves_the_rest() {
        assert_eq!(
            answer_in(b"client-a\nnext\n"),
            (Answer::Line("client-a".to_owned()), 9)
        );
    }

    #[test]
    fn an_empty_line_is_an_answer() {
        assert_eq!(answer_in(b"\n"), (Answer::Line(String::new()), 1));
    }

    #[test]
    fn bytes_without_a_newline_are_the_end_of_input() {
        assert_eq!(answer_in(b"client-a"), (Answer::Ended, 8));
        assert_eq!(answer_in(b"client-a\x03"), (Answer::Ended, 9));
    }

    #[test]
    fn nothing_is_the_end_of_input() {
        assert_eq!(answer_in(b""), (Answer::Ended, 0));
    }

    #[test]
    fn bytes_that_are_not_text_never_make_a_name() {
        let (answer, used) = answer_in(b"client-\xff\n");
        assert_eq!(used, 9);
        assert_eq!(answer, Answer::Line("client-\u{fffd}".to_owned()));
    }
}
