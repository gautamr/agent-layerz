use proptest::prelude::{prop_assert, prop_assert_eq, proptest};

use super::{Glob, RuleMode, RuleName};
use crate::diagnostic::{Message, MessagePart};

fn accepts_name(text: &str) -> bool {
    RuleName::try_from(text).is_ok()
}

#[test]
fn a_rule_name_is_kebab_case() {
    for text in ["backend", "a", "a-b", "a1-b2-3", "0"] {
        assert!(accepts_name(text), "{text}");
    }
    for text in [
        "", "-a", "a-", "a--b", "A", "a_b", "a.b", "a b", "é", "a/b", "a\u{0}",
    ] {
        assert!(!accepts_name(text), "{text:?}");
    }
}

#[test]
fn a_rule_name_goes_into_a_message_as_it_is() {
    let name = RuleName::try_from("api-v2").expect("a name");
    let message = Message::new("rule ").then(MessagePart::printable(&name));
    assert_eq!(message.to_string(), "rule api-v2");
}

#[test]
fn a_glob_is_text_with_a_character_that_is_not_white_space() {
    assert!(Glob::try_from("src/**").is_ok());
    assert!(Glob::try_from(" a ").is_ok());
    for text in ["", " ", "\t\n", "\u{a0}"] {
        assert!(Glob::try_from(text).is_err(), "{text:?}");
    }
}

#[test]
fn a_glob_is_kept_as_written() {
    assert_eq!(Glob::try_from(" a ").expect("a glob").as_str(), " a ");
}

#[test]
fn the_modes_are_replace_and_extend_and_nothing_else() {
    assert_eq!(RuleMode::try_from("replace"), Ok(RuleMode::Replace));
    assert_eq!(RuleMode::try_from("extend"), Ok(RuleMode::Extend));
    for text in ["", "Replace", "append", "add", "remove"] {
        assert!(RuleMode::try_from(text).is_err(), "{text:?}");
    }
}

proptest! {
    #[test]
    fn a_name_is_accepted_exactly_when_it_matches_the_kebab_grammar(text in "[a-z0-9A-Z_. -]{0,8}") {
        let groups_ok = text
            .split('-')
            .all(|group| !group.is_empty() && group.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit()));
        prop_assert_eq!(accepts_name(&text), groups_ok);
    }

    #[test]
    fn names_order_by_code_point(left in "[a-z0-9-]{1,6}", right in "[a-z0-9-]{1,6}") {
        if let (Ok(a), Ok(b)) = (RuleName::try_from(left.as_str()), RuleName::try_from(right.as_str())) {
            prop_assert!((a < b) == (left < right));
        }
    }
}
