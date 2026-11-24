//! alz's own stable JSON emitter (decision-33, spec §6 "Name order"): two-space indentation
//! and a line feed at the end, with the keys of every object in code point order, which is the byte
//! order of their UTF-8 text and so Rust's `str` order. A key is never ordered by UTF-16 unit, and
//! a key that is a whole number is a plain string in that order (TASK-64).
//!
//! [`emit`] is pure and total. Nesting is as deep as the value is; a value that alz builds from
//! typed data is shallow, and one read from a file is at most 128 deep (`json::MAX_DEPTH`).

use crate::json::{JsonNumber, JsonObject, JsonValue};

/// One level of indentation: two spaces.
const INDENT_STEP: &str = "  ";

/// `value` as stable JSON: two-space indent, keys in code point order, a final line feed.
#[must_use]
pub fn emit(value: &JsonValue) -> String {
    format!("{}\n", render(value, 0))
}

fn render(value: &JsonValue, level: usize) -> String {
    match value {
        JsonValue::Null => "null".to_owned(),
        JsonValue::Bool(flag) => flag.to_string(),
        JsonValue::Number(number) => number_text(*number),
        JsonValue::Text(text) => quote(text),
        JsonValue::Array(items) => block(
            ("[", "]"),
            level,
            items
                .iter()
                .map(|item| render(item, level.saturating_add(1))),
        ),
        JsonValue::Object(object) => block(
            ("{", "}"),
            level,
            sorted(object).map(|(key, item)| {
                format!("{}: {}", quote(key), render(item, level.saturating_add(1)))
            }),
        ),
    }
}

/// The entries of `object` in name order.
fn sorted(object: &JsonObject) -> impl Iterator<Item = (&str, &JsonValue)> + '_ {
    let mut entries: Vec<_> = object.iter().collect();
    entries.sort_by_key(|(key, _)| *key);
    entries.into_iter()
}

/// The lines of one array or object between its brackets, or the bare brackets when it is empty.
fn block((open, close): (&str, &str), level: usize, lines: impl Iterator<Item = String>) -> String {
    let inner = INDENT_STEP.repeat(level.saturating_add(1));
    let body = lines
        .map(|line| format!("{inner}{line}"))
        .collect::<Vec<_>>()
        .join(",\n");
    if body.is_empty() {
        format!("{open}{close}")
    } else {
        format!("{open}\n{body}\n{}{close}", INDENT_STEP.repeat(level))
    }
}

/// A JSON string, escaped as `JSON.stringify` does: the quote, the backslash, `\b \f \n \r \t`,
/// and every other control character below U+0020 as `\u00xx` in lower case. Everything else,
/// U+007F and U+2028 included, is written as it is.
fn quote(text: &str) -> String {
    let escaped: String = text
        .chars()
        .map(|character| match character {
            '"' => "\\\"".to_owned(),
            '\\' => "\\\\".to_owned(),
            '\u{8}' => "\\b".to_owned(),
            '\u{c}' => "\\f".to_owned(),
            '\n' => "\\n".to_owned(),
            '\r' => "\\r".to_owned(),
            '\t' => "\\t".to_owned(),
            control if control < ' ' => format!("\\u{:04x}", u32::from(control)),
            other => other.to_string(),
        })
        .collect();
    format!("\"{escaped}\"")
}

fn number_text(number: JsonNumber) -> String {
    match number {
        JsonNumber::Int(whole) => whole.to_string(),
        JsonNumber::Float(float) => float_text(float),
    }
}

/// A float as the shortest digits that read
/// back as the same number, in positional form from 1e-6 up to 1e21 and in exponent form outside.
/// alz writes `null` for a number that is not finite, and `0` for negative zero.
fn float_text(float: f64) -> String {
    if !float.is_finite() {
        return "null".to_owned();
    }
    if float == 0.0 {
        return "0".to_owned();
    }
    let sign = if float < 0.0 { "-" } else { "" };
    let scientific = format!("{:e}", float.abs());
    let (mantissa, exponent) = scientific.split_once('e').unwrap_or((&scientific, "0"));
    let digits: String = mantissa.chars().filter(char::is_ascii_digit).collect();
    // The decimal point sits after `point` digits: 1.5e0 has digits "15" and point 1.
    let point = exponent.parse::<i64>().unwrap_or(0).saturating_add(1);
    let count = i64::try_from(digits.len()).unwrap_or(i64::MAX);
    let body = if count <= point && point <= 21 {
        let zeros = usize::try_from(point.saturating_sub(count)).unwrap_or(0);
        format!("{digits}{}", "0".repeat(zeros))
    } else if 0 < point && point <= 21 {
        let (whole, fraction) = digits
            .split_at_checked(usize::try_from(point).unwrap_or(0))
            .unwrap_or((&digits, ""));
        format!("{whole}.{fraction}")
    } else if -6 < point && point <= 0 {
        let zeros = usize::try_from(point.saturating_neg()).unwrap_or(0);
        format!("0.{}{digits}", "0".repeat(zeros))
    } else {
        let power = point.saturating_sub(1);
        let (first, rest) = digits.split_at_checked(1).unwrap_or((&digits, ""));
        let fraction = if rest.is_empty() {
            String::new()
        } else {
            format!(".{rest}")
        };
        let power_sign = if power < 0 { "-" } else { "+" };
        format!("{first}{fraction}e{power_sign}{}", power.unsigned_abs())
    };
    format!("{sign}{body}")
}

#[cfg(test)]
mod tests;
