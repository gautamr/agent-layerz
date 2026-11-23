//! How an overlay writes the edit of a list (spec §7.2): a YAML list replaces the inherited
//! list, and a string of segments `+[a, b]`, `-[c]` and `[d]` appends, removes and replaces, in
//! the order written. A field written as `~` or left out keeps the inherited list.

use crate::merge::{ListEdit, OrderedSet};
use crate::schema::{Checked, Refusal, SchemaError, all, refine_node};
use crate::yaml::Value;
use crate::yaml::fields::{KeyPath, Section};

use super::vocabulary::AgentRefusal;

/// What one segment does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Sign {
    Add,
    Remove,
    Replace,
}

/// One segment of an edit: its sign and its items as written, trimmed and unquoted, empty ones
/// left out.
pub(super) type Segment<'a> = (Sign, Vec<&'a str>);

/// The segments of `text`, `None` when some part of it is not a segment. White space between
/// segments is free.
pub(super) fn segments(text: &str) -> Option<Vec<Segment<'_>>> {
    // A local loop: it consumes the text segment by segment, and nothing escapes.
    let mut rest = text.trim_start();
    let mut found = Vec::new();
    while !rest.is_empty() {
        let (segment, after) = segment(rest)?;
        found.push(segment);
        rest = after.trim_start();
    }
    Some(found)
}

/// The segment at the start of `text`, and the text after it.
fn segment(text: &str) -> Option<(Segment<'_>, &str)> {
    let (sign, after_sign) = text
        .strip_prefix('+')
        .map(|rest| (Sign::Add, rest))
        .or_else(|| text.strip_prefix('-').map(|rest| (Sign::Remove, rest)))
        .unwrap_or((Sign::Replace, text));
    let (inner, after) = after_sign.strip_prefix('[')?.split_once(']')?;
    let items = inner
        .split(',')
        .map(|item| unquoted(item.trim()))
        .filter(|item| !item.is_empty())
        .collect();
    Some(((sign, items), after))
}

/// `item` without one pair of matching quotes around it.
fn unquoted(item: &str) -> &str {
    ['"', '\'']
        .into_iter()
        .find_map(|quote| {
            item.strip_prefix(quote)
                .and_then(|rest| rest.strip_suffix(quote))
        })
        .unwrap_or(item)
}

/// The edit of a list that the field `field` of an overlay says, `None` when it is absent.
///
/// # Errors
///
/// A list or a string that is not an edit, and each item that is not a valid `T`.
pub(super) fn refine_edit<T>(
    section: &Section<'_>,
    field: &'static str,
) -> Checked<Option<ListEdit<T>>>
where
    T: Eq + Clone + for<'a> TryFrom<&'a str>,
    for<'a> <T as TryFrom<&'a str>>::Error: Into<Refusal>,
{
    let path = section.field_path(field);
    match section.node(field) {
        None => Ok(None),
        Some(node) => match node.value() {
            Value::Text(text) => text_edit::<T>(&path, node.at(), text).map(Some),
            Value::Null
            | Value::Bool(_)
            | Value::Int(_)
            | Value::Float(_)
            | Value::Seq(_)
            | Value::Map(_) => section
                .items(field)?
                .map(|items| {
                    all(items
                        .into_iter()
                        .map(|(item_path, item)| refine_node::<T>(item_path, item)))
                    .map(|items| ListEdit::Replace(items.into_iter().collect()))
                })
                .transpose(),
        },
    }
}

fn text_edit<T>(path: &KeyPath, at: crate::diagnostic::Position, text: &str) -> Checked<ListEdit<T>>
where
    T: Eq + Clone + for<'a> TryFrom<&'a str>,
    for<'a> <T as TryFrom<&'a str>>::Error: Into<Refusal>,
{
    let refused = |refusal: Refusal| SchemaError::refused(path.clone(), refusal, at).into();
    let parts = segments(text).ok_or_else(|| refused(AgentRefusal::ListEdit.into()))?;
    all(parts.into_iter().map(|(sign, items)| {
        all(items
            .into_iter()
            .map(|item| T::try_from(item).map_err(|error| refused(error.into()))))
        .map(|items| edit_of(sign, items.into_iter().collect()))
    }))
    .map(|edits| edits.into_iter().fold(ListEdit::default(), ListEdit::merge))
}

fn edit_of<T: Eq + Clone>(sign: Sign, items: OrderedSet<T>) -> ListEdit<T> {
    match sign {
        Sign::Replace => ListEdit::Replace(items),
        Sign::Add => ListEdit::Adjust {
            remove: OrderedSet::default(),
            add: items,
        },
        Sign::Remove => ListEdit::Adjust {
            remove: items,
            add: OrderedSet::default(),
        },
    }
}
