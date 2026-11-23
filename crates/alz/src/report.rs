//! Renders what clap produced: help and version text on standard output, usage errors on
//! standard error, each sanitised (decision-26), and the exit code that goes with them.
//!
//! clap echoes the arguments it rejects, and an argument is untrusted text. The values clap
//! keeps as the context of an error are therefore sanitised fragment by fragment, line feeds
//! included, before clap renders them: the only line feeds left in the text are clap's own, so
//! no argument can start a line of its own, such as a forged diagnostic.

use std::io::{self, Write};

use alz_core::Diagnostics;
use clap::Error;
use clap::builder::StyledStr;
use clap::error::{ContextKind, ContextValue};

use crate::terminal_text::{strip_untrusted, strip_untrusted_keeping_line_feeds};

/// Exit code of a successful run: help and version text were printed (decision-26).
const SUCCESS: u8 = 0;

/// Exit code of a run that could not do what was asked, here: the output could not be written
/// (a closed pipe, a full disk). decision-26 reserves 70 for an internal failure, a bug in alz;
/// an unwritable output is a condition of the environment, so it is an ordinary failure.
const WRITE_FAILED: u8 = 1;

/// Exit code of a usage error (decision-26).
const USAGE_ERROR: u8 = 2;

/// Exit code of a run that reports at least one error diagnostic (decision-26).
const FAILED: u8 = 1;

/// Writes every diagnostic of a run to standard error, one line each, and returns the exit code
/// they give: 0 when none is an error, 1 when any is, so warnings alone do not fail a command
/// (spec §10.1). When standard error cannot be written there is nowhere left to say so; the exit
/// code is [`WRITE_FAILED`] then.
pub fn emit_diagnostics(diagnostics: &Diagnostics, stderr: &mut impl Write) -> u8 {
    match write_through(stderr, &diagnostics.to_string()) {
        Ok(()) if diagnostics.has_errors() => FAILED,
        Ok(()) => SUCCESS,
        Err(_) => WRITE_FAILED,
    }
}

/// The standard stream clap's output belongs on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stream {
    Stdout,
    Stderr,
}

/// What to print for a clap result, and how the run ends.
#[derive(Debug, PartialEq, Eq)]
pub struct Rendered {
    stream: Stream,
    text: String,
    exit_code: u8,
}

impl Rendered {
    /// The sanitised rendering of clap's help, version text or usage error.
    pub fn of(error: Error) -> Self {
        let (stream, exit_code) = if error.use_stderr() {
            (Stream::Stderr, USAGE_ERROR)
        } else {
            (Stream::Stdout, SUCCESS)
        };
        let error = sanitise_context(error);
        Self {
            stream,
            text: strip_untrusted_keeping_line_feeds(&error.render().to_string()),
            exit_code,
        }
    }

    /// Writes the text to its stream and returns the exit code. When the write fails, the failure
    /// is reported on standard error, as far as that stream works, and the exit code is
    /// [`WRITE_FAILED`].
    pub fn emit(&self, stdout: &mut impl Write, stderr: &mut impl Write) -> u8 {
        let written = match self.stream {
            Stream::Stdout => write_through(stdout, &self.text),
            Stream::Stderr => write_through(stderr, &self.text),
        };
        match written {
            Ok(()) => self.exit_code,
            Err(failure) => {
                // If standard error is the stream that failed, there is nowhere left to say so;
                // the exit code still does.
                writeln!(stderr, "alz: cannot write its output: {failure}").unwrap_or_default();
                WRITE_FAILED
            }
        }
    }
}

/// Writes all of the text and flushes, so a failure of buffered output shows up here and not at
/// the end of the process.
fn write_through(stream: &mut impl Write, text: &str) -> io::Result<()> {
    stream.write_all(text.as_bytes())?;
    stream.flush()
}

/// The error with every untrusted value in its context stripped of control, line-separator and
/// format characters. The usage text is clap's own, built from the command definition.
fn sanitise_context(error: Error) -> Error {
    let cleaned: Vec<(ContextKind, ContextValue)> = error
        .context()
        .filter(|(kind, _)| *kind != ContextKind::Usage)
        .map(|(kind, value)| (kind, strip_value(value)))
        .collect();
    cleaned.into_iter().fold(error, |mut error, (kind, value)| {
        error.insert(kind, value);
        error
    })
}

/// The context value with the characters an untrusted fragment must not carry removed.
fn strip_value(value: &ContextValue) -> ContextValue {
    match value {
        ContextValue::String(text) => ContextValue::String(strip_untrusted(text)),
        ContextValue::Strings(texts) => {
            ContextValue::Strings(texts.iter().map(|text| strip_untrusted(text)).collect())
        }
        ContextValue::StyledStr(text) => ContextValue::StyledStr(strip_styled(text)),
        ContextValue::StyledStrs(texts) => {
            ContextValue::StyledStrs(texts.iter().map(strip_styled).collect())
        }
        ContextValue::None | ContextValue::Bool(_) | ContextValue::Number(_) => value.clone(),
        // `ContextValue` is non-exhaustive: a variant added by a later clap is sanitised through
        // its text form, never passed on as it is.
        other => ContextValue::String(strip_untrusted(&other.to_string())),
    }
}

/// The plain text of a styled string, sanitised. clap's own styling is dropped: the output is
/// rendered without it (decision-26).
fn strip_styled(text: &StyledStr) -> StyledStr {
    StyledStr::from(strip_untrusted(&text.to_string()))
}

