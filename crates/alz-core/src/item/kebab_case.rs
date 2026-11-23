//! The grammar of item names: kebab-case, `^[a-z0-9]+(?:-[a-z0-9]+)*$` (spec §7.2, §7.3, §7.10).

/// Whether `text` is lower-case letters and digits in non-empty groups joined by `-`.
pub(super) fn is_kebab_case(text: &str) -> bool {
    text.split('-').all(|group| {
        !group.is_empty()
            && group
                .bytes()
                .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit())
    })
}

#[cfg(test)]
mod tests {
    use super::is_kebab_case;

    #[test]
    fn kebab_case_is_groups_of_lower_case_letters_and_digits_joined_by_one_dash() {
        for text in ["a", "a-b", "a1-b2-3", "0"] {
            assert!(is_kebab_case(text), "{text}");
        }
        for text in ["", "-a", "a-", "a--b", "A", "a_b", "a.b", "a b", "é", "a/b"] {
            assert!(!is_kebab_case(text), "{text:?}");
        }
    }
}
