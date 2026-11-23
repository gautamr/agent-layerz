//! How a piece of text is written: plain, quoted or as a block. The rules are the reference's
//! (the `yaml` package, 2.9.1, with line folding off), tested against its samples. In short:
//!
//! 1. Text with a control character (U+0000 to U+0008, U+000B to U+001F, U+007F to U+009F) is
//!    always double-quoted.
//! 2. Otherwise text is plain unless YAML would read it differently: it starts with an
//!    indicator, holds `: ` or ` #`, ends in a space or a colon, is empty, or reads as null, a
//!    boolean, an integer or a float in the YAML 1.2 core schema. Such text is quoted, with
//!    single quotes if it holds a double quote and no single quote, and double quotes otherwise.
//! 3. Multi-line text is a literal block (`|`), unless it is a key, which is quoted.
//! 4. A key of the top level that starts a line with `---`, `...` or `%` is quoted.

use std::iter;

use super::INDENT_STEP;

/// How the text is to be written once the rules above have decided.
enum Style {
    Plain,
    Quoted,
    Block { force_indent: bool },
}

/// `text` as a scalar. `implicit_key` says it is a mapping key, and `indent` is the indent of
/// the context the reference gives it: the key's or value's own nesting, not the entry's.
pub(super) fn scalar(text: &str, implicit_key: bool, indent: &str) -> String {
    if has_control_character(text) {
        return double_quoted(text, implicit_key, indent);
    }
    if is_read_differently(text) {
        return quoted(text, implicit_key, indent);
    }
    match plain_style(text, implicit_key, indent) {
        Style::Plain => text.to_owned(),
        Style::Quoted => quoted(text, implicit_key, indent),
        Style::Block { force_indent } => block(text, indent, force_indent),
    }
}

/// Whether the reference would write the text plainly, quoted or as a block.
fn plain_style(text: &str, implicit_key: bool, indent: &str) -> Style {
    let multi_line = text.contains('\n');
    if implicit_key && multi_line {
        return Style::Quoted;
    }
    if needs_quotes_or_block(text) {
        return if implicit_key || !multi_line {
            Style::Quoted
        } else {
            Style::Block {
                force_indent: false,
            }
        };
    }
    if !implicit_key && multi_line {
        return Style::Block {
            force_indent: false,
        };
    }
    if has_document_marker(text) {
        if indent.is_empty() {
            return Style::Block { force_indent: true };
        }
        if implicit_key && indent == INDENT_STEP {
            return Style::Quoted;
        }
    }
    if reads_as_non_text(text) {
        Style::Quoted
    } else {
        Style::Plain
    }
}

/// Text that the reference writes without quotes and alz's reader reads back as another value or
/// refuses, so alz quotes it (the round-trip law; the samples of class
/// `reference-reads-differently` pin each case): text that starts with a byte order mark, and text
/// of only spaces and line feeds that has both, which a block would not keep. Signed and binary
/// integers are the third case, in `is_integer`.
fn is_read_differently(text: &str) -> bool {
    let only_blanks_and_line_feeds = text.chars().all(|c| matches!(c, ' ' | '\n'));
    text.starts_with('\u{feff}')
        || (only_blanks_and_line_feeds && text.contains(' ') && text.contains('\n'))
}

/// U+0000 to U+0008, U+000B to U+001F and U+007F to U+009F: not tab and not line feed.
fn has_control_character(text: &str) -> bool {
    text.chars()
        .any(|c| matches!(c, '\u{0}'..='\u{8}' | '\u{b}'..='\u{1f}' | '\u{7f}'..='\u{9f}'))
}

/// The reference's test for text that plain style cannot hold.
fn needs_quotes_or_block(text: &str) -> bool {
    let mut chars = text.chars();
    let first = chars.next();
    let second = chars.next();
    let starts_badly = first.is_some_and(|c| {
        matches!(
            c,
            '\n' | '\t'
                | ' '
                | ','
                | '['
                | ']'
                | '{'
                | '}'
                | '#'
                | '&'
                | '*'
                | '!'
                | '|'
                | '>'
                | '\''
                | '"'
                | '%'
                | '@'
                | '`'
        )
    });
    let lone_indicator = matches!(text, "?" | "-");
    let indicator_then_space =
        matches!(first, Some('?' | '-')) && matches!(second, Some(' ' | '\t'));
    let ends_badly = text
        .chars()
        .next_back()
        .is_some_and(|c| matches!(c, '\n' | '\t' | ' ' | ':'));
    let inner_trouble = text.chars().zip(text.chars().skip(1)).any(|pair| {
        matches!(
            pair,
            ('\n' | ':', ' ' | '\t') | (' ' | '\t', '\n') | ('\n' | '\t' | ' ', '#')
        )
    });
    starts_badly || lone_indicator || indicator_then_space || ends_badly || inner_trouble
}

/// A line that starts with `%`, `---` or `...`. A line starts after a line feed, a carriage
/// return, U+2028 or U+2029.
fn has_document_marker(text: &str) -> bool {
    text.split(['\n', '\r', '\u{2028}', '\u{2029}'])
        .any(|line| line.starts_with(['%']) || line.starts_with("---") || line.starts_with("..."))
}

