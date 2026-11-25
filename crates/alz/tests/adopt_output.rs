//! What `alz build --adopt` prints (spec §10.3 "`alz build`"): `adopted <from> (nothing left to
//! keep)`, the problems of a plan with their fixed words, native text stripped, a collision that
//! adopting leaves, and `nothing was adopted; fix the problems above and rerun` with the
//! repository unchanged. Each run has its own `HOME` and `XDG_*` folders.

// The tests sit in a `mod tests` so that `clippy::tests_outside_test_module` holds for this file
// as for every other one.
#[cfg(test)]
mod tests {
    use std::path::Path;
    use std::time::Duration;

    use alz_test_support::{ScratchDir, read_tree};
    use assert_cmd::{Command, cargo::cargo_bin_cmd};

    const NOTHING_ADOPTED: &str = "nothing was adopted; fix the problems above and rerun\n";

    struct Run {
        code: i32,
        stdout: String,
        stderr: String,
    }

    /// A repository whose repo layer is `.layer`, with `files` (paths from the repository root)
    /// planted besides.
    fn repository(files: &[(&str, &str)]) -> ScratchDir {
        let scratch = ScratchDir::new().expect("a scratch directory");
        for folder in ["home", "config", "state", "cache"] {
            scratch.make_dirs(folder).expect("made");
        }
        let base = [
            (
                "layer.yaml",
                "stack:\n  - source: path:./.layer\ntargets: [claude]\n",
            ),
            (
                ".layer/layer.yaml",
                "name: shop-api\nkind: repo\nvisibility: private\n",
            ),
        ];
        for (path, text) in base.iter().chain(files) {
            let path = Path::new("repo").join(path);
            if let Some(parent) = path.parent() {
                scratch.make_dirs(parent).expect("made");
            }
            scratch.write(&path, text).expect("written");
        }
        scratch
    }

    fn adopt(scratch: &ScratchDir) -> Run {
        let mut command: Command = cargo_bin_cmd!("alz");
        let output = command
            .current_dir(scratch.join("repo"))
            .env_clear()
            .env("PATH", "/usr/bin:/bin")
            .env("HOME", scratch.join("home"))
            .env("XDG_CONFIG_HOME", scratch.join("config"))
            .env("XDG_STATE_HOME", scratch.join("state"))
            .env("XDG_CACHE_HOME", scratch.join("cache"))
            .timeout(Duration::from_secs(60))
            .args(["build", "--adopt"])
            .output()
            .expect("alz runs and ends in time");
        Run {
            code: output.status.code().expect("an exit code"),
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        }
    }

    /// The stderr lines of a refused plan: exit 1, nothing on standard output, the repository as
    /// it was, and the closing line last.
    fn problems_of(files: &[(&str, &str)]) -> Vec<String> {
        let scratch = repository(files);
        let before = read_tree(&scratch.join("repo")).expect("read");
        let run = adopt(&scratch);
        assert_eq!((run.code, run.stdout.as_str()), (1, ""), "{}", run.stderr);
        assert_eq!(read_tree(&scratch.join("repo")).expect("read"), before);
        let mut lines: Vec<String> = run.stderr.lines().map(str::to_owned).collect();
        assert_eq!(
            lines.pop().map(|line| line + "\n"),
            Some(NOTHING_ADOPTED.to_owned())
        );
        lines
    }

    const RULE: &str = "---\npaths: [\"src/**\"]\n---\nbody\n";
    const AGENT: &str = "---\nname: a\ndescription: d\n---\nb\n";

    /// Spec §10.3 `alz build`: a native file of which nothing is kept prints
    /// `adopted <from> (nothing left to keep)`, and the build runs again.
    #[test]
    fn a_native_file_with_nothing_to_keep_is_adopted_as_nothing_left_to_keep() {
        for (path, text) in [
            (".claude/settings.json", "{}\n"),
            (".mcp.json", "{\"mcpServers\":{}}\n"),
        ] {
            let scratch = repository(&[(path, text)]);
            let run = adopt(&scratch);
            assert_eq!(
                (run.code, run.stdout.as_str(), run.stderr.as_str()),
                (
                    0,
                    format!(
                        "adopted {path} (nothing left to keep)\nbuilt 2 files for claude: 2 created, 0 updated, 0 deleted (output visibility: private)\n"
                    )
                    .as_str(),
                    ""
                ),
                "{path}"
            );
        }
    }

    /// Spec §10.3 `alz build`: the problems of a plan whose words the spec gives, each as
    /// `error [adopt] <file>: <problem>` before `nothing was adopted; …`.
    #[test]
    fn a_plan_problem_is_one_error_adopt_line_with_the_words_of_the_spec() {
        assert_eq!(
            problems_of(&[(".claude/agents/nested/x.md", "x\n")]),
            [
                "error [adopt] .claude/agents/nested/x.md: cannot be adopted automatically; move it into a layer by hand"
            ]
        );
        assert_eq!(
            problems_of(&[(
                ".claude/agents/a.md",
                "---\nname: a\ndescription: d\ntools: [Foo]\n---\nb\n"
            )]),
            [
                "error [adopt] .claude/agents/a.md: tools entry 'Foo' has no neutral form; set it by hand"
            ]
        );
        assert_eq!(
            problems_of(&[(".claude/rules/r.md", RULE), (".layer/rules/r.md", RULE)]),
            ["error [adopt] .claude/rules/r.md: .layer/rules/r.md already exists"]
        );
    }

    /// Spec §10.3 `alz build`: text of the native file that a problem repeats is stripped
    /// (class 1), so an escape character of a rule or a variable name never reaches the terminal.
    #[test]
    fn native_text_in_a_plan_problem_is_stripped_of_control_characters() {
        let rule = problems_of(&[(
            ".claude/settings.json",
            "{\"permissions\":{\"allow\":[\"Foo\\u001b[31m(x)\"]}}\n",
        )]);
        let variable = problems_of(&[(
            ".mcp.json",
            "{\"mcpServers\":{\"db\":{\"command\":\"x\",\"env\":{\"TO\\u001bKEN\":\"abc\"}}}}\n",
        )]);
        let lines = rule.iter().chain(&variable);
        assert!(
            lines
                .clone()
                .all(|line| line.chars().all(|c| !c.is_control())),
            "{rule:?} {variable:?}"
        );
        assert!(
            rule.iter().any(|line| line.contains("'Foo[31m(x)'")),
            "{rule:?}"
        );
        assert!(
            variable.iter().any(|line| line.contains("'TOKEN'")),
            "{variable:?}"
        );
    }

    /// Spec §10.3 `alz build`: a collision that adopting does not remove (a file that shadows the
    /// generated `AGENTS.md`, which no layer can hold) is an error, whatever else could be adopted:
    /// exit 1, the collision on standard error, and nothing adopted.
    #[test]
    fn a_collision_that_adopting_cannot_remove_is_still_an_error() {
        let scratch = repository(&[
            (".claude/agents/a.md", AGENT),
            ("AGENTS.override.md", "x\n"),
        ]);
        let before = read_tree(&scratch.join("repo")).expect("read");
        let run = adopt(&scratch);
        assert_eq!((run.code, run.stdout.as_str()), (1, ""), "{}", run.stderr);
        assert!(
            run.stderr
                .lines()
                .any(|line| line.starts_with("error [collision] AGENTS.override.md: ")),
            "{}",
            run.stderr
        );
        assert_eq!(read_tree(&scratch.join("repo")).expect("read"), before);
    }
}
