//! `PersonAtTerminal` (spec §7.13 "Trust and `alz account allow`", TASK-55.7.11), driven through
//! a pseudo-terminal with the `pty-run` driver of `alz-test-support` (TASK-55.7.1).
//!
//! Each test starts this test executable again, under `pty-run` or on a pipe, with an environment
//! variable that makes `the_child_that_asks` run: it checks for a person, asks one question for
//! each name it is given, and prints what happened to standard output. The terminal's settings are
//! read back through `/dev/tty`, which under `pty-run` is the pseudo-terminal, so this file never
//! touches standard input.

// The tests sit in a `mod tests` so that `clippy::tests_outside_test_module` holds for this file
// as for every other one.
#[cfg(test)]
mod tests {
    use std::ffi::OsString;
    use std::fs::File;
    use std::io::{Read, Write};
    use std::process::{Child, ChildStdin, Command, Stdio};
    use std::sync::mpsc::{self, Receiver};
    use std::thread::{self, JoinHandle};
    use std::time::Duration;

    use alz_core::account::{AccountName, AgentSessionMarker, Answer, Confirmation};
    use alz_io::{EnvSnapshot, PersonAtTerminal};
    use alz_test_support::{pty_run_executable, test_executable};
    use rustix::termios::tcgetattr;

    /// The names the child asks for, comma separated; unset in the tests of the whole file.
    const NAMES: &str = "ALZ_TERMINAL_TEST_NAMES";
    /// How long a test waits for the child to print what it should.
    const PATIENCE: Duration = Duration::from_secs(20);
    const CTRL_C: &[u8] = b"\x03";