/// Whether the YAML 1.2 core schema reads plain `text` as null, a boolean, an integer or a float.
fn reads_as_non_text(text: &str) -> bool {
    is_null(text) || is_bool(text) || is_integer(text) || is_float(text)
}

fn is_null(text: &str) -> bool {
    matches!(text, "" | "~" | "null" | "Null" | "NULL")
}

fn is_bool(text: &str) -> bool {
    matches!(text, "true" | "True" | "TRUE" | "false" | "False" | "FALSE")
}

fn digits(text: &str) -> bool {
    !text.is_empty() && text.chars().all(|c| c.is_ascii_digit())
}

fn without_sign(text: &str) -> &str {
    text.strip_prefix(['-', '+']).unwrap_or(text)
}

/// Decimal integers with an optional sign, and `0o` and `0x` integers, which the core schema has
/// without a sign. alz's reader also reads a sign on those and the `0b` form, so the emitter
/// quotes them where the reference does not (a deviation, with `is_read_differently`).
fn is_integer(text: &str) -> bool {
    let unsigned = without_sign(text);
    let radix = |prefix: &str, valid: fn(char) -> bool| {
        unsigned
            .strip_prefix(prefix)
            .is_some_and(|rest| !rest.is_empty() && rest.chars().all(valid))
    };
    digits(unsigned)
        || radix("0o", |c| matches!(c, '0'..='7'))
        || radix("0x", |c| c.is_ascii_hexdigit())
        || radix("0b", |c| matches!(c, '0' | '1'))
}

fn is_float(text: &str) -> bool {
    let unsigned = without_sign(text);
    matches!(text, ".nan" | ".NaN" | ".NAN")
        || matches!(unsigned, ".inf" | ".Inf" | ".INF")
        || is_fraction(unsigned)
        || unsigned
            .split_once(['e', 'E'])
            .is_some_and(|(mantissa, exponent)| {
                (digits(mantissa) || is_fraction(mantissa)) && digits(without_sign(exponent))
            })
}

/// Digits, a point and more digits, with digits on at least one side of the point.
fn is_fraction(text: &str) -> bool {
    text.split_once('.').is_some_and(|(whole, fraction)| {
        (whole.is_empty() || digits(whole))
            && (fraction.is_empty() || digits(fraction))
            && !(whole.is_empty() && fraction.is_empty())
    })
}

/// Text that plain style cannot hold, in quotes.
fn quoted(text: &str, implicit_key: bool, indent: &str) -> String {
    if text.contains('"') && !text.contains('\'') {
        single_quoted(text, implicit_key, indent)
    } else {
        double_quoted(text, implicit_key, indent)
    }
}

/// The indent that continuation lines of quoted text take: the context's, or two spaces when the
/// context has none and a line would start like a document marker.
fn continuation_indent<'a>(text: &str, indent: &'a str) -> &'a str {
    if !indent.is_empty() {
        indent
    } else if has_document_marker(text) {
        INDENT_STEP
    } else {
        ""
    }
}

fn single_quoted(text: &str, implicit_key: bool, indent: &str) -> String {
    let blank_next_to_line_break = text
        .chars()
        .zip(text.chars().skip(1))
        .any(|pair| matches!(pair, (' ' | '\t', '\n') | ('\n', ' ' | '\t')));
    if (implicit_key && text.contains('\n')) || blank_next_to_line_break {
        return double_quoted(text, implicit_key, indent);
    }
    let line_break = format!("\n{}", continuation_indent(text, indent));
    format!(
        "'{}'",
        after_line_feed_runs(&text.replace('\'', "''"), &line_break, true)
    )
}

/// `text` with `suffix` after every run of line feeds, or after every run that is not at the end
/// of `text` when `at_end` is false.
fn after_line_feed_runs(text: &str, suffix: &str, at_end: bool) -> String {
    text.chars()
        .zip(text.chars().skip(1).map(Some).chain(iter::once(None)))
        .fold(String::new(), |mut out, (c, next)| {
            out.push(c);
            if c == '\n' && next != Some('\n') && (next.is_some() || at_end) {
                out.push_str(suffix);
            }
            out
        })
}

/// `text` as JSON writes a string, which is where the reference starts a double-quoted scalar.
fn json_string(text: &str) -> String {
    let body: String = text
        .chars()
        .map(|c| match c {
            '"' => "\\\"".to_owned(),
            '\\' => "\\\\".to_owned(),
            '\u{8}' => "\\b".to_owned(),
            '\u{c}' => "\\f".to_owned(),
            '\n' => "\\n".to_owned(),
            '\r' => "\\r".to_owned(),
            '\t' => "\\t".to_owned(),
            c if c < ' ' => format!("\\u{:04x}", u32::from(c)),
            c => c.to_string(),
        })
        .collect();
    format!("\"{body}\"")
}