#[cfg(test)]
mod tests {
    use std::io::{self, Write};

    use alz_core::{Code, Diagnostic, Diagnostics, Message};
    use clap::{CommandFactory, Parser};

    use super::{Rendered, Stream, emit_diagnostics};
    use crate::cli::Cli;

    /// A stream that fails every write, as a closed pipe does.
    struct Broken;

    impl Write for Broken {
        fn write(&mut self, _bytes: &[u8]) -> io::Result<usize> {
            Err(io::ErrorKind::BrokenPipe.into())
        }

        fn flush(&mut self) -> io::Result<()> {
            Err(io::ErrorKind::BrokenPipe.into())
        }
    }

    fn rendered_for(arguments: &[&str]) -> Rendered {
        let error = Cli::try_parse_from(arguments).expect_err("the arguments are rejected");
        Rendered::of(error)
    }

    #[test]
    fn a_usage_error_belongs_on_stderr_with_exit_code_two() {
        let rendered = rendered_for(&["alz", "--no-such-flag"]);
        assert_eq!(rendered.stream, Stream::Stderr);
        assert_eq!(rendered.exit_code, 2);
    }

    #[test]
    fn help_belongs_on_stdout_with_exit_code_zero() {
        let rendered = rendered_for(&["alz", "--help"]);
        assert_eq!(rendered.stream, Stream::Stdout);
        assert_eq!(rendered.exit_code, 0);
        assert!(rendered.text.contains("Usage: alz"));
    }

    #[test]
    fn a_line_feed_in_an_argument_never_starts_a_line() {
        let forged = "--x\nerror [fake] layer.yaml: forged";
        let rendered = rendered_for(&["alz", forged]);
        assert!(
            rendered
                .text
                .lines()
                .all(|line| !line.starts_with("error [fake]")),
            "text: {:?}",
            rendered.text
        );
        assert!(rendered.text.contains("--xerror [fake] layer.yaml: forged"));
    }

    #[test]
    fn a_line_separator_in_an_argument_is_removed() {
        let rendered = rendered_for(&["alz", "--x\u{2028}y\u{2029}z"]);
        assert!(rendered.text.contains("--xyz"), "text: {:?}", rendered.text);
    }

    #[test]
    fn escape_sequences_and_carriage_returns_in_an_argument_are_removed() {
        let rendered = rendered_for(&["alz", "--vers\u{1b}[31mion\r\n"]);
        assert!(
            rendered
                .text
                .contains("unexpected argument '--vers[31mion' found"),
            "text: {:?}",
            rendered.text
        );
        assert!(
            !rendered.text.contains(['\u{1b}', '\r']),
            "text: {:?}",
            rendered.text
        );
    }

    #[test]
    fn the_command_definition_still_renders_its_own_line_structure() {
        let usage = Cli::command().render_usage().to_string();
        let rendered = rendered_for(&["alz", "--no-such-flag"]);
        assert!(rendered.text.contains(&usage), "text: {:?}", rendered.text);
    }

    #[test]
    fn a_failed_write_exits_one_and_says_so_on_stderr() {
        let rendered = rendered_for(&["alz", "--help"]);
        let mut stderr = Vec::new();
        let code = rendered.emit(&mut Broken, &mut stderr);
        assert_eq!(code, 1);
        assert_eq!(
            String::from_utf8_lossy(&stderr),
            "alz: cannot write its output: broken pipe\n"
        );
    }

    #[test]
    fn a_failed_write_to_stderr_still_exits_one() {
        let rendered = rendered_for(&["alz", "--no-such-flag"]);
        assert_eq!(rendered.emit(&mut Vec::new(), &mut Broken), 1);
    }

    #[test]
    fn a_successful_write_returns_the_exit_code_of_the_rendering() {
        let rendered = rendered_for(&["alz", "--no-such-flag"]);
        let (mut stdout, mut stderr) = (Vec::new(), Vec::new());
        assert_eq!(rendered.emit(&mut stdout, &mut stderr), 2);
        assert_eq!(String::from_utf8_lossy(&stdout), "");
        assert_eq!(String::from_utf8_lossy(&stderr), rendered.text);
    }

    #[test]
    fn diagnostics_with_an_error_go_to_stderr_one_line_each_and_exit_one() {
        let diagnostics = Diagnostics::new()
            .with(Diagnostic::warning(Code::Lock, None, "stale".into()))
            .with(Diagnostic::error(Code::Yaml, None, Message::new("bad")));
        let mut stderr = Vec::new();
        assert_eq!(emit_diagnostics(&diagnostics, &mut stderr), 1);
        assert_eq!(
            String::from_utf8_lossy(&stderr),
            "warning [lock] stale\nerror [yaml] bad\n"
        );
    }

    #[test]
    fn warnings_alone_exit_zero() {
        let diagnostics =
            Diagnostics::new().with(Diagnostic::warning(Code::Lock, None, "x".into()));
        assert_eq!(emit_diagnostics(&diagnostics, &mut Vec::new()), 0);
    }

    #[test]
    fn diagnostics_that_cannot_be_written_exit_one() {
        let diagnostics =
            Diagnostics::new().with(Diagnostic::warning(Code::Lock, None, "x".into()));
        assert_eq!(emit_diagnostics(&diagnostics, &mut Broken), 1);
    }
}