    #[expect(
        clippy::disallowed_methods,
        reason = "the re-run of this test binary receives its names through its environment"
    )]
    fn names_to_ask() -> Option<OsString> {
        std::env::var_os(NAMES)
    }

    /// The terminal's settings, as text that two reads compare.
    fn settings_of_the_terminal() -> String {
        let tty = File::options()
            .read(true)
            .write(true)
            .open("/dev/tty")
            .expect("the child has a controlling terminal");
        let settings = tcgetattr(&tty).expect("the settings of the terminal");
        format!(
            "{:?} {:?} {:?} {:?} {:?}",
            settings.input_modes,
            settings.output_modes,
            settings.control_modes,
            settings.local_modes,
            settings.special_codes
        )
    }

    /// Not a test of its own: the child of the tests below. Without `NAMES` it does nothing.
    #[test]
    fn the_child_that_asks() {
        let Some(names) = names_to_ask() else { return };
        let person = match PersonAtTerminal::check(&EnvSnapshot::capture()) {
            Ok(person) => person,
            Err(refused) => {
                let markers = refused.agent_session().map_or_default(|session| {
                    session
                        .markers()
                        .map(AgentSessionMarker::name)
                        .collect::<Vec<_>>()
                        .join(",")
                });
                println!(
                    "refused: terminal={} markers={markers}",
                    !refused.lacks_terminal()
                );
                return;
            }
        };
        let mut person = person;
        let before = settings_of_the_terminal();
        for (index, name) in names.to_str().expect("text").split(',').enumerate() {
            let asked = AccountName::try_from(name).expect("a name");
            let answer = person
                .ask(&format!("Type {name}: "))
                .expect("the question is asked");
            let confirmed = Confirmation::of(&asked, &answer);
            println!("answer {}: {answer:?} {confirmed:?}", index + 1);
            println!(
                "settings restored after {}: {}",
                index + 1,
                settings_of_the_terminal() == before
            );
            if !confirmed.is_confirmed() {
                println!("stopped");
                return;
            }
        }
        println!("done");
    }

    /// The child running, with the driver's or the pipe's streams read on threads.
    struct Running {
        process: Child,
        input: Option<ChildStdin>,
        stderr: Receiver<Vec<u8>>,
        stderr_so_far: String,
        stdout: JoinHandle<String>,
    }

    struct Finished {
        code: Option<i32>,
        stdout: String,
        stderr: String,
    }

    /// Without any agent-session marker, which this test run may itself have set.
    fn clean(command: &mut Command) -> &mut Command {
        AgentSessionMarker::ALL
            .into_iter()
            .fold(command, |command, marker| command.env_remove(marker.name()))
    }

    fn spawn(mut command: Command, names: &str, markers: &[(&str, &str)]) -> Running {
        clean(&mut command)
            .envs(markers.iter().copied())
            .env(NAMES, names)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let mut process = command.spawn().expect("the child starts");
        let input = process.stdin.take();
        let mut stdout = process.stdout.take().expect("stdout is piped");
        let mut stderr = process.stderr.take().expect("stderr is piped");
        let (chunks, received) = mpsc::channel();
        thread::spawn(move || {
            let mut chunk = [0_u8; 4096];
            while let Ok(read) = stderr.read(&mut chunk) {
                let Some(bytes) = chunk.get(..read).filter(|bytes| !bytes.is_empty()) else {
                    break;
                };
                if chunks.send(bytes.to_vec()).is_err() {
                    break;
                }
            }
        });
        Running {
            process,
            input,
            stderr: received,
            stderr_so_far: String::new(),
            stdout: thread::spawn(move || {
                let mut all = String::new();
                stdout.read_to_string(&mut all).expect("text");
                all
            }),
        }
    }

    /// The child under a pseudo-terminal, as a person's terminal.
    fn on_terminal(names: &str, markers: &[(&str, &str)]) -> Running {
        let mut command = Command::new(pty_run_executable().expect("the driver is built"));
        command
            .args(["--deadline-ms", "60000", "--"])
            .arg(test_executable().expect("the test executable"))
            .args(["--exact", "tests::the_child_that_asks", "--nocapture"]);
        spawn(command, names, markers)
    }

    impl Running {
        fn wait_for_stderr(&mut self, needle: &str) {
            while !self.stderr_so_far.contains(needle) {
                let chunk = self.stderr.recv_timeout(PATIENCE).unwrap_or_else(|_| {
                    panic!("no {needle:?} on stderr: {:?}", self.stderr_so_far)
                });
                self.stderr_so_far
                    .push_str(&String::from_utf8_lossy(&chunk));
            }
        }

        fn type_in(&mut self, bytes: &[u8]) {
            let input = self.input.as_mut().expect("the input is open");
            input.write_all(bytes).expect("the driver reads");
            input.flush().expect("flushed");
        }

        /// Closes what the driver reads: it types the terminal's end-of-file character.
        fn end_input(&mut self) {
            self.input = None;
        }

        fn finish(mut self) -> Finished {
            self.input = None;
            let code = self.process.wait().expect("the child ends").code();
            let rest: Vec<Vec<u8>> = self.stderr.iter().collect();
            for chunk in rest {
                self.stderr_so_far
                    .push_str(&String::from_utf8_lossy(&chunk));
            }
            Finished {
                code,
                stdout: self.stdout.join().expect("the reader ends"),
                stderr: self.stderr_so_far,
            }
        }
    }

    fn restored_every_time(finished: &Finished, answers: usize) -> bool {
        (1..=answers).all(|n| {
            finished
                .stdout
                .contains(&format!("settings restored after {n}: true"))
        })
    }

    #[test]
    fn asks_on_stderr_and_reads_one_answer_for_each_prompt() {
        let mut child = on_terminal("client-a,client-a-api", &[]);
        child.wait_for_stderr("Type client-a: ");
        child.type_in(b" client-a\t\n");
        child.wait_for_stderr("Type client-a-api: ");
        child.type_in(b"client-a-api  \n");
        let finished = child.finish();
        assert_eq!(finished.code, Some(0), "{}", finished.stderr);
        assert!(
            finished
                .stdout
                .contains("answer 1: Answer::Line(..) Confirmed"),
            "{}",
            finished.stdout
        );
        assert!(
            finished
                .stdout
                .contains("answer 2: Answer::Line(..) Confirmed")
        );
        assert!(finished.stdout.contains("done"));
        assert!(restored_every_time(&finished, 2), "{}", finished.stdout);
        assert!(
            !finished.stdout.contains("client-a"),
            "the answer is never printed: {}",
            finished.stdout
        );
    }

    #[test]
    fn answers_typed_before_their_question_are_read_by_the_question_they_answer() {
        let mut child = on_terminal("client-a,client-a-api", &[]);
        child.wait_for_stderr("Type client-a: ");
        child.type_in(b"client-a\nclient-a-api\n");
        let finished = child.finish();
        assert_eq!(finished.code, Some(0), "{}", finished.stderr);
        assert!(
            finished
                .stdout
                .contains("answer 1: Answer::Line(..) Confirmed")
        );
        assert!(
            finished
                .stdout
                .contains("answer 2: Answer::Line(..) Confirmed")
        );
        assert!(restored_every_time(&finished, 2), "{}", finished.stdout);
    }

    #[test]
    fn an_answer_that_is_not_the_name_is_no_and_the_settings_are_restored() {
        for wrong in [
            "y",
            "",
            "client-b",
            "CLIENT-A",
            "client-a now",
            "client-a-api",
        ] {
            let mut child = on_terminal("client-a", &[]);
            child.wait_for_stderr("Type client-a: ");
            child.type_in(format!("{wrong}\n").as_bytes());
            let finished = child.finish();
            assert!(
                finished
                    .stdout
                    .contains("answer 1: Answer::Line(..) NotConfirmed"),
                "{wrong:?}: {}",
                finished.stdout
            );
            assert!(restored_every_time(&finished, 1), "{}", finished.stdout);
        }
    }

    #[test]
    fn ctrl_c_answers_no_without_a_signal_and_the_settings_are_restored() {
        let mut child = on_terminal("client-a,client-a-api", &[]);
        child.wait_for_stderr("Type client-a: ");
        child.type_in(CTRL_C);
        let finished = child.finish();
        // The child ended by itself: a signal would show as 130 and print nothing.
        assert_eq!(finished.code, Some(0), "{}", finished.stderr);
        assert!(
            finished
                .stdout
                .contains("answer 1: Answer::Ended NotConfirmed"),
            "{}",
            finished.stdout
        );
        assert!(restored_every_time(&finished, 1), "{}", finished.stdout);
        assert!(
            !finished.stdout.contains("client-a-api"),
            "no second question"
        );
        assert!(!finished.stderr.contains("Type client-a-api"));
    }

    #[test]
    fn ctrl_c_after_the_name_is_still_no() {
        let mut child = on_terminal("client-a", &[]);
        child.wait_for_stderr("Type client-a: ");
        child.type_in(b"client-a\x03");
        let finished = child.finish();
        assert_eq!(finished.code, Some(0), "{}", finished.stderr);
        assert!(
            finished
                .stdout
                .contains("answer 1: Answer::Ended NotConfirmed"),
            "{}",
            finished.stdout
        );
        assert!(restored_every_time(&finished, 1), "{}", finished.stdout);
    }

    #[test]
    fn the_end_of_input_answers_no_and_the_settings_are_restored() {
        let mut child = on_terminal("client-a", &[]);
        child.wait_for_stderr("Type client-a: ");
        child.end_input();
        let finished = child.finish();
        assert_eq!(finished.code, Some(0), "{}", finished.stderr);
        assert!(
            finished
                .stdout
                .contains("answer 1: Answer::Ended NotConfirmed"),
            "{}",
            finished.stdout
        );
        assert!(restored_every_time(&finished, 1), "{}", finished.stdout);
    }

    #[test]
    fn the_name_without_a_newline_before_the_end_of_input_is_no() {
        let mut child = on_terminal("client-a", &[]);
        child.wait_for_stderr("Type client-a: ");
        child.type_in(b"client-a");
        child.end_input();
        let finished = child.finish();
        assert!(
            finished
                .stdout
                .contains("answer 1: Answer::Ended NotConfirmed"),
            "{}",
            finished.stdout
        );
    }

    #[test]
    fn standard_input_that_is_not_a_terminal_is_refused_before_anything_is_read() {
        let mut command = Command::new(test_executable().expect("the test executable"));
        command.args(["--exact", "tests::the_child_that_asks", "--nocapture"]);
        let mut child = spawn(command, "client-a", &[]);
        // The child may have ended already, which closes the pipe; what is typed is not the point.
        if let Some(input) = child.input.as_mut() {
            input.write_all(b"client-a\n").unwrap_or_default();
        }
        let finished = child.finish();
        assert_eq!(finished.code, Some(0), "{}", finished.stderr);
        assert!(
            finished
                .stdout
                .contains("refused: terminal=false markers=\n"),
            "{}",
            finished.stdout
        );
        assert!(!finished.stdout.contains("answer"));
        assert!(
            !finished.stderr.contains("Type client-a"),
            "nothing was asked"
        );
    }

    #[test]
    fn a_marker_refuses_even_when_empty_and_is_named_not_valued() {
        let mut child = on_terminal(
            "client-a",
            &[("CODEX_CI", "secret-value"), ("CLAUDECODE", "")],
        );
        child.end_input();
        let finished = child.finish();
        assert!(
            finished
                .stdout
                .contains("refused: terminal=true markers=CLAUDECODE,CODEX_CI\n"),
            "{}",
            finished.stdout
        );
        assert!(!finished.stdout.contains("secret-value"));
        assert!(
            !finished.stderr.contains("Type client-a"),
            "nothing was asked"
        );
    }

    #[test]
    fn every_marker_alone_refuses_a_terminal() {
        for marker in AgentSessionMarker::ALL {
            let mut child = on_terminal("client-a", &[(marker.name(), "")]);
            child.end_input();
            let finished = child.finish();
            assert!(
                finished.stdout.contains(&format!(
                    "refused: terminal=true markers={}\n",
                    marker.name()
                )),
                "{}: {}",
                marker.name(),
                finished.stdout
            );
        }
    }

    #[test]
    fn a_pipe_and_a_marker_name_both_reasons() {
        let mut command = Command::new(test_executable().expect("the test executable"));
        command.args(["--exact", "tests::the_child_that_asks", "--nocapture"]);
        let mut child = spawn(command, "client-a", &[("AI_AGENT", "1")]);
        child.end_input();
        let finished = child.finish();
        assert!(
            finished
                .stdout
                .contains("refused: terminal=false markers=AI_AGENT\n"),
            "{}",
            finished.stdout
        );
    }

    #[test]
    fn answers_are_never_shown_by_debug() {
        assert_eq!(
            format!("{:?}", Answer::Line("client-a".to_owned())),
            "Answer::Line(..)"
        );
    }
}
