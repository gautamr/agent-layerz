use proptest::prelude::{Strategy, prop_assert, prop_assert_eq, proptest};

use super::myers::{Edit, script};
use super::{Comparison, Hunk, Row, compare, compare_within, lines};
use crate::untrusted_text::Stripped;

fn rendered(old: &str, new: &str) -> String {
    compare(old.as_bytes(), new.as_bytes()).render(&Stripped::from_untrusted("dir/file.md"))
}

/// The new text, made from the old text and the hunks alone: the old lines between hunks are
/// copied, the hunks give the rest.
fn applied(old: &str, hunks: &[Hunk]) -> String {
    let old_lines = lines(old);
    let (done, taken) = hunks
        .iter()
        .fold((String::new(), 0_usize), |(mut done, taken), hunk| {
            let first = hunk.old_start - usize::from(hunk.old_count > 0);
            done.extend(
                old_lines
                    .iter()
                    .skip(taken)
                    .take(first - taken)
                    .map(|line| line_text(&line.text, line.terminated)),
            );
            done.extend(hunk.rows.iter().filter(|row| row.in_new()).map(|row| {
                let (Row::Context(line) | Row::Added(line) | Row::Removed(line)) = row;
                line_text(&line.text, line.terminated)
            }));
            (done, first + hunk.old_count)
        });
    done + &old_lines
        .iter()
        .skip(taken)
        .map(|line| line_text(&line.text, line.terminated))
        .collect::<String>()
}

fn line_text(text: &str, terminated: bool) -> String {
    format!("{text}{}", if terminated { "\n" } else { "" })
}

#[test]
fn a_change_in_the_middle_keeps_three_lines_of_context() {
    let old = "1\n2\n3\n4\n5\n6\n7\n8\n9\n10\n11\n12\n13\n";
    let new = "1\n2\n3\n4\n5\n6\nseven\n8\n9\n10\n11\n12\n13\n";
    assert_eq!(
        rendered(old, new),
        "--- a/dir/file.md\n+++ b/dir/file.md\n@@ -4,7 +4,7 @@\n 4\n 5\n 6\n-7\n+seven\n 8\n 9\n 10\n"
    );
}

#[test]
fn a_created_file_is_compared_with_empty_text() {
    assert_eq!(
        rendered("", "a\nb\n"),
        "--- a/dir/file.md\n+++ b/dir/file.md\n@@ -0,0 +1,2 @@\n+a\n+b\n"
    );
}

#[test]
fn a_deleted_file_is_compared_with_empty_text_after_it() {
    assert_eq!(
        rendered("a\nb\n", ""),
        "--- a/dir/file.md\n+++ b/dir/file.md\n@@ -1,2 +0,0 @@\n-a\n-b\n"
    );
}

#[test]
fn a_last_line_without_a_line_feed_is_marked() {
    assert_eq!(
        rendered("a\nb", "a\nb\n"),
        "--- a/dir/file.md\n+++ b/dir/file.md\n@@ -1,2 +1,2 @@\n a\n-b\n\\ No newline at end of file\n+b\n"
    );
}

#[test]
fn removed_lines_come_before_added_lines_in_a_run() {
    assert_eq!(
        rendered("a\nb\nc\n", "a\nx\ny\nc\n"),
        "--- a/dir/file.md\n+++ b/dir/file.md\n@@ -1,3 +1,4 @@\n a\n-b\n+x\n+y\n c\n"
    );
}

#[test]
fn hunks_whose_context_meets_are_joined_and_others_are_not() {
    let numbers = |changed: &[usize]| -> String {
        (1..=20)
            .map(|n| {
                if changed.contains(&n) {
                    format!("changed {n}\n")
                } else {
                    format!("{n}\n")
                }
            })
            .collect()
    };
    let at = |a: usize, b: usize| {
        let Comparison::Text(hunks) = compare(numbers(&[]).as_bytes(), numbers(&[a, b]).as_bytes())
        else {
            unreachable!("the texts differ");
        };
        hunks.len()
    };
    // Six unchanged lines between two changes: the contexts meet, one hunk.
    assert_eq!(at(5, 12), 1);
    // Seven: the contexts leave a gap, two hunks.
    assert_eq!(at(5, 13), 2);
}

#[test]
fn an_insertion_between_lines_counts_its_context_on_both_sides() {
    assert_eq!(
        rendered("a\nb\n", "a\nx\nb\n"),
        "--- a/dir/file.md\n+++ b/dir/file.md\n@@ -1,2 +1,3 @@\n a\n+x\n b\n"
    );
    let Comparison::Text(hunks) = compare(b"a\n", b"a\nb\n") else {
        unreachable!("the texts differ");
    };
    assert_eq!(
        hunks
            .iter()
            .map(|h| (h.old_start, h.old_count))
            .collect::<Vec<_>>(),
        [(1, 1)]
    );
}

