use super::check;
use super::forbid::{MAX_ENTRIES, MAX_ENTRY_CHARS};
use crate::layer::{LayerContents, LayerMeta};
use crate::lint::parse_document;
use crate::path::SafeRelPath;

/// A layer with `visibility` and the pattern entries `patterns` (each written as a YAML
/// single-quoted scalar), holding `files` besides its `layer.yaml`.
fn layer(
    visibility: &str,
    patterns: &[String],
    files: &[(&str, Vec<u8>)],
) -> (LayerMeta, LayerContents) {
    let entries: String = patterns
        .iter()
        .map(|pattern| ["  - '", &pattern.replace('\'', "''"), "'\n"].concat())
        .collect();
    let field = if patterns.is_empty() {
        String::new()
    } else {
        format!("forbid_patterns:\n{entries}")
    };
    let yaml = format!("name: shared\nkind: personal\nvisibility: {visibility}\n{field}");
    let meta =
        LayerMeta::from_yaml(&parse_document(yaml.as_bytes()).expect("YAML")).expect("layer");
    let contents = std::iter::once(("layer.yaml", yaml.into_bytes()))
        .chain(files.iter().map(|(path, bytes)| (*path, bytes.clone())))
        .map(|(path, bytes)| (SafeRelPath::try_from(path).expect("path"), bytes))
        .collect();
    (meta, contents)
}

fn lines(visibility: &str, patterns: &[String], files: &[(&str, &str)]) -> Vec<String> {
    let files: Vec<(&str, Vec<u8>)> = files
        .iter()
        .map(|(path, text)| (*path, text.as_bytes().to_vec()))
        .collect();
    let (meta, contents) = layer(visibility, patterns, &files);
    check(&meta, &contents)
        .iter()
        .map(ToString::to_string)
        .collect()
}

fn one(pattern: &str) -> Vec<String> {
    lines("public", &[pattern.to_owned()], &[])
}

/// A token assembled at run time, so that no file of the repository holds a token-shaped string.
fn token() -> String {
    format!("{}_{}", "ghp", "aB3".repeat(13))
}

#[test]
fn a_layer_that_is_not_public_is_not_checked() {
    for visibility in ["private", "confidential"] {
        let found = lines(
            visibility,
            &["acme".to_owned()],
            &[("a.md", &format!("acme {} dev@acme.io build.corp", token()))],
        );
        assert!(found.is_empty(), "{visibility}: {found:?}");
    }
}

#[test]
fn a_match_names_the_layer_the_file_and_the_entry_number_and_never_the_text() {
    let found = lines(
        "public",
        &["nothing".to_owned(), "(?i)\\bacme\\b".to_owned()],
        &[("skills/billing/SKILL.md", "Bill ACME monthly\n")],
    );
    assert_eq!(
        found,
        ["error [leak-forbid] shared:skills/billing/SKILL.md: matches forbid_patterns entry #2"]
    );
}

#[test]
fn a_pattern_is_matched_against_the_whole_text_of_the_file() {
    let found = lines(
        "public",
        &["(?s)one.*three".to_owned()],
        &[("a.md", "one\ntwo\nthree\n")],
    );
    assert_eq!(found.len(), 1, "{found:?}");
}

#[test]
fn a_line_anchor_matches_at_each_line() {
    for text in [
        "first\nacme\nlast\n",
        "first\r\nacme\r\nlast\r\n",
        "acme\nlast",
        "first\nacme",
    ] {
        let found = lines("public", &["^acme$".to_owned()], &[("a.md", text)]);
        assert_eq!(found.len(), 1, "{text:?}: {found:?}");
    }
    let found = lines(
        "public",
        &["^acme$".to_owned()],
        &[("a.md", "first\nacme corp\nnot acme\n")],
    );
    assert!(found.is_empty(), "{found:?}");
}