/// A JSON `\uXXXX` escape of a control character, as YAML writes it.
fn yaml_escape(code: &str, json_escape: &str) -> String {
    match code {
        "0000" => "\\0".to_owned(),
        "0007" => "\\a".to_owned(),
        "000b" => "\\v".to_owned(),
        "001b" => "\\e".to_owned(),
        "0085" => "\\N".to_owned(),
        "00a0" => "\\_".to_owned(),
        "2028" => "\\L".to_owned(),
        "2029" => "\\P".to_owned(),
        _ => code
            .strip_prefix("00")
            .map_or_else(|| json_escape.to_owned(), |low| format!("\\x{low}")),
    }
}

/// Below this many UTF-16 code units of JSON a line feed in double quotes stays an escape.
const MIN_MULTI_LINE_UNITS: usize = 40;

/// Double-quoted text as the reference writes it: JSON with YAML's escapes, and where the text is
/// long and holds line feeds, real line breaks (a blank line stands for a line feed). It walks the
/// JSON text with a cursor because each escape decides how many characters follow it.
fn double_quoted(text: &str, implicit_key: bool, indent: &str) -> String {
    let json: Vec<char> = json_string(text).chars().collect();
    let units: usize = json.iter().map(|c| c.len_utf16()).sum();
    let line_indent = continuation_indent(text, indent);
    let at = |index: usize| json.get(index).copied();
    let slice = |from: usize, to: usize| -> String {
        json.iter()
            .skip(from)
            .take(to.saturating_sub(from))
            .collect()
    };

    let mut out = String::new();
    let mut start = 0_usize;
    let mut cursor = 0_usize;
    while let Some(first) = at(cursor) {
        let mut current = first;
        if current == ' ' && at(cursor + 1) == Some('\\') && at(cursor + 2) == Some('n') {
            // A space before a line feed is escaped, or the break would swallow it.
            out.push_str(&slice(start, cursor));
            out.push_str("\\ ");
            cursor += 1;
            start = cursor;
            current = '\\';
        }
        if current == '\\' {
            match at(cursor + 1) {
                Some('u') => {
                    out.push_str(&slice(start, cursor));
                    out.push_str(&yaml_escape(
                        &slice(cursor + 2, cursor + 6),
                        &slice(cursor, cursor + 6),
                    ));
                    cursor += 5;
                    start = cursor + 1;
                }
                Some('n')
                    if !(implicit_key
                        || at(cursor + 2) == Some('"')
                        || units < MIN_MULTI_LINE_UNITS) =>
                {
                    out.push_str(&slice(start, cursor));
                    out.push_str("\n\n");
                    while at(cursor + 2) == Some('\\')
                        && at(cursor + 3) == Some('n')
                        && at(cursor + 4) != Some('"')
                    {
                        out.push('\n');
                        cursor += 2;
                    }
                    out.push_str(line_indent);
                    if at(cursor + 2) == Some(' ') {
                        out.push('\\');
                    }
                    cursor += 1;
                    start = cursor + 1;
                }
                Some(_) | None => cursor += 1,
            }
        }
        cursor += 1;
    }
    if start == 0 {
        json.iter().collect()
    } else {
        out + &slice(start, json.len())
    }
}

/// Whether the text ends in blanks after a line feed, which a block cannot keep.
fn ends_in_blank_line(text: &str) -> bool {
    let trimmed = text.trim_end_matches([' ', '\t']);
    trimmed.len() < text.len() && trimmed.ends_with('\n')
}

/// Text as a literal block: `|`, a header that says how final line feeds and leading spaces are
/// kept, and the lines indented by `context_indent`, or by two spaces at the top level where a
/// line looks like a document marker or `force_indent` says so.
fn block(text: &str, context_indent: &str, force_indent: bool) -> String {
    if ends_in_blank_line(text) {
        return quoted(text, false, context_indent);
    }
    let indent = if !context_indent.is_empty() {
        context_indent
    } else if force_indent || has_document_marker(text) {
        INDENT_STEP
    } else {
        ""
    };
    if text.is_empty() {
        return "|\n".to_owned();
    }
    let body = text.trim_end_matches(['\n', '\t', ' ']);
    let end = text.strip_prefix(body).unwrap_or_default();
    let chomp = match end.find('\n') {
        None => "-",
        Some(first) if body.is_empty() || first != end.len() - 1 => "+",
        Some(_) => "",
    };
    let end = after_line_feed_runs(end.strip_suffix('\n').unwrap_or(end), indent, false);
    let lead = body.len() - body.trim_start_matches([' ', '\n']).len();
    let leading = body.get(..lead).unwrap_or_default();
    let start_with_space = leading.contains(' ');
    let (start, rest) = body
        .split_at_checked(leading.rfind('\n').map_or(0, |last| last + 1))
        .unwrap_or_default();
    let indent_size = if indent.is_empty() { "1" } else { "2" };
    let header = format!("{}{chomp}", if start_with_space { indent_size } else { "" });
    let start = after_line_feed_runs(start, indent, true);
    let rest = after_line_feed_runs(rest, indent, true);
    format!("|{header}\n{indent}{start}{rest}{end}")
}
