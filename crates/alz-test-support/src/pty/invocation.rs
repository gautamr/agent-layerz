//! The command line of `pty-run`, parsed once into a value.

use std::ffi::OsString;
use std::time::Duration;

/// The deadline when the command line gives none: long enough for a slow machine, short enough
/// that a stuck program ends a test run rather than hanging it.
pub const DEFAULT_DEADLINE: Duration = Duration::from_secs(30);

/// What the person who started `pty-run` asked for: a program to run on a terminal, and how long
/// it may take.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Invocation {
    deadline: Duration,
    program: OsString,
    arguments: Vec<OsString>,
}

/// A command line that is not `[--deadline-ms N] [--] program [argument...]`.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum UsageError {
    /// No program was named.
    #[error("no program to run; usage: pty-run [--deadline-ms N] [--] program [argument...]")]
    NoProgram,
    /// `--deadline-ms` has no value or a value that is not a number of milliseconds.
    #[error("--deadline-ms takes a whole number of milliseconds")]
    BadDeadline,
}

impl Invocation {
    /// Parses the arguments after the program name.
    ///
    /// # Errors
    ///
    /// [`UsageError::NoProgram`] when no program is named, and [`UsageError::BadDeadline`] when
    /// `--deadline-ms` has no usable value.
    pub fn parse(arguments: impl IntoIterator<Item = OsString>) -> Result<Self, UsageError> {
        let mut rest = arguments.into_iter().peekable();
        let deadline = if rest
            .next_if(|argument| argument == "--deadline-ms")
            .is_some()
        {
            rest.next()
                .and_then(|value| value.into_string().ok())
                .and_then(|text| text.parse::<u64>().ok())
                .map(Duration::from_millis)
                .ok_or(UsageError::BadDeadline)?
        } else {
            DEFAULT_DEADLINE
        };
        // One optional `--` separates the options from the program, so a program named like an
        // option stays reachable.
        drop(rest.next_if(|argument| argument == "--"));
        let program = rest.next().ok_or(UsageError::NoProgram)?;
        Ok(Self {
            deadline,
            program,
            arguments: rest.collect(),
        })
    }

    /// How long the program may run before the driver kills it.
    #[must_use]
    pub fn deadline(&self) -> Duration {
        self.deadline
    }

    /// The program to run.
    #[must_use]
    pub fn program(&self) -> &OsString {
        &self.program
    }

    /// The program's arguments.
    #[must_use]
    pub fn arguments(&self) -> &[OsString] {
        &self.arguments
    }
}

#[cfg(test)]
mod tests {
    use super::{DEFAULT_DEADLINE, Invocation, UsageError};
    use std::ffi::OsString;
    use std::time::Duration;

    fn parse(words: &[&str]) -> Result<Invocation, UsageError> {
        Invocation::parse(words.iter().map(OsString::from))
    }

    #[test]
    fn takes_the_deadline_the_separator_the_program_and_its_arguments_in_order() {
        let invocation = parse(&["--deadline-ms", "250", "--", "sh", "-c", "true"]);
        assert_eq!(
            invocation.as_ref().map(Invocation::deadline),
            Ok(Duration::from_millis(250))
        );
        assert_eq!(
            invocation.as_ref().map(|i| i.program().clone()),
            Ok(OsString::from("sh"))
        );
        assert_eq!(
            invocation.map(|i| i.arguments().to_vec()),
            Ok(vec![OsString::from("-c"), OsString::from("true")])
        );
    }

    #[test]
    fn uses_the_default_deadline_and_needs_no_separator() {
        let invocation = parse(&["sh"]);
        assert_eq!(invocation.map(|i| i.deadline()), Ok(DEFAULT_DEADLINE));
    }

    #[test]
    fn leaves_an_option_after_the_program_to_the_program() {
        let invocation = parse(&["sh", "--deadline-ms", "5"]);
        assert_eq!(
            invocation.as_ref().map(Invocation::deadline),
            Ok(DEFAULT_DEADLINE)
        );
        assert_eq!(invocation.map(|i| i.arguments().len()), Ok(2));
    }

    #[test]
    fn refuses_a_missing_program_and_a_bad_deadline() {
        assert_eq!(parse(&[]), Err(UsageError::NoProgram));
        assert_eq!(parse(&["--deadline-ms", "5"]), Err(UsageError::NoProgram));
        assert_eq!(parse(&["--deadline-ms"]), Err(UsageError::BadDeadline));
        assert_eq!(
            parse(&["--deadline-ms", "soon", "sh"]),
            Err(UsageError::BadDeadline)
        );
        assert_eq!(
            parse(&["--deadline-ms", "-1", "sh"]),
            Err(UsageError::BadDeadline)
        );
    }
}
