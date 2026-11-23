//! The Unicode format characters (general category Cf) of Unicode 17.0, the version whose `\p{Cf}`
//! Node 22 uses, so the Rust and the TypeScript `alz` strip the same text (decision-26).
//!
//! The table is written by hand. The test below checks it against `extracted/DerivedGeneralCategory.txt`
//! of UCD 17.0, vendored under `tests/fixtures/unicode/`; when they differ it prints the ranges to paste.

/// The Unicode format characters (category Cf) as inclusive ranges, sorted and disjoint.
pub(super) const FORMAT_CHARACTERS: &[(char, char)] = &[
    ('\u{ad}', '\u{ad}'),
    ('\u{600}', '\u{605}'),
    ('\u{61c}', '\u{61c}'),
    ('\u{6dd}', '\u{6dd}'),
    ('\u{70f}', '\u{70f}'),
    ('\u{890}', '\u{891}'),
    ('\u{8e2}', '\u{8e2}'),
    ('\u{180e}', '\u{180e}'),
    ('\u{200b}', '\u{200f}'),
    ('\u{202a}', '\u{202e}'),
    ('\u{2060}', '\u{2064}'),
    ('\u{2066}', '\u{206f}'),
    ('\u{feff}', '\u{feff}'),
    ('\u{fff9}', '\u{fffb}'),
    ('\u{110bd}', '\u{110bd}'),
    ('\u{110cd}', '\u{110cd}'),
    ('\u{13430}', '\u{1343f}'),
    ('\u{1bca0}', '\u{1bca3}'),
    ('\u{1d173}', '\u{1d17a}'),
    ('\u{e0001}', '\u{e0001}'),
    ('\u{e0020}', '\u{e007f}'),
];

#[cfg(test)]
mod tests {
    use super::FORMAT_CHARACTERS;

    /// The Cf lines of the vendored UCD extract, as inclusive ranges.
    fn ranges_of_the_unicode_data() -> Vec<(char, char)> {
        include_str!("../../tests/fixtures/unicode/DerivedGeneralCategory-17.0.0-Cf.txt")
            .lines()
            .filter_map(|line| line.split_once(';'))
            .filter(|(_, rest)| rest.trim_start().starts_with("Cf "))
            .map(|(code_points, _)| code_points.trim())
            .map(|code_points| {
                let (first, last) = code_points
                    .split_once("..")
                    .unwrap_or((code_points, code_points));
                (code_point(first), code_point(last))
            })
            .collect()
    }

    fn code_point(hexadecimal: &str) -> char {
        u32::from_str_radix(hexadecimal, 16)
            .ok()
            .and_then(char::from_u32)
            .expect("a hexadecimal code point")
    }

    #[test]
    fn the_table_equals_the_format_characters_of_unicode_17() {
        let expected = ranges_of_the_unicode_data();
        let paste = expected
            .iter()
            .map(|(first, last)| {
                format!(
                    "    ('\\u{{{:x}}}', '\\u{{{:x}}}'),",
                    u32::from(*first),
                    u32::from(*last)
                )
            })
            .collect::<Vec<_>>()
            .join("\n");
        assert_eq!(
            FORMAT_CHARACTERS,
            expected.as_slice(),
            "FORMAT_CHARACTERS differs from UCD 17.0; paste these ranges:\n{paste}"
        );
    }

    #[test]
    fn the_unicode_data_holds_170_format_characters() {
        // The file's own total ("# Total code points: 170"), so a truncated extract fails here.
        let total: u32 = ranges_of_the_unicode_data()
            .iter()
            .map(|(first, last)| u32::from(*last) - u32::from(*first) + 1)
            .sum();
        assert_eq!(total, 170);
    }
}