#[test]
fn the_patterns_in_layer_yaml_do_not_match_themselves_but_the_rest_of_the_file_does() {
    let (meta, contents) = layer("public", &["shared-secret-word".to_owned()], &[]);
    assert!(check(&meta, &contents).iter().next().is_none());
    let yaml = "name: shared\nkind: personal\nvisibility: public\nforbid_patterns:\n  - acme\ndescription: for acme\n";
    let meta =
        LayerMeta::from_yaml(&parse_document(yaml.as_bytes()).expect("YAML")).expect("layer");
    let contents: LayerContents = [(
        SafeRelPath::try_from("layer.yaml").expect("path"),
        yaml.as_bytes().to_vec(),
    )]
    .into_iter()
    .collect();
    let found: Vec<String> = check(&meta, &contents)
        .iter()
        .map(ToString::to_string)
        .collect();
    assert_eq!(
        found,
        ["error [leak-forbid] shared:layer.yaml: matches forbid_patterns entry #1"]
    );
}

#[test]
fn a_secret_is_named_by_its_kind_and_never_printed() {
    let secret = token();
    let found = lines(
        "public",
        &[],
        &[("instructions/10-ci.md", &format!("- CI: {secret}\n"))],
    );
    assert_eq!(
        found,
        [
            "error [leak-secret] shared:instructions/10-ci.md: looks like a secret (GitHub token); remove it from the public layer"
        ]
    );
    assert!(found.iter().all(|line| !line.contains(&secret)));
}

#[test]
fn every_file_is_scanned_including_binary_and_not_utf8_ones() {
    let secret = token();
    let mut bytes = vec![0_u8, 159, 146, 150];
    bytes.extend_from_slice(secret.as_bytes());
    let (meta, contents) = layer("public", &[], &[("assets/blob.bin", bytes)]);
    let found: Vec<String> = check(&meta, &contents)
        .iter()
        .map(ToString::to_string)
        .collect();
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(found[0].starts_with("error [leak-secret] shared:assets/blob.bin:"));
}

#[test]
fn an_email_and_a_private_host_are_warnings_once_for_each_file() {
    let found = lines(
        "public",
        &[],
        &[(
            "a.md",
            "dev@acme.io and again ops@acme.io; git.corp and wiki.corp\n",
        )],
    );
    assert_eq!(
        found,
        [
            "warning [leak-email] shared:a.md: contains an email address; check that it may be public",
            "warning [leak-host] shared:a.md: contains what looks like a private host name; check that it may be public",
        ]
    );
}

#[test]
fn public_addresses_do_not_warn() {
    let found = lines(
        "public",
        &[],
        &[(
            "a.md",
            "1+me@users.noreply.github.com, a@example.com, b@anthropic.com\n",
        )],
    );
    assert!(found.is_empty(), "{found:?}");
}

#[test]
fn a_look_around_or_a_backreference_is_refused_naming_the_feature() {
    for pattern in ["a(?=b)", "a(?!b)", "(?<=a)b", "(?<!a)b"] {
        assert_eq!(
            one(pattern),
            [
                "error [leak-forbid] shared:layer.yaml: forbid_patterns entry #1 uses look-around, which is not supported by the pattern syntax"
            ],
            "{pattern}"
        );
    }
    for pattern in [r"(a)\1", r"(?<n>a)\k<n>", r"(a)(b)\2"] {
        assert_eq!(
            one(pattern),
            [
                "error [leak-forbid] shared:layer.yaml: forbid_patterns entry #1 uses a backreference, which is not supported by the pattern syntax"
            ],
            "{pattern}"
        );
    }
}

#[test]
fn an_escaped_or_classed_look_around_is_text_and_compiles() {
    for pattern in [r"\(?=x", r"[(?=]", r"a\\1", "[^]](x)", "[](?=]"] {
        let found = one(pattern);
        assert!(
            found.iter().all(|line| !line.contains("not supported")),
            "{pattern}: {found:?}"
        );
    }
}

