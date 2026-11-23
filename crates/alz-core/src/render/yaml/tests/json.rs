//! A small JSON reader for the golden samples: strings, booleans, integers, arrays and objects
//! with their keys in file order. The samples are JSON because the reference wrote them; no JSON
//! dependency is worth a decision for one fixture.

/// A JSON value of the sample file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Json {
    Text(String),
    Bool(bool),
    Int(i64),
    List(Vec<Json>),
    Object(Vec<(String, Json)>),
}

/// The JSON value of `text`, or what is wrong with it.
pub(super) fn parse(text: &str) -> Result<Json, String> {
    let chars: Vec<char> = text.chars().collect();
    let (value, end) = value(&chars, skip_blanks(&chars, 0))?;
    match chars.get(skip_blanks(&chars, end)) {
        None => Ok(value),
        Some(other) => Err(format!("unexpected {other:?} after the value")),
    }
}

fn skip_blanks(chars: &[char], from: usize) -> usize {
    from + chars
        .iter()
        .skip(from)
        .take_while(|c| c.is_ascii_whitespace())
        .count()
}

fn expect(chars: &[char], at: usize, wanted: char) -> Result<usize, String> {
    match chars.get(at) {
        Some(found) if *found == wanted => Ok(at + 1),
        other => Err(format!("expected {wanted:?} at {at}, found {other:?}")),
    }
}

fn word(chars: &[char], at: usize, word: &str) -> Option<usize> {
    let end = at + word.chars().count();
    chars
        .get(at..end)
        .filter(|found| found.iter().copied().eq(word.chars()))
        .map(|_| end)
}

fn value(chars: &[char], at: usize) -> Result<(Json, usize), String> {
    match chars.get(at) {
        Some('"') => string(chars, at + 1).map(|(text, end)| (Json::Text(text), end)),
        Some('[') => list(chars, skip_blanks(chars, at + 1)),
        Some('{') => object(chars, skip_blanks(chars, at + 1)),
        Some('-' | '0'..='9') => integer(chars, at),
        _ => word(chars, at, "true")
            .map(|end| (Json::Bool(true), end))
            .or_else(|| word(chars, at, "false").map(|end| (Json::Bool(false), end)))
            .ok_or_else(|| format!("unexpected value at {at}")),
    }
}

fn integer(chars: &[char], at: usize) -> Result<(Json, usize), String> {
    let length = chars
        .iter()
        .skip(at)
        .take_while(|c| c.is_ascii_digit() || **c == '-')
        .count();
    let text: String = chars.iter().skip(at).take(length).collect();
    text.parse()
        .map(|number| (Json::Int(number), at + length))
        .map_err(|error| format!("not an integer at {at}: {text} ({error})"))
}

fn list(chars: &[char], at: usize) -> Result<(Json, usize), String> {
    if chars.get(at) == Some(&']') {
        return Ok((Json::List(Vec::new()), at + 1));
    }
    let mut items = Vec::new();
    let mut cursor = at;
    loop {
        let (item, end) = value(chars, skip_blanks(chars, cursor))?;
        items.push(item);
        let end = skip_blanks(chars, end);
        match chars.get(end) {
            Some(',') => cursor = end + 1,
            Some(']') => return Ok((Json::List(items), end + 1)),
            other => return Err(format!("expected ',' or ']' at {end}, found {other:?}")),
        }
    }
}

fn object(chars: &[char], at: usize) -> Result<(Json, usize), String> {
    if chars.get(at) == Some(&'}') {
        return Ok((Json::Object(Vec::new()), at + 1));
    }
    let mut entries = Vec::new();
    let mut cursor = at;
    loop {
        let quote = expect(chars, skip_blanks(chars, cursor), '"')?;
        let (key, after_key) = string(chars, quote)?;
        let colon = expect(chars, skip_blanks(chars, after_key), ':')?;
        let (item, end) = value(chars, skip_blanks(chars, colon))?;
        entries.push((key, item));
        let end = skip_blanks(chars, end);
        match chars.get(end) {
            Some(',') => cursor = end + 1,
            Some('}') => return Ok((Json::Object(entries), end + 1)),
            other => return Err(format!("expected ',' or '}}' at {end}, found {other:?}")),
        }
    }
}

/// The text of a string whose opening quote is before `at`, and the position after its closing quote.
fn string(chars: &[char], at: usize) -> Result<(String, usize), String> {
    let mut out = String::new();
    let mut cursor = at;
    loop {
        match chars.get(cursor) {
            None => return Err("a string is not closed".to_owned()),
            Some('"') => return Ok((out, cursor + 1)),
            Some('\\') => {
                let (decoded, used) = escape(chars, cursor + 1)?;
                out.push(decoded);
                cursor += 1 + used;
            }
            Some(other) => {
                out.push(*other);
                cursor += 1;
            }
        }
    }
}

/// The character of the escape after a backslash, and how many characters it used.
fn escape(chars: &[char], at: usize) -> Result<(char, usize), String> {
    match chars.get(at) {
        Some('"') => Ok(('"', 1)),
        Some('\\') => Ok(('\\', 1)),
        Some('/') => Ok(('/', 1)),
        Some('b') => Ok(('\u{8}', 1)),
        Some('f') => Ok(('\u{c}', 1)),
        Some('n') => Ok(('\n', 1)),
        Some('r') => Ok(('\r', 1)),
        Some('t') => Ok(('\t', 1)),
        Some('u') => {
            let hex: String = chars.iter().skip(at + 1).take(4).collect();
            u32::from_str_radix(&hex, 16)
                .ok()
                .and_then(char::from_u32)
                .map(|decoded| (decoded, 5))
                .ok_or_else(|| format!("bad \\u escape {hex:?}"))
        }
        other => Err(format!("bad escape {other:?}")),
    }
}