#[test]
fn equal_texts_have_no_diff() {
    assert_eq!(compare(b"a\n", b"a\n"), Comparison::Unchanged);
    // A mode change alone and an empty file have the headers and no hunk, as in the reference.
    assert_eq!(
        rendered("a\n", "a\n"),
        "--- a/dir/file.md\n+++ b/dir/file.md\n"
    );
    assert_eq!(rendered("", ""), "--- a/dir/file.md\n+++ b/dir/file.md\n");
}

#[test]
fn a_nul_or_invalid_utf8_makes_the_file_binary() {
    let path = Stripped::from_untrusted("x.bin");
    for (old, new) in [
        (&b"a\0b"[..], &b"a\n"[..]),
        (b"a\n", b"\xff\xfe"),
        (b"", b"\0"),
    ] {
        assert_eq!(
            compare(old, new).render(&path),
            "Binary file x.bin changes\n"
        );
    }
}

#[test]
fn a_nul_after_the_first_8192_bytes_is_still_text() {
    let mut new = "a\n".repeat(5000).into_bytes();
    new.push(0);
    assert!(matches!(compare(b"", &new), Comparison::Text(_)));
}

#[test]
fn control_and_format_characters_never_reach_the_patch() {
    let printed = rendered("a\n", "a\u{1b}[2J\u{202e}b\r\n");
    assert!(printed.contains("+a[2Jb\n"), "{printed:?}");
    assert!(
        !printed.contains('\u{1b}') && !printed.contains('\r') && !printed.contains('\u{202e}')
    );
    let path = compare(b"a\n", b"b\n").render(&Stripped::from_untrusted("x\u{1b}y\n"));
    assert!(path.starts_with("--- a/xy\n+++ b/xy\n"), "{path:?}");
}

/// The length of a longest common subsequence, by the textbook table.
fn lcs(left: &[String], right: &[String]) -> usize {
    let row = vec![0_usize; right.len() + 1];
    left.iter()
        .fold(row, |previous, l| {
            right
                .iter()
                .enumerate()
                .fold(
                    (vec![0_usize], 0_usize),
                    |(mut current, diagonal), (index, r)| {
                        let up = previous.get(index + 1).copied().unwrap_or_default();
                        let left_value = current.last().copied().unwrap_or_default();
                        let value = if l == r {
                            diagonal + 1
                        } else {
                            up.max(left_value)
                        };
                        current.push(value);
                        (current, up)
                    },
                )
                .0
        })
        .last()
        .copied()
        .unwrap_or_default()
}

fn text() -> impl Strategy<Value = String> {
    proptest::collection::vec("[abc]{0,2}", 0..14).prop_flat_map(|parts| {
        proptest::bool::ANY.prop_map(move |newline| {
            let joined = parts.join("\n");
            if newline && !joined.is_empty() {
                joined + "\n"
            } else {
                joined
            }
        })
    })
}

proptest! {
    #[test]
    fn applying_the_hunks_to_the_old_text_gives_the_new_text(old in text(), new in text()) {
        match compare(old.as_bytes(), new.as_bytes()) {
            Comparison::Unchanged => prop_assert_eq!(&old, &new),
            Comparison::Binary => prop_assert!(false, "generated texts are not binary"),
            Comparison::Text(hunks) => {
                prop_assert!(!hunks.is_empty());
                prop_assert_eq!(applied(&old, &hunks), new);
            }
        }
    }

    #[test]
    fn equal_texts_give_no_diff_and_the_same_texts_the_same_diff(old in text(), new in text()) {
        prop_assert_eq!(compare(old.as_bytes(), old.as_bytes()), Comparison::Unchanged);
        prop_assert_eq!(compare(old.as_bytes(), new.as_bytes()), compare(old.as_bytes(), new.as_bytes()));
    }

    #[test]
    fn the_edit_script_is_a_shortest_one_with_removals_first(
        old in proptest::collection::vec("[abc]", 0..16),
        new in proptest::collection::vec("[abc]", 0..16),
    ) {
        let edits = script(&old, &new, usize::MAX);
        let kept = edits.iter().filter(|e| **e == Edit::Keep).count();
        prop_assert_eq!(kept, lcs(&old, &new));
        prop_assert_eq!(edits.iter().filter(|e| **e != Edit::Add).count(), old.len());
        prop_assert_eq!(edits.iter().filter(|e| **e != Edit::Remove).count(), new.len());
        prop_assert!(!edits.windows(2).any(|pair| pair == [Edit::Add, Edit::Remove]));
    }

    #[test]
    fn the_hunk_counts_match_their_rows_and_context_is_at_most_three_lines(old in text(), new in text()) {
        if let Comparison::Text(hunks) = compare(old.as_bytes(), new.as_bytes()) {
            for hunk in &hunks {
                prop_assert_eq!(hunk.old_count, hunk.rows.iter().filter(|r| r.in_old()).count());
                prop_assert_eq!(hunk.new_count, hunk.rows.iter().filter(|r| r.in_new()).count());
                let leading = hunk.rows.iter().take_while(|r| matches!(r, Row::Context(_))).count();
                let trailing = hunk.rows.iter().rev().take_while(|r| matches!(r, Row::Context(_))).count();
                prop_assert!(leading <= 3 && trailing <= 3);
            }
        }
    }
}

