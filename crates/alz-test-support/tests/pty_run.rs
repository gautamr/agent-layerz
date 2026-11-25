//! The `pty-run` driver, started as the acceptance suite starts it: with pipes for its standard
//! streams, a script of input on its standard input, and the program's own streams on the others.

#[cfg(test)]
mod tests {
    use std::io::{BufRead, BufReader, Read, Write};
    use std::process::{Child, ChildStdin, Command, Stdio};
    use std::sync::Mutex;
    use std::sync::mpsc::{self, Receiver};
    use std::thread;
    use std::time::{Duration, Instant};

    /// How long a test waits for a line the program should print.
    const PATIENCE: Duration = Duration::from_secs(20);

    /// Held while a test starts the driver. On macOS the standard library makes a child's pipes
    /// and only then marks them close-on-exec, so a driver another test starts at that moment
    /// would inherit them and hand them on to its program (one descriptor too many).
    static STARTING: Mutex<()> = Mutex::new(());

    /// A running driver: lines of the program's standard error arrive on `stderr`.
    struct Driver {
        child: Child,
        input: Option<ChildStdin>,
        stderr: Receiver<String>,
        stdout: Receiver<Vec<u8>>,
    }

    struct Finished {
        code: Option<i32>,
        stdout: String,
        stderr: String,
        elapsed: Duration,
    }

    fn start(options: &[&str], program: &[&str]) -> Driver {
        let mut child = {
            let _starting = STARTING
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            Command::new(env!("CARGO_BIN_EXE_pty-run"))
                .args(options)
                .arg("--")
                .args(program)
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .expect("the driver starts")
        };
        let input = child.stdin.take();
        let mut err = child.stderr.take().expect("stderr is piped");
        let mut out = child.stdout.take().expect("stdout is piped");
        let (lines, stderr) = mpsc::channel();
        thread::spawn(move || {
            for line in BufReader::new(&mut err).lines().map_while(Result::ok) {
                if lines.send(line).is_err() {
                    break;
                }
            }
        });
        let (bytes, stdout) = mpsc::channel();
        thread::spawn(move || {
            let mut all = Vec::new();
            out.read_to_end(&mut all).expect("stdout is read");
            bytes.send(all).expect("the test still listens");
        });
        Driver {
            child,
            input,
            stderr,
            stdout,
        }
    }

    impl Driver {
        fn type_in(&mut self, bytes: &[u8]) {
            let input = self.input.as_mut().expect("input is still open");
            input.write_all(bytes).expect("the driver reads its input");
            input.flush().expect("the input is flushed");
        }

        fn end_input(&mut self) {
            self.input = None;
        }

        fn wait_for_stderr_line(&self, expected: &str) {
            let line = self
                .stderr
                .recv_timeout(PATIENCE)
                .expect("a line of standard error arrives");
            assert_eq!(line, expected);
        }

        /// Every line of standard error the program printed, up to its end.
        fn finish(mut self) -> Finished {
            let started = Instant::now();
            drop(self.input.take());
            let status = self.child.wait().expect("the driver ends");
            let stdout = self.stdout.recv_timeout(PATIENCE).expect("stdout is read");
            // Every line, up to the end of the stream: the reader thread drops its sender at the end
            // of file, so this waits for the last line instead of taking only those already passed on.
            let stderr = self.stderr.iter().collect::<Vec<_>>().join("\n");
            Finished {
                code: status.code(),
                stdout: String::from_utf8(stdout).expect("stdout is text"),
                stderr,
                elapsed: started.elapsed(),
            }
        }
    }

    #[test]
    fn gives_a_program_a_terminal_as_standard_input_and_pipes_as_the_other_streams() {
        let finished = start(
            &[],
            &[
                "sh",
                "-c",
                "if [ -t 0 ]; then echo in:tty; else echo in:none; fi; \
                 if [ -t 1 ]; then echo out:tty; else echo out:pipe; fi; \
                 if [ -t 2 ]; then echo err:tty >&2; else echo err:pipe >&2; fi",
            ],
        )
        .finish();
        assert_eq!(finished.stdout, "in:tty\nout:pipe\n");
        assert_eq!(finished.stderr, "err:pipe");
        assert_eq!(finished.code, Some(0));
    }

    #[test]
    fn delivers_a_typed_line_to_a_program_that_reads_one() {
        let mut driver = start(
            &[],
            &["sh", "-c", "echo ready >&2; read line; echo \"got:$line\""],
        );
        driver.wait_for_stderr_line("ready");
        driver.type_in(b"hello there\n");
        let finished = driver.finish();
        assert_eq!(finished.stdout, "got:hello there\n");
        assert_eq!(finished.code, Some(0));
    }