/// §9.2 `forbid_patterns`: `\d`, `\s`, `\w` and `\b` are ASCII only, and `(?i)` folds ASCII
/// letters only (decision-38).
#[test]
fn the_classes_and_the_case_folding_of_a_pattern_are_ascii_only() {
    let matches = |pattern: &str, text: &str| {
        // The text of `layer.yaml` is matched too, so only the lines about `a.md` count.
        lines("public", &[pattern.to_owned()], &[("a.md", text)])
            .iter()
            .filter(|line| line.contains("shared:a.md:"))
            .count()
            == 1
    };
    // The ASCII cases match, so that a "no match" below is the rule and not a broken pattern.
    assert!(matches(r"^\w+$", "a_Z9"));
    assert!(matches(r"^\d$", "7"));
    assert!(matches(r"^\s$", " "));
    assert!(matches(r"(?i)acme", "ACME"));
    // The Unicode cases do not.
    assert!(
        !matches(r"\w", "\u{e9}"),
        "\\w holds no letter beyond ASCII"
    );
    assert!(
        !matches(r"\d", "\u{663}"),
        "\\d holds no digit beyond ASCII"
    );
    assert!(
        !matches(r"\s", "\u{3000}\u{a0}"),
        "\\s holds no space beyond ASCII"
    );
    assert!(
        matches(r"\bb", "\u{e9}b"),
        "\\b sees a letter beyond ASCII as no word character"
    );
    assert!(
        !matches("(?i)\u{e9}", "\u{c9}"),
        "(?i) folds no letter beyond ASCII"
    );
    assert!(
        !matches(r"(?i)k", "\u{212a}"),
        "(?i) folds no letter beyond ASCII"
    );
}

#[test]
fn an_entry_that_does_not_compile_is_refused_by_its_number() {
    let found = lines(
        "public",
        &[
            "fine".to_owned(),
            "(unclosed".to_owned(),
            r"\p{L}".to_owned(),
        ],
        &[],
    );
    assert_eq!(
        found,
        [
            "error [leak-forbid] shared:layer.yaml: forbid_patterns entry #2 is not a valid pattern, or compiles to more than 64 KiB or nests more than 32 levels",
            "error [leak-forbid] shared:layer.yaml: forbid_patterns entry #3 is not a valid pattern, or compiles to more than 64 KiB or nests more than 32 levels",
        ]
    );
}

#[test]
fn an_entry_past_the_size_or_the_nest_limit_is_refused() {
    let nested = format!("{}a{}", "(".repeat(40), ")".repeat(40));
    let big = "a{100}{100}".to_owned();
    let wide = "(?:[a-z]{1,50}){1,50}".to_owned();
    let found = lines("public", &[nested, big, wide], &[]);
    assert_eq!(found.len(), 3, "{found:?}");
    assert!(
        found
            .iter()
            .all(|line| line.contains("compiles to more than 64 KiB"))
    );
}

#[test]
fn an_entry_longer_than_512_characters_is_refused_and_one_of_512_is_not() {
    assert_eq!(
        one(&"a".repeat(MAX_ENTRY_CHARS + 1)),
        [
            "error [leak-forbid] shared:layer.yaml: forbid_patterns entry #1 is longer than 512 characters"
        ]
    );
    assert_eq!(one(&"a".repeat(MAX_ENTRY_CHARS)), Vec::<String>::new());
    assert_eq!(one(&"é".repeat(MAX_ENTRY_CHARS)), Vec::<String>::new());
}

#[test]
fn the_65th_entry_is_refused_and_the_first_64_are_used() {
    let patterns: Vec<String> = (1..=MAX_ENTRIES + 3).map(|n| format!("word{n}x")).collect();
    let found = lines("public", &patterns, &[("a.md", "word64x word65x\n")]);
    assert_eq!(
        found,
        [
            "error [leak-forbid] shared:layer.yaml: forbid_patterns entry #65 is past the 64 entries a layer may have; it and every later entry are refused",
            "error [leak-forbid] shared:a.md: matches forbid_patterns entry #64",
        ]
    );
}

#[test]
fn no_diagnostic_holds_a_refused_entry_or_a_matched_text() {
    let entries = [
        "(?=hidden-entry-text)".to_owned(),
        "(hidden-entry-text".to_owned(),
        format!("hidden-entry-text{}", "a".repeat(MAX_ENTRY_CHARS)),
        "matched-hidden-text".to_owned(),
    ];
    let found = lines("public", &entries, &[("a.md", "matched-hidden-text\n")]);
    assert_eq!(found.len(), 4, "{found:?}");
    assert!(
        found.iter().all(|line| !line.contains("hidden")),
        "{found:?}"
    );
}

