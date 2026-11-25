//! The coverage matrix `docs/spec-coverage.md` against the spec and the tests (TASK-55.9.5,
//! decision-39). The spec, `docs/LAYERING-SPEC.md`, is the contract. The matrix has one row for
//! each anchor of the spec and says which Rust tests prove it, so that `test/` may go only when no
//! requirement lacks a test.
//!
//! An anchor is one of these, outside code fences:
//!
//! 1. a `##` or `###` heading, written `§<number>` with the number of the heading (`§4.3`);
//! 2. a paragraph that opens with a bold lead, a line that starts with `**`, written
//!    `§<number> <lead>` with the number of the heading it stands under (`§0` before the first
//!    heading) and the text between the first two `**`, as written (`§4.3 Writing.`);
//! 3. a numbered item of section 13, a line that starts with `<n>. `, written `§13.<n>`.
//!
//! Each row is `covered` (tests named as `path::test_name`, which must exist), `n/a` (a reason),
//! `planned` (a task ID, for behaviour the spec marks as not built) or `gap` (the behaviour that
//! lacks a test). The guard fails on a missing row, a stale row, a duplicate row, a status that is
//! not one of the four, a test that does not exist, and a summary that is out of date.

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};
    use std::fs;
    use std::path::{Path, PathBuf};

    /// The workspace root: this crate lives in `crates/alz-test-support`.
    fn root() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
    }

    /// The heading number of a `##` or `###` line, such as `4.3` for `### 4.3 Repo-level files`.
    fn heading_number(line: &str) -> Option<String> {
        let rest = line
            .strip_prefix("### ")
            .or_else(|| line.strip_prefix("## "))?;
        let first = rest.split_whitespace().next()?;
        let number = first.trim_end_matches('.');
        number
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_digit())
            .then(|| number.to_owned())
    }

    /// The bold lead of a paragraph: the text between the first two `**` of a line that starts
    /// with `**`.
    fn bold_lead(line: &str) -> Option<Result<&str, String>> {
        let rest = line.strip_prefix("**")?;
        Some(
            rest.split_once("**")
                .map(|(lead, _)| lead)
                .ok_or_else(|| format!("a bold lead that does not close on its line: {line}")),
        )
    }

    /// The number of a numbered item, `7` for `7. **Permission floor:** ...`.
    fn item_number(line: &str) -> Option<u32> {
        let (digits, rest) = line.split_once(". ")?;
        (!digits.is_empty() && digits.chars().all(|c| c.is_ascii_digit()) && !rest.is_empty())
            .then(|| digits.parse().ok())
            .flatten()
    }

    /// The first component of a section number: `4` for `4.3`.
    fn top_level(section: &str) -> &str {
        section.split('.').next().unwrap_or(section)
    }

    /// The section an anchor id stands under: `4.3` for `§4.3 Writing.`, `13` for `§13.7`.
    fn section_of(anchor: &str) -> String {
        let body = anchor.strip_prefix('§').unwrap_or(anchor);
        let number = body.split(' ').next().unwrap_or(body);
        if top_level(number) == "13" {
            "13".to_owned()
        } else {
            number.to_owned()
        }
    }

    /// Every anchor of the spec in document order, or the problems of reading it.
    fn anchors(spec: &str) -> Result<Vec<String>, Vec<String>> {
        struct State {
            in_fence: bool,
            section: String,
            found: Vec<String>,
            problems: Vec<String>,
        }
        let end = spec.lines().fold(
            State {
                in_fence: false,
                section: "0".to_owned(),
                found: Vec::new(),
                problems: Vec::new(),
            },
            |mut state, line| {
                if line.starts_with("```") {
                    state.in_fence = !state.in_fence;
                } else if !state.in_fence {
                    if let Some(number) = heading_number(line) {
                        state.found.push(format!("§{number}"));
                        state.section = number;
                    } else if let Some(lead) = bold_lead(line) {
                        match lead {
                            Ok(text) => state.found.push(format!("§{} {text}", state.section)),
                            Err(problem) => state.problems.push(problem),
                        }
                    } else if top_level(&state.section) == "13"
                        && let Some(n) = item_number(line)
                    {
                        state.found.push(format!("§13.{n}"));
                    }
                }
                state
            },
        );
        if end.problems.is_empty() {
            Ok(end.found)
        } else {
            Err(end.problems)
        }
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum Status {
        Covered,
        NotApplicable,
        Planned,
        Gap,
    }

    impl Status {
        fn parse(text: &str) -> Option<Self> {
            match text {
                "covered" => Some(Self::Covered),
                "n/a" => Some(Self::NotApplicable),
                "planned" => Some(Self::Planned),
                "gap" => Some(Self::Gap),
                _ => None,
            }
        }
    }

    #[derive(Debug)]
    struct Row {
        anchor: String,
        status: Status,
        evidence: String,
    }

    /// The text of the matrix between its `## <title>` heading and the next `##` heading.
    fn part<'a>(matrix: &'a str, title: &str) -> Option<&'a str> {
        let heading = format!("## {title}\n");
        let start = matrix.find(&heading)? + heading.len();
        let rest = matrix.get(start..)?;
        Some(rest.split_once("\n## ").map_or(rest, |(head, _)| head))
    }

    /// The rows of the matrix: the table lines under `## Rows` whose first cell starts with `§`.
    fn rows(matrix: &str) -> Result<Vec<Row>, Vec<String>> {
        let Some(table) = part(matrix, "Rows") else {
            return Err(vec!["the matrix has no `## Rows` section".to_owned()]);
        };
        let parsed = table
            .lines()
            .filter(|line| line.starts_with("| §"))
            .map(|line| {
                let cells = line
                    .trim_start_matches("| ")
                    .trim_end_matches(" |")
                    .splitn(3, " | ")
                    .collect::<Vec<_>>();
                match cells.as_slice() {
                    [anchor, status, evidence] => Status::parse(status)
                        .map(|status| Row {
                            anchor: (*anchor).to_owned(),
                            status,
                            evidence: (*evidence).trim().to_owned(),
                        })
                        .ok_or_else(|| {
                            format!("the row of {anchor} has the status `{status}`, which is not covered, n/a, planned or gap")
                        }),
                    _ => Err(format!("a row that does not have three cells: {line}")),
                }
            })
            .collect::<Vec<_>>();
        let problems = parsed
            .iter()
            .filter_map(|row| row.as_ref().err().cloned())
            .collect::<Vec<_>>();
        if problems.is_empty() {
            Ok(parsed.into_iter().filter_map(Result::ok).collect())
        } else {
            Err(problems)
        }
    }

    /// The Rust files below `crates/`, as paths relative to the workspace root with `/`.
    fn rust_files(root: &Path) -> Vec<String> {
        fn walk(dir: &Path, root: &Path, out: &mut Vec<String>) {
            let mut entries = fs::read_dir(dir).map_or_else(
                |_| Vec::new(),
                |entries| entries.filter_map(Result::ok).collect::<Vec<_>>(),
            );
            entries.sort_by_key(fs::DirEntry::path);
            for entry in entries {
                let path = entry.path();
                if path.is_dir() {
                    walk(&path, root, out);
                } else if path.extension().is_some_and(|e| e == "rs")
                    && let Ok(relative) = path.strip_prefix(root)
                {
                    out.push(relative.to_string_lossy().replace('\\', "/"));
                }
            }
        }
        let mut out = Vec::new();
        walk(&root.join("crates"), root, &mut out);
        out
    }

    /// The name of the function a line declares, if any: `fn name(` or `fn name<`.
    fn declared_fn(line: &str) -> Option<&str> {
        let trimmed = line.trim_start();
        let after = trimmed
            .strip_prefix("pub(crate) fn ")
            .or_else(|| trimmed.strip_prefix("pub fn "))
            .or_else(|| trimmed.strip_prefix("async fn "))
            .or_else(|| trimmed.strip_prefix("fn "))?;
        let end = after.find(|c: char| !(c.is_alphanumeric() || c == '_'))?;
        after.get(..end).filter(|name| !name.is_empty())
    }

    /// The test functions of one source text: a function with `#[test]` among the attributes and
    /// comments above it, or any function that follows a `proptest!` line (the macro's functions
    /// carry no attribute). The source is read as text; no test is run.
    fn test_functions(source: &str) -> BTreeSet<String> {
        let lines = source.lines().collect::<Vec<_>>();
        let first_proptest = lines.iter().position(|line| line.contains("proptest!"));
        lines
            .iter()
            .enumerate()
            .filter_map(|(index, line)| {
                let name = declared_fn(line)?;
                let marked = lines[..index]
                    .iter()
                    .rev()
                    .map(|above| above.trim())
                    .take_while(|above| {
                        above.starts_with('#') || above.starts_with("//") || above.is_empty()
                    })
                    .any(|above| above.starts_with("#[test]"));
                let in_proptest = first_proptest.is_some_and(|start| index > start);
                (marked || in_proptest).then(|| name.to_owned())
            })
            .collect()
    }

    /// Every `path::test_name` of the workspace.
    fn test_index(root: &Path) -> BTreeSet<String> {
        rust_files(root)
            .into_iter()
            .flat_map(|path| {
                let source = fs::read_to_string(root.join(&path)).unwrap_or_default();
                test_functions(&source)
                    .into_iter()
                    .map(|name| format!("{path}::{name}"))
                    .collect::<Vec<_>>()
            })
            .collect()
    }

    /// The task that fills the gaps of a section.
    fn gap_task(section: &str) -> &'static str {
        match (top_level(section), section) {
            ("4" | "5" | "6", _) => "TASK-55.9.6",
            (_, "7.12" | "7.13") | ("9", _) => "TASK-55.9.8",
            ("7" | "8" | "11", _) => "TASK-55.9.7",
            _ => "TASK-55.9.9",
        }
    }

    const GAP_TASKS: [(&str, &str); 4] = [
        ("TASK-55.9.6", "§4, §5, §6"),
        ("TASK-55.9.7", "§7 (7.1 to 7.11), §8, §11"),
        ("TASK-55.9.8", "§7.12, §7.13, §9"),
        ("TASK-55.9.9", "§0 to §3, §10, §12 to §15"),
    ];

    /// The two summary tables the matrix must hold, computed from its rows.
    fn summary(rows: &[Row]) -> String {
        let count = |status: Status, wanted: &dyn Fn(&Row) -> bool| {
            rows.iter()
                .filter(|row| row.status == status && wanted(row))
                .count()
        };
        let by_section = rows
            .iter()
            .map(|row| top_level(&section_of(&row.anchor)).to_owned())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .map(|number| (number.parse::<u32>().unwrap_or(u32::MAX), number))
            .collect::<BTreeMap<_, _>>();
        let mut text = String::from(
            "| Section | covered | n/a | planned | gap | total |\n| --- | ---: | ---: | ---: | ---: | ---: |\n",
        );
        let line = |label: &str, wanted: &dyn Fn(&Row) -> bool| {
            let numbers = [
                count(Status::Covered, wanted),
                count(Status::NotApplicable, wanted),
                count(Status::Planned, wanted),
                count(Status::Gap, wanted),
            ];
            format!(
                "| {label} | {} | {} | {} | {} | {} |\n",
                numbers[0],
                numbers[1],
                numbers[2],
                numbers[3],
                numbers.iter().sum::<usize>()
            )
        };
        for number in by_section.values() {
            text.push_str(&line(&format!("§{number}"), &|row: &Row| {
                top_level(&section_of(&row.anchor)) == number
            }));
        }
        text.push_str(&line("all", &|_: &Row| true));
        text.push_str("\n| Gap task | Sections | gap rows |\n| --- | --- | ---: |\n");
        text.push_str(
            &GAP_TASKS
                .iter()
                .map(|(task, sections)| {
                    let gaps = count(Status::Gap, &|row: &Row| {
                        gap_task(&section_of(&row.anchor)) == *task
                    });
                    format!("| {task} | {sections} | {gaps} |")
                })
                .collect::<Vec<_>>()
                .join("\n"),
        );
        text.push('\n');
        text
    }

    /// Every problem of the matrix against the spec's anchors and the tests that exist.
    fn problems(
        anchors: &[String],
        rows: &[Row],
        tests: &BTreeSet<String>,
        matrix: &str,
    ) -> Vec<String> {
        let wanted = anchors.iter().collect::<BTreeSet<_>>();
        let mut seen = BTreeSet::new();
        let mut found = Vec::new();
        for anchor in anchors {
            if anchors.iter().filter(|other| *other == anchor).count() > 1 {
                found.push(format!(
                    "the spec has the anchor {anchor} more than once; add words to tell them apart"
                ));
            }
        }
        for row in rows {
            if !seen.insert(row.anchor.clone()) {
                found.push(format!("{} has more than one row", row.anchor));
            }
            if !wanted.contains(&row.anchor) {
                found.push(format!(
                    "stale row: the spec has no anchor {}; remove it or rename it",
                    row.anchor
                ));
            }
            found.extend(row_problems(row, tests));
        }
        found.extend(
            anchors
                .iter()
                .filter(|anchor| !seen.contains(*anchor))
                .map(|anchor| format!("missing row: {anchor}")),
        );
        let expected = summary(rows);
        let current = part(matrix, "Summary").unwrap_or_default();
        if !current.contains(expected.trim_end()) {
            found.push(format!(
                "the summary is out of date; it must hold:\n{expected}"
            ));
        }
        found
    }

    /// The problems of one row's evidence.
    fn row_problems(row: &Row, tests: &BTreeSet<String>) -> Vec<String> {
        let anchor = &row.anchor;
        match row.status {
            Status::Covered => {
                let named = row
                    .evidence
                    .split(',')
                    .map(str::trim)
                    .filter(|token| !token.is_empty())
                    .collect::<Vec<_>>();
                if named.is_empty() {
                    return vec![format!("{anchor} is covered and names no test")];
                }
                named
                    .into_iter()
                    .filter(|token| !tests.contains(*token))
                    .map(|token| {
                        format!("{anchor} names `{token}`, which is not a test function (path::test_name)")
                    })
                    .collect()
            }
            Status::NotApplicable | Status::Gap if row.evidence.chars().count() < 12 => {
                vec![format!(
                    "{anchor} needs its reason or the behaviour that lacks a test"
                )]
            }
            Status::Planned if !row.evidence.contains("TASK-") => {
                vec![format!("{anchor} is planned and names no task")]
            }
            Status::NotApplicable | Status::Planned | Status::Gap => Vec::new(),
        }
    }

    fn read(path: &str) -> String {
        fs::read_to_string(root().join(path))
            .unwrap_or_else(|error| panic!("{path} must be readable: {error}"))
    }

    /// The matrix is true: every anchor of the spec has exactly one row, every row names an
    /// anchor the spec has, every test it names exists, and its summary is current.
    #[test]
    fn the_matrix_has_a_true_row_for_every_anchor_of_the_spec() {
        let spec = read("docs/LAYERING-SPEC.md");
        let matrix = read("docs/spec-coverage.md");
        let anchors = anchors(&spec).unwrap_or_else(|problems| panic!("{}", problems.join("\n")));
        let rows = rows(&matrix).unwrap_or_else(|problems| panic!("{}", problems.join("\n")));
        let found = problems(&anchors, &rows, &test_index(&root()), &matrix);
        assert!(found.is_empty(), "{}", found.join("\n"));
    }

    // The guard shown failing, on small inputs.

    fn one_test() -> BTreeSet<String> {
        BTreeSet::from(["crates/x/tests/t.rs::a_test".to_owned()])
    }

    fn matrix_of(rows: &str, summary_rows: &[Row]) -> String {
        format!("## Rows\n\n{rows}\n## Summary\n\n{}", summary(summary_rows))
    }

    fn check(spec: &str, rows_text: &str, tests: &BTreeSet<String>) -> Vec<String> {
        let found = anchors(spec).expect("the spec reads");
        let provisional = matrix_of(rows_text, &[]);
        let parsed = rows(&provisional).expect("the rows read");
        let matrix = matrix_of(rows_text, &parsed);
        problems(&found, &parsed, tests, &matrix)
    }

    const SPEC: &str = "# Title\n\n**Status:** x\n\n## 1. One\n\n**Lead.** text\n\n### 1.1 Sub\n\n```\n**not a lead**\n## not a heading\n```\n";

    const ROWS: &str = "| Anchor | Status | Evidence |\n| --- | --- | --- |\n| §0 Status: | n/a | a note about the draft |\n| §1 | covered | crates/x/tests/t.rs::a_test |\n| §1 Lead. | gap | the lead behaviour has no test |\n| §1.1 | planned | TASK-1 builds it |\n";

    #[test]
    fn anchors_are_headings_bold_leads_and_numbered_items_outside_code_fences() {
        let spec = "## 12. Facts\n\n1. not an anchor\n\n## 13. Acceptance\n\n1. **One:** a\n   continued\n2. **Two:** b\n";
        assert_eq!(
            anchors(SPEC).expect("reads"),
            ["§0 Status:", "§1", "§1 Lead.", "§1.1"]
        );
        assert_eq!(
            anchors(spec).expect("reads"),
            ["§12", "§13", "§13.1", "§13.2"]
        );
    }

    #[test]
    fn a_matrix_with_a_true_row_for_each_anchor_has_no_problem() {
        assert_eq!(check(SPEC, ROWS, &one_test()), Vec::<String>::new());
    }

    #[test]
    fn the_guard_fails_on_an_anchor_without_a_row() {
        let rows = ROWS.replace("| §1 Lead. | gap | the lead behaviour has no test |\n", "");
        assert_eq!(check(SPEC, &rows, &one_test()), ["missing row: §1 Lead."]);
    }

    #[test]
    fn the_guard_fails_on_a_row_for_an_anchor_the_spec_no_longer_has() {
        let rows = format!("{ROWS}| §9 Gone. | n/a | the paragraph was removed |\n");
        let found = check(SPEC, &rows, &one_test());
        assert_eq!(found.len(), 1, "{found:?}");
        assert!(found[0].starts_with("stale row: the spec has no anchor §9 Gone."));
    }

    #[test]
    fn the_guard_fails_on_a_test_function_that_does_not_exist() {
        let rows = ROWS.replace("t.rs::a_test", "t.rs::no_such_test");
        let found = check(SPEC, &rows, &one_test());
        assert_eq!(found.len(), 1, "{found:?}");
        assert!(
            found[0].contains("`crates/x/tests/t.rs::no_such_test`, which is not a test function")
        );
    }

    #[test]
    fn the_guard_fails_on_a_second_row_a_bad_status_and_an_empty_reason() {
        let twice = format!("{ROWS}| §1.1 | n/a | a second row for one anchor |\n");
        assert!(
            check(SPEC, &twice, &one_test()).contains(&"§1.1 has more than one row".to_owned())
        );
        let bad = ROWS.replace("| gap |", "| done |");
        assert!(rows(&matrix_of(&bad, &[])).is_err());
        let empty = ROWS.replace("a note about the draft", "none");
        assert_eq!(
            check(SPEC, &empty, &one_test()),
            ["§0 Status: needs its reason or the behaviour that lacks a test"]
        );
    }

    #[test]
    fn the_guard_fails_when_the_summary_is_out_of_date() {
        let found = problems(
            &anchors(SPEC).expect("reads"),
            &rows(&matrix_of(ROWS, &[])).expect("reads"),
            &one_test(),
            &matrix_of(ROWS, &[]),
        );
        assert_eq!(found.len(), 1, "{found:?}");
        assert!(found[0].starts_with("the summary is out of date"));
    }

    #[test]
    fn a_test_function_is_one_with_the_test_attribute_or_inside_a_proptest_block() {
        let source = "#[test]\nfn plain() {}\n\n/// doc\n#[test]\n#[expect(clippy::x, reason = \"y\")]\nfn with_doc() {}\n\nfn helper() {}\n\nproptest! {\n    fn law(x in 0..1) {}\n}\n";
        assert_eq!(
            test_functions(source),
            BTreeSet::from(["plain".to_owned(), "with_doc".to_owned(), "law".to_owned()])
        );
    }
}
