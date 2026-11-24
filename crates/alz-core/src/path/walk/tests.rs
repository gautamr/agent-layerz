use std::collections::BTreeMap;

use proptest::prelude::{Strategy, prop_assert, proptest};
use unicode_normalization::UnicodeNormalization;

use super::{Entry, LinkTarget, Name, RealPathWalk, Resolved, Step, Stop};
use crate::account::{AccountFolders, Relation};
use crate::path::AbsolutePath;

fn path(text: &str) -> AbsolutePath {
    AbsolutePath::try_from(text).expect("absolute")
}

fn target(text: &str) -> LinkTarget {
    LinkTarget::try_from(text).expect("a link target")
}

#[derive(Clone, Copy)]
enum Node {
    Dir,
    File,
    Link(&'static str),
}

/// A file system as a table of absolute path text to what is there. Every request that the walk
/// makes is recorded, so a test can say what was never asked.
struct Table(BTreeMap<&'static str, Node>);

impl Table {
    fn entry(&self, at: &AbsolutePath) -> Entry {
        match self.0.get(at.as_str()) {
            None => Entry::Missing,
            Some(Node::Dir) => Entry::Directory,
            Some(Node::File) => Entry::Other,
            Some(Node::Link(_)) => Entry::Link,
        }
    }

    fn link(&self, at: &AbsolutePath) -> LinkTarget {
        match self.0.get(at.as_str()) {
            Some(Node::Link(to)) => target(to),
            _ => target("missing-link"),
        }
    }
}

type Outcome = Result<Resolved, Stop>;

/// Answers every request from the table; returns the requests, as text, and the outcome.
fn run(folders: &AccountFolders, table: &Table, start: &str) -> (Vec<String>, Outcome) {
    drive(table, RealPathWalk::start(&path(start), folders))
}

/// [`run`] for a walk that locates a directory.
fn locate(folders: &AccountFolders, table: &Table, start: &str) -> (Vec<String>, Outcome) {
    drive(table, RealPathWalk::locate(&path(start), folders))
}

fn drive(table: &Table, first: Step<'_>) -> (Vec<String>, Outcome) {
    let mut asked = Vec::new();
    let mut step = first;
    loop {
        step = match step {
            Step::Lstat(request) => {
                asked.push(format!("lstat {}", request.path()));
                let entry = table.entry(request.path());
                request.answer(entry)
            }
            Step::Readlink(request) => {
                asked.push(format!("readlink {}", request.path()));
                let to = table.link(request.path());
                request.answer(&to)
            }
            Step::Done(resolved) => return (asked, Ok(resolved)),
            Step::Stopped(ended) => return (asked, Err(ended)),
        };
    }
}

fn folders() -> AccountFolders {
    AccountFolders::from_env(&path("/Users/u"), Some(&path("/work/cfg")), None)
}

fn table(nodes: &[(&'static str, Node)]) -> Table {
    Table(nodes.iter().copied().collect())
}

fn existing(text: &str) -> Resolved {
    Resolved::Existing(path(text))
}

fn never_asked(asked: &[String], below: &str) {
    assert!(
        !asked.iter().any(|request| request.contains(below)),
        "{below} was asked about: {asked:?}"
    );
}

#[test]
fn a_path_without_links_is_its_own_real_path() {
    let fs = table(&[
        ("/srv", Node::Dir),
        ("/srv/repo", Node::Dir),
        ("/srv/repo/layer", Node::Dir),
    ]);
    let (asked, outcome) = run(&folders(), &fs, "/srv//repo/./layer/");
    assert_eq!(outcome, Ok(existing("/srv/repo/layer")));
    assert_eq!(
        asked,
        ["lstat /srv", "lstat /srv/repo", "lstat /srv/repo/layer"]
    );
}

#[test]
fn the_root_is_never_asked_about() {
    let (asked, outcome) = run(&folders(), &table(&[]), "/");
    // `/` is above `$HOME`, so it is no result; but nothing is asked about it.
    assert_eq!(outcome, Err(Stop::Refused(Relation::Home)));
    let (asked_elsewhere, outcome) = run(&folders(), &table(&[("/srv", Node::Dir)]), "/srv");
    assert_eq!(outcome, Ok(existing("/srv")));
    assert_eq!(asked_elsewhere, ["lstat /srv"]);
    assert_eq!(asked, Vec::<String>::new());
}

#[test]
fn a_relative_link_continues_from_its_folder_and_an_absolute_one_from_the_root() {
    let fs = table(&[
        ("/srv", Node::Dir),
        ("/srv/repo", Node::Dir),
        ("/srv/repo/relative", Node::Link("../hub/layer")),
        ("/srv/repo/absolute", Node::Link("/srv/hub")),
        ("/srv/hub", Node::Dir),
        ("/srv/hub/layer", Node::Dir),
    ]);
    assert_eq!(
        run(&folders(), &fs, "/srv/repo/relative").1,
        Ok(existing("/srv/hub/layer"))
    );
    let (asked, outcome) = run(&folders(), &fs, "/srv/repo/absolute/layer");
    assert_eq!(outcome, Ok(existing("/srv/hub/layer")));
    assert!(asked.contains(&"readlink /srv/repo/absolute".to_owned()));
}

#[test]
fn dot_dot_after_a_link_goes_to_the_parent_of_what_the_link_points_at() {
    let fs = table(&[
        ("/srv", Node::Dir),
        ("/srv/repo", Node::Dir),
        ("/srv/repo/alias", Node::Link("/srv/hub/layer")),
        ("/srv/hub", Node::Dir),
        ("/srv/hub/layer", Node::Dir),
        ("/srv/hub/other", Node::Dir),
    ]);
    assert_eq!(
        run(&folders(), &fs, "/srv/repo/alias/../other").1,
        Ok(existing("/srv/hub/other"))
    );
}

#[test]
fn a_link_planted_into_an_account_folder_ends_the_walk_before_any_lookup_there() {
    for (link, expected) in [
        ("/Users/u/.claude-work", Relation::AccountFolder),
        ("/Users/u/.claude-work/skills", Relation::AccountFolder),
        ("/users/U/.CODEX-Client/x", Relation::AccountFolder),
        ("../../Users/u/.claude-work", Relation::AccountFolder),
        ("/work/cfg", Relation::Configured),
        ("/work/cfg/projects", Relation::Configured),
    ] {
        let fs = table(&[
            ("/srv", Node::Dir),
            ("/srv/repo", Node::Dir),
            ("/srv/repo/alias", Node::Link(link)),
            ("/Users", Node::Dir),
            ("/Users/u", Node::Dir),
            ("/Users/u/.claude-work", Node::Dir),
            ("/work", Node::Dir),
            ("/work/cfg", Node::Dir),
        ]);
        let (asked, outcome) = run(&folders(), &fs, "/srv/repo/alias/anything");
        assert_eq!(outcome, Err(Stop::Refused(expected)), "{link}");
        for inside in [".claude-work", ".CODEX-Client", "/work/cfg"] {
            never_asked(&asked, inside);
        }
        assert!(
            asked
                .iter()
                .all(|request| !request.to_lowercase().contains(".claude-")),
            "{link}: {asked:?}"
        );
    }
}

#[test]
fn a_path_written_with_another_case_or_form_of_an_account_folder_is_not_asked_about() {
    let nfd = AccountFolders::from_env(&path("/Users/\u{e9}"), None, None);
    for (folders, written) in [
        (folders(), "/Users/u/.CLAUDE-work/projects"),
        (folders(), "/USERS/U/.claude"),
        (nfd, "/Users/e\u{301}/.Codex-x"),
    ] {
        let (asked, outcome) = run(&folders, &table(&[]), written);
        assert_eq!(
            outcome,
            Err(Stop::Refused(Relation::AccountFolder)),
            "{written}"
        );
        never_asked(&asked, ".c");
        never_asked(&asked, ".C");
    }
}

#[test]
fn home_and_the_folders_above_it_are_looked_at_but_never_a_result() {
    let fs = table(&[
        ("/Users", Node::Dir),
        ("/Users/u", Node::Dir),
        ("/Users/u/dev", Node::Dir),
    ]);
    let (asked, outcome) = run(&folders(), &fs, "/Users/u");
    assert_eq!(outcome, Err(Stop::Refused(Relation::Home)));
    assert_eq!(asked, ["lstat /Users", "lstat /Users/u"]);
    assert_eq!(
        run(&folders(), &fs, "/Users").1,
        Err(Stop::Refused(Relation::Home))
    );
    assert_eq!(
        run(&folders(), &fs, "/Users/u/dev/..").1,
        Err(Stop::Refused(Relation::Home))
    );
}

#[test]
fn a_folder_that_holds_a_configured_folder_is_refused() {
    let fs = table(&[("/work", Node::Dir)]);
    assert_eq!(
        run(&folders(), &fs, "/work").1,
        Err(Stop::Refused(Relation::Holds))
    );
}

#[test]
fn a_missing_name_ends_the_walk_with_what_exists_and_what_does_not() {
    let fs = table(&[("/srv", Node::Dir), ("/srv/repo", Node::Dir)]);
    let (asked, outcome) = run(&folders(), &fs, "/srv/repo/a/./b/c");
    assert_eq!(
        asked,
        ["lstat /srv", "lstat /srv/repo", "lstat /srv/repo/a"]
    );
    match outcome {
        Ok(Resolved::Missing { existing, below }) => {
            assert_eq!(existing.as_str(), "/srv/repo");
            assert_eq!(
                below.iter().map(Name::as_str).collect::<Vec<_>>(),
                ["a", "b", "c"]
            );
        }
        other => panic!("not a missing path: {other:?}"),
    }
}

#[test]
fn a_missing_name_followed_by_dot_dot_cannot_be_resolved() {
    let fs = table(&[("/srv", Node::Dir)]);
    assert_eq!(run(&folders(), &fs, "/srv/a/../b").1, Err(Stop::NotFound));
}

#[test]
fn a_missing_path_that_would_hold_a_configured_folder_is_refused_without_a_lookup_there() {
    let fs = table(&[("/work", Node::Dir)]);
    let (asked, outcome) = run(&folders(), &fs, "/work/cfg-missing/../x");
    assert_eq!(outcome, Err(Stop::NotFound));
    assert_eq!(asked, ["lstat /work", "lstat /work/cfg-missing"]);
}

#[test]
fn a_file_with_more_path_below_it_is_not_a_directory() {
    let fs = table(&[("/srv", Node::Dir), ("/srv/file", Node::File)]);
    assert_eq!(
        run(&folders(), &fs, "/srv/file/x").1,
        Err(Stop::NotADirectory)
    );
    assert_eq!(
        run(&folders(), &fs, "/srv/file").1,
        Ok(existing("/srv/file"))
    );
}

#[test]
fn a_loop_of_links_stops_after_forty_of_them() {
    let fs = table(&[("/srv", Node::Dir), ("/srv/loop", Node::Link("loop"))]);
    let (asked, outcome) = run(&folders(), &fs, "/srv/loop");
    assert_eq!(outcome, Err(Stop::TooManyLinks));
    assert_eq!(
        asked
            .iter()
            .filter(|request| request.starts_with("readlink"))
            .count(),
        40
    );
}

#[test]
fn a_refusal_names_the_reason_and_never_the_path() {
    for stop in [
        Stop::Refused(Relation::AccountFolder),
        Stop::Refused(Relation::Configured),
        Stop::Refused(Relation::Home),
        Stop::Refused(Relation::Holds),
        Stop::TooManyLinks,
        Stop::NotFound,
        Stop::NotADirectory,
    ] {
        let text = stop.to_string();
        assert!(!text.contains('/'), "{text}");
    }
}

#[test]
fn a_link_target_is_not_empty_and_holds_no_nul() {
    assert!(LinkTarget::try_from("").is_err());
    assert!(LinkTarget::try_from("a\0b").is_err());
    assert!(LinkTarget::try_from("../a").is_ok());
}

// The law, TASK-52 AC #3: whatever the answers, nothing inside an account folder is asked about,
// and no result touches one.

/// Names that make a path a case, form or link attack on the folders below.
const NAMES: [&str; 17] = [
    "Users",
    "users",
    "\u{e9}",
    "e\u{301}",
    "\u{c9}",
    "x",
    "other",
    "cfg",
    "dir",
    "work",
    ".claude-w",
    ".CLAUDE-W",
    ".codex-c",
    ".claude",
    ".claudex",
    "..",
    ".",
];

fn name() -> impl Strategy<Value = String> {
    proptest::sample::select(NAMES.to_vec()).prop_map(str::to_owned)
}

/// Where a path starts: nowhere in particular, or in the home or the configured folder, spelled
/// in a form or case that the folders must still recognise.
fn written() -> impl Strategy<Value = String> {
    let start = proptest::sample::select(vec![
        "",
        "/",
        "/Users/\u{e9}/",
        "/users/e\u{301}/",
        "/USERS/\u{c9}/",
        "/cfg/dir/",
        "/CFG/Dir/",
        "../",
        "../../",
    ]);
    (start, proptest::collection::vec(name(), 0..5))
        .prop_map(|(start, names)| format!("{start}{}", names.join("/")))
}

/// What `lstat` finds, and where a link would point if it is one.
fn answer() -> impl Strategy<Value = (Entry, String)> {
    (
        proptest::sample::select(vec![
            Entry::Missing,
            Entry::Directory,
            Entry::Directory,
            Entry::Link,
            Entry::Link,
            Entry::Other,
        ]),
        written().prop_filter("not empty", |text| !text.is_empty()),
    )
}

/// An account folder by this test's own rule, written differently from the implementation: the
/// components folded with the same Unicode forms, joined, and compared as text.
fn oracle_inside(real: &AbsolutePath) -> bool {
    let folded = real
        .components()
        .fold(Vec::<String>::new(), |mut parts, component| {
            match component {
                "." => {}
                ".." => {
                    parts.pop();
                }
                name => parts.push(
                    name.nfc()
                        .collect::<String>()
                        .to_lowercase()
                        .nfc()
                        .collect(),
                ),
            }
            parts
        });
    let text = format!("/{}/", folded.join("/"));
    let account = [
        "/users/\u{e9}/.claude/",
        "/users/\u{e9}/.codex/",
        "/users/\u{e9}/.claude.json/",
    ]
    .iter()
    .any(|folder| text.starts_with(folder))
        || text.starts_with("/users/\u{e9}/.claude-")
        || text.starts_with("/users/\u{e9}/.codex-");
    account || text.starts_with("/cfg/dir/")
}

fn attack_folders() -> AccountFolders {
    AccountFolders::from_env(&path("/Users/\u{e9}"), Some(&path("/cfg/dir")), None)
}

proptest! {
    #[test]
    fn no_answers_make_the_walk_ask_about_a_name_inside_an_account_folder(
        start in written().prop_map(|text| format!("/{text}")),
        answers in proptest::collection::vec(answer(), 0..24),
    ) {
        let folders = attack_folders();
        let mut remaining = answers.into_iter();
        let mut link = target("x");
        let mut step = RealPathWalk::start(&path(&start), &folders);
        for _ in 0..2_000 {
            step = match step {
                Step::Lstat(request) => {
                    prop_assert!(!oracle_inside(request.path()), "lstat {}", request.path());
                    prop_assert!(
                        !folders.relation(request.path()).is_some_and(Relation::is_inside),
                        "lstat {}", request.path()
                    );
                    let (entry, to) = remaining.next().unwrap_or((Entry::Directory, String::new()));
                    link = LinkTarget::try_from(to.as_str()).unwrap_or_else(|_empty| target("x"));
                    request.answer(entry)
                }
                Step::Readlink(request) => {
                    prop_assert!(!oracle_inside(request.path()), "readlink {}", request.path());
                    request.answer(&link)
                }
                Step::Done(resolved) => {
                    let whole = match &resolved {
                        Resolved::Existing(path) => path.clone(),
                        Resolved::Missing { existing, below } => path(&format!(
                            "{}/{}",
                            existing.as_str().trim_end_matches('/'),
                            below.iter().map(Name::as_str).collect::<Vec<_>>().join("/")
                        )),
                    };
                    prop_assert!(!folders.touches(&whole), "result {whole}");
                    prop_assert!(!oracle_inside(&whole), "result {whole}");
                    return Ok(());
                }
                Step::Stopped(_) => return Ok(()),
            };
        }
        prop_assert!(false, "the walk did not end");
    }
}

#[test]
fn locating_a_folder_that_holds_a_configured_folder_finds_its_real_path() {
    let fs = table(&[("/work", Node::Dir), ("/work/cfg", Node::Dir)]);
    let (asked, outcome) = locate(&folders(), &fs, "/work");
    assert_eq!(outcome, Ok(existing("/work")));
    assert_eq!(asked, ["lstat /work"]);
    let (_, strict) = run(&folders(), &fs, "/work");
    assert_eq!(strict, Err(Stop::Refused(Relation::Holds)));
}

#[test]
fn locating_inside_a_configured_folder_names_the_folder_and_asks_nothing_inside() {
    let fs = table(&[
        ("/work", Node::Dir),
        ("/work/cfg", Node::Dir),
        ("/work/cfg/sub", Node::Dir),
    ]);
    let (asked, outcome) = locate(&folders(), &fs, "/work/cfg/sub");
    assert_eq!(
        outcome,
        Err(Stop::Inside {
            relation: Relation::Configured,
            at: path("/work/cfg")
        })
    );
    never_asked(&asked, "/work/cfg");
}

#[test]
fn locating_through_a_planted_link_stops_at_the_account_folder_it_leads_to() {
    let fs = table(&[
        ("/Users", Node::Dir),
        ("/Users/u", Node::Dir),
        ("/Users/u/.claude-work", Node::Dir),
        ("/Users/u/.claude-work/x", Node::Dir),
        ("/repo", Node::Dir),
        ("/repo/plant", Node::Link("/Users/u/.claude-work/x")),
    ]);
    let (asked, outcome) = locate(&folders(), &fs, "/repo/plant");
    assert_eq!(
        outcome,
        Err(Stop::Inside {
            relation: Relation::AccountFolder,
            at: path("/Users/u/.claude-work")
        })
    );
    never_asked(&asked, ".claude-work");
}

#[test]
fn locating_the_home_folder_or_above_it_is_refused_as_home() {
    let fs = table(&[("/Users", Node::Dir), ("/Users/u", Node::Dir)]);
    let (_, outcome) = locate(&folders(), &fs, "/Users/u");
    assert_eq!(outcome, Err(Stop::Refused(Relation::Home)));
}