/// What `check` says of a public layer whose `layer.yaml` is `yaml` exactly.
fn lines_of_yaml(yaml: &str) -> Vec<String> {
    let meta =
        LayerMeta::from_yaml(&parse_document(yaml.as_bytes()).expect("YAML")).expect("layer");
    let contents: LayerContents = [(
        SafeRelPath::try_from("layer.yaml").expect("path"),
        yaml.as_bytes().to_vec(),
    )]
    .into_iter()
    .collect();
    check(&meta, &contents)
        .iter()
        .map(ToString::to_string)
        .collect()
}

#[test]
fn a_token_in_a_forbid_patterns_line_is_reported() {
    let secret = token();
    let head = "name: shared\nkind: personal\nvisibility: public\nforbid_patterns:\n";
    let entry = format!("{head}  - '{secret}'\n");
    let comment = format!("{head}  - acme # {secret}\n");
    for yaml in [entry, comment] {
        let found = lines_of_yaml(&yaml);
        assert!(
            found
                .iter()
                .any(|line| line.starts_with("error [leak-secret] shared:layer.yaml:")),
            "{found:?}"
        );
        assert!(found.iter().all(|line| !line.contains(&secret)));
    }
}

#[test]
fn an_address_or_a_host_in_a_forbid_patterns_line_is_warned_about() {
    let yaml = "name: shared\nkind: personal\nvisibility: public\nforbid_patterns:\n  - acme # ask dev@acme.io at wiki.corp\n";
    assert_eq!(
        lines_of_yaml(yaml),
        [
            "warning [leak-email] shared:layer.yaml: contains an email address; check that it may be public",
            "warning [leak-host] shared:layer.yaml: contains what looks like a private host name; check that it may be public",
        ]
    );
}

#[test]
fn a_token_in_a_utf16_file_is_found_in_either_byte_order() {
    let text = format!("- CI: {}\n", token());
    let little: Vec<u8> = [0xFF, 0xFE]
        .into_iter()
        .chain(text.encode_utf16().flat_map(u16::to_le_bytes))
        .collect();
    let big: Vec<u8> = [0xFE, 0xFF]
        .into_iter()
        .chain(text.encode_utf16().flat_map(u16::to_be_bytes))
        .collect();
    for bytes in [little, big] {
        let (meta, contents) = layer("public", &[], &[("a.md", bytes)]);
        let found: Vec<String> = check(&meta, &contents)
            .iter()
            .map(ToString::to_string)
            .collect();
        assert_eq!(
            found,
            [
                "error [leak-secret] shared:a.md: looks like a secret (GitHub token); remove it from the public layer"
            ]
        );
    }
}

#[test]
fn a_pattern_is_matched_against_the_decoded_text_of_a_utf16_file() {
    let bytes: Vec<u8> = [0xFF, 0xFE]
        .into_iter()
        .chain("bill acme\n".encode_utf16().flat_map(u16::to_le_bytes))
        .collect();
    let (meta, contents) = layer("public", &["acme".to_owned()], &[("a.md", bytes)]);
    let found: Vec<String> = check(&meta, &contents)
        .iter()
        .map(ToString::to_string)
        .collect();
    assert_eq!(found.len(), 1, "{found:?}");
}

#[test]
fn the_diagnostics_come_in_one_fixed_order() {
    let found = lines(
        "public",
        &["(?=x)".to_owned(), "mm".to_owned()],
        &[
            ("b.md", "dev@acme.io mm"),
            ("a.md", &format!("mm {}", token())),
        ],
    );
    let heads: Vec<&str> = found
        .iter()
        .map(|line| line.split(": ").next().unwrap_or(""))
        .collect();
    assert_eq!(
        heads,
        [
            "error [leak-forbid] shared:layer.yaml",
            "error [leak-forbid] shared:a.md",
            "error [leak-secret] shared:a.md",
            "error [leak-forbid] shared:b.md",
            "warning [leak-email] shared:b.md",
        ]
    );
}
