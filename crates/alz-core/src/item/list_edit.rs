//! How an overlay writes the edit of a list (spec §7.2): a YAML list replaces the inherited
//! list, and a string of segments `+[a, b]`, `-[c]` and `[d]` appends, removes and replaces, in
//! the order written. A field written as `~` or left out keeps the inherited list.

use crate::merge::{ListEdit, OrderedSet};
use crate::schema::{Checked, Refusal, SchemaError, all, refine_node};
use crate::yaml::Value;
use crate::yaml::fields::{KeyPath, Section};

use super::AgentRefusal;

/// What one segment does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EditSign {
    /// `+[..]`: append these items.
    Add,
    /// `-[..]`: drop these items.
    Remove,
    /// `[..]`, or a YAML list: the list is exactly these items.
    Replace,
}

/// One segment of an edit as the layer wrote it: its sign and its items, trimmed and unquoted.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct EditSegment {
    sign: EditSign,
    items: Vec<String>,
}

impl EditSegment {
    /// The segment with `sign` and `items`.
    #[must_use]
    pub const fn new(sign: EditSign, items: Vec<String>) -> Self {
        Self { sign, items }
    }

    /// What the segment does.
    #[must_use]
    pub const fn sign(&self) -> EditSign {
        self.sign
    }

    /// The items, in the order written.
    #[must_use]
    pub fn items(&self) -> &[String] {
        &self.items
    }
}

/// An edit of a list as the layer wrote it: its segments in the order written, which the merge
/// folds into one [`ListEdit`] and `alz explain` shows as written (spec §10.3). An edit written
/// as a string without any segment has none.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct WrittenEdit(Vec<EditSegment>);

impl WrittenEdit {
    /// The edit made of `segments`.
    #[must_use]
    pub const fn new(segments: Vec<EditSegment>) -> Self {
        Self(segments)
    }

    /// The segments in the order written.
    #[must_use]
    pub fn segments(&self) -> &[EditSegment] {
        &self.0
    }
}

/// The edit that `field` of `section` says as it was written, `None` when it is absent or has no
/// edit form.
pub(super) fn written_edit(section: &Section<'_>, field: &'static str) -> Option<WrittenEdit> {
    section.node(field).and_then(|node| match node.value() {
        Value::Text(text) => segments(text).map(|parts| {
            WrittenEdit(
                parts
                    .into_iter()
                    .map(|(sign, items)| {
                        EditSegment::new(sign, items.into_iter().map(str::to_owned).collect())
                    })
                    .collect(),
            )
        }),
        Value::Null
        | Value::Bool(_)
        | Value::Int(_)
        | Value::Float(_)
        | Value::Seq(_)
        | Value::Map(_) => section.items(field).ok().flatten().map(|items| {
            WrittenEdit(vec![EditSegment::new(
                EditSign::Replace,
                items
                    .into_iter()
                    .filter_map(|(_, item)| match item.value() {
                        Value::Text(text) => Some(text.clone()),
                        Value::Null
                        | Value::Bool(_)
                        | Value::Int(_)
                        | Value::Float(_)
                        | Value::Seq(_)
                        | Value::Map(_) => None,
                    })
                    .collect(),
            )])
        }),
    })
}

/// One segment of an edit: its sign and its items as written, trimmed and unquoted, empty ones
/// left out.
pub(super) type Segment<'a> = (EditSign, Vec<&'a str>);

/// The segments of `text`, `None` when some part of it is not a segment. White space between
/// segments is free, but a text of white space alone is not an edit: only the empty text says no
/// segment (the reference refuses the first and accepts the second).
pub(super) fn segments(text: &str) -> Option<Vec<Segment<'_>>> {
    if !text.is_empty() && text.trim().is_empty() {
        return None;
    }
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
        .map(|rest| (EditSign::Add, rest))
        .or_else(|| text.strip_prefix('-').map(|rest| (EditSign::Remove, rest)))
        .unwrap_or((EditSign::Replace, text));
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

/// The edit of a list that the field `field` says, `None` when it is absent. A YAML list is the
/// edit `from_list` makes of its items: a replacement for an overlay, an addition for the lists
/// of `permissions.yaml`. A string is a sequence of segments.
///
/// # Errors
///
/// A list or a string that is not an edit, and each item that is not a valid `T`.
pub(super) fn refine_edit<T>(
    section: &Section<'_>,
    field: &'static str,
    from_list: fn(OrderedSet<T>) -> ListEdit<T>,
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
                    .map(|items| from_list(items.into_iter().collect()))
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

fn edit_of<T: Eq + Clone>(sign: EditSign, items: OrderedSet<T>) -> ListEdit<T> {
    match sign {
        EditSign::Replace => ListEdit::Replace(items),
        EditSign::Add => ListEdit::Adjust {
            remove: OrderedSet::default(),
            add: items,
        },
        EditSign::Remove => ListEdit::Adjust {
            remove: items,
            add: OrderedSet::default(),
        },
    }
}