    #[test]
    fn sends_the_end_of_input_as_the_end_of_the_terminal_file() {
        let mut driver = start(
            &[],
            &["sh", "-c", "echo ready >&2; read line || echo ended-by-eof"],
        );
        driver.wait_for_stderr_line("ready");
        driver.end_input();
        let finished = driver.finish();
        assert_eq!(finished.stdout, "ended-by-eof\n");
        assert_eq!(finished.code, Some(0));
    }

    #[test]
    fn delivers_ctrl_c_to_the_program_as_an_interrupt() {
        let mut driver = start(
            &[],
            &[
                "sh",
                "-c",
                "trap 'echo interrupted; exit 3' INT; echo ready >&2; while :; do sleep 0.05; done",
            ],
        );
        driver.wait_for_stderr_line("ready");
        driver.type_in(&[0x03]);
        let finished = driver.finish();
        assert_eq!(finished.stdout, "interrupted\n");
        assert_eq!(finished.code, Some(3));
    }

    #[test]
    fn an_uncaught_ctrl_c_ends_the_program_on_the_interrupt_signal() {
        let mut driver = start(&[], &["sh", "-c", "echo ready >&2; exec sleep 30"]);
        driver.wait_for_stderr_line("ready");
        driver.type_in(&[0x03]);
        let finished = driver.finish();
        assert_eq!(finished.code, Some(128 + 2));
    }

    #[test]
    fn kills_a_program_that_never_ends_at_the_deadline_and_everything_it_started() {
        let driver = start(
            &["--deadline-ms", "3000"],
            &["sh", "-c", "sleep 30 & echo $! >&2; wait"],
        );
        let sleeper = driver
            .stderr
            .recv_timeout(PATIENCE)
            .expect("the program prints the process it started");
        let finished = driver.finish();
        assert_eq!(finished.code, Some(124));
        assert!(
            finished.elapsed < Duration::from_secs(10),
            "{:?}",
            finished.elapsed
        );
        let still_there = Command::new("kill")
            .args(["-0", sleeper.trim()])
            .stderr(Stdio::null())
            .status()
            .expect("kill runs")
            .success();
        assert!(!still_there, "process {sleeper} survived the deadline");
    }

    #[test]
    fn hands_the_program_no_file_descriptor_beyond_the_three_standard_ones() {
        let finished = start(
            &[],
            &[
                "sh",
                "-c",
                "n=3; while [ $n -lt 20 ]; do [ -e /dev/fd/$n ] && echo open:$n; n=$((n + 1)); done; echo checked",
            ],
        )
        .finish();
        assert_eq!(finished.stdout, "checked\n");
    }

    #[test]
    fn reports_the_programs_exit_code() {
        let finished = start(&[], &["sh", "-c", "exit 7"]).finish();
        assert_eq!(finished.code, Some(7));
    }

    /// One run in which the input closes while the program is still ending.
    fn run_with_the_input_closing_before_the_program_ends() -> Option<i32> {
        let mut driver = start(
            &[],
            &[
                "sh",
                "-c",
                "echo ready >&2; read line; echo typed >&2; sleep 0.1; exit 5",
            ],
        );
        driver.wait_for_stderr_line("ready");
        driver.type_in(b"go\n");
        driver.wait_for_stderr_line("typed");
        driver.finish().code
    }

    #[test]
    fn reports_the_exit_code_when_the_input_closes_before_the_program_has_ended() {
        // The test closes the driver's input while the program is still ending, the order in which
        // a test finishes a run, and several drivers run at once so that the driver's own exit
        // races the end of its terminal. The last close of the driver's end of the terminal hangs
        // it up, which on Linux ended the driver by a signal if it came before the driver exited.
        let codes: Vec<Option<i32>> = thread::scope(|scope| {
            let runs: Vec<_> = (0..16)
                .map(|_| {
                    scope.spawn(|| {
                        (0..5)
                            .map(|_| run_with_the_input_closing_before_the_program_ends())
                            .collect::<Vec<_>>()
                    })
                })
                .collect();
            runs.into_iter()
                .flat_map(|run| run.join().expect("the runs end"))
                .collect()
        });
        assert!(
            codes.iter().all(|code| *code == Some(5)),
            "exit codes: {codes:?}"
        );
    }

    #[test]
    fn answers_a_usage_error_and_a_missing_program_with_the_driver_error_code() {
        // It exits at once with a usage error, so holding the lock for the whole run costs nothing.
        let none = {
            let _starting = STARTING
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            Command::new(env!("CARGO_BIN_EXE_pty-run"))
                .stdin(Stdio::null())
                .output()
                .expect("the driver starts")
        };
        assert_eq!(none.status.code(), Some(125));
        assert!(String::from_utf8_lossy(&none.stderr).contains("no program to run"));
        let missing = start(&[], &["/nonexistent/program"]).finish();
        assert_eq!(missing.code, Some(125));
        assert!(missing.stderr.contains("cannot start the program"));
    }
}