/// A budget so small that the search gives up on most pairs of texts.
const TINY_BUDGET: usize = 24;

#[test]
fn past_the_budget_the_middle_is_replaced_whole_between_the_shared_ends() {
    let old = "head\n1\n2\n3\n4\n5\n6\n7\n8\ntail\n";
    let new = "head\n8\n1\n7\n2\n6\n3\n5\n4\ntail\n";
    let Comparison::Text(hunks) = compare_within(old.as_bytes(), new.as_bytes(), 1) else {
        unreachable!("the texts differ");
    };
    assert_eq!(hunks.len(), 1);
    let kinds: String = hunks
        .iter()
        .flat_map(|hunk| hunk.rows.iter())
        .map(|row| match row {
            Row::Context(_) => 'c',
            Row::Removed(_) => '-',
            Row::Added(_) => '+',
        })
        .collect();
    assert_eq!(kinds, "c--------++++++++c");
}

/// The longest run of lines two texts cannot share, 1 MiB each: every line differs.
fn crafted(side: char) -> String {
    (0..150_000)
        .map(|n| format!("{side}{n}\n"))
        .collect::<Vec<_>>()
        .concat()
}

#[test]
fn crafted_texts_of_about_one_mebibyte_are_compared_in_bounded_time() {
    // Every line differs; and a few dozen edits scattered over a long common text, which is as
    // near the cap as a pair can be without giving up.
    let scattered = |changed: bool| -> String {
        (0..150_000)
            .map(|n| {
                if changed && n % 3_750 == 1 {
                    format!("changed {n}\n")
                } else {
                    format!("{n}\n")
                }
            })
            .collect::<Vec<_>>()
            .concat()
    };
    let pairs = [
        (crafted('a'), crafted('b')),
        (scattered(false), scattered(true)),
    ];
    assert!(
        pairs
            .iter()
            .all(|(old, new)| old.len() > 800_000 && new.len() > 800_000)
    );
    let (sender, receiver) = std::sync::mpsc::channel();
    // The worker is left behind when it is too slow: the test fails at the timeout.
    std::thread::spawn(move || {
        let sizes: Vec<usize> = pairs
            .iter()
            .map(|(old, new)| {
                compare(old.as_bytes(), new.as_bytes())
                    .render(&Stripped::from_untrusted("big"))
                    .len()
            })
            .collect();
        sender.send(sizes).expect("the test is waiting");
    });
    let sizes = receiver
        .recv_timeout(std::time::Duration::from_secs(30))
        .expect("the comparison ends in time");
    assert!(sizes.iter().all(|size| *size > 0));
}

proptest! {
    #[test]
    fn a_diff_that_gave_up_the_search_still_applies_cleanly(old in text(), new in text()) {
        match compare_within(old.as_bytes(), new.as_bytes(), TINY_BUDGET) {
            Comparison::Unchanged => prop_assert_eq!(&old, &new),
            Comparison::Binary => prop_assert!(false, "generated texts are not binary"),
            Comparison::Text(hunks) => prop_assert_eq!(applied(&old, &hunks), new),
        }
    }

    #[test]
    fn a_budget_script_has_every_element_once_and_no_addition_before_a_removal(
        old in proptest::collection::vec("[abc]", 0..16),
        new in proptest::collection::vec("[abc]", 0..16),
        cap in 0_usize..40,
    ) {
        let edits = script(&old, &new, cap);
        prop_assert_eq!(edits.iter().filter(|e| **e != Edit::Add).count(), old.len());
        prop_assert_eq!(edits.iter().filter(|e| **e != Edit::Remove).count(), new.len());
        prop_assert!(!edits.windows(2).any(|pair| pair == [Edit::Add, Edit::Remove]));
    }
}
