//! The pre-pass over the parser's events (decision-28's open choice of how tags reach alz, and
//! where spec §7.13's four budgets are counted). It reads the events once, before the tree is
//! built, and refuses:
//! - an explicit tag outside the YAML 1.2 core set (`yaml-tag`);
//! - a second document;
//! - an alias to an anchor whose node is still open (cyclic) or that is not defined (unresolved);
//! - a decoded key or value with a control character other than tab and line feed;
//! - more than 100 aliases as written, more than 64 levels of nesting, more than 20000 nodes and
//!   more than 10000 events replayed through aliases.
//!
//! It stops at the first problem, so a hostile file costs no more than the events before it, and
//! it keeps no text: only counts, anchor extents and the position of every node, which the tree
//! uses for its positions.
//!
//! How the budgets count:
//! - **Nodes** are scalars, aliases, mappings and sequences as written.
//! - **Nesting** is the depth of the tree that results, aliases expanded: an alias at depth `d`
//!   of an anchor whose node is `h` levels deep needs `d + h` at most 64. A scalar is 0 levels.
//! - **Replayed events**: using an alias costs the events of its anchor's node. That is one
//!   event for a scalar and two plus its children for a mapping or sequence, and a child that is
//!   an alias is one event plus the events of its own anchor. A bomb that nests aliases
//!   (`b: [*a, *a]`, `c: [*b, *b]`, ...) therefore costs what it expands to.

use std::collections::BTreeMap;
use std::ops::Range;

use serde_saphyr::granit_parser::{ErrorKind, Event, Marker, Parser, ScanError, Span, Tag};

use crate::diagnostic::Position;
use crate::yaml::error::{YamlError, YamlErrorKind};
use crate::yaml::text::{YamlText, is_forbidden_when_decoded};

/// Aliases as written in a document.
pub const MAX_ALIASES: usize = 100;
/// Mappings and sequences inside one another.
pub const MAX_DEPTH: usize = 64;
/// Scalars, aliases, mappings and sequences in a document.
pub const MAX_NODES: usize = 20_000;
/// Events replayed through aliases.
pub const MAX_REPLAYED_EVENTS: usize = 10_000;

/// What the pre-pass learned about a text that passed it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Checked {
    /// The position of every node in the order the tree reads them (a mapping's key, then its
    /// value), aliases expanded: each node an alias repeats has the alias's position.
    positions: Vec<Position>,
}

impl Checked {
    /// The position of every node, in reading order.
    pub fn positions(&self) -> &[Position] {
        &self.positions
    }

    /// Whether the text holds no document, so there is nothing to read.
    pub fn is_empty(&self) -> bool {
        self.positions.is_empty()
    }
}

/// Reads the events of `text` and checks them.
///
/// # Errors
///
/// The first problem, as a [`YamlError`] at its position.
#[expect(
    clippy::disallowed_methods,
    reason = "the yaml::parse pre-pass is the one reader of YAML events (decision-28, TASK-62)"
)]
pub fn check(text: &YamlText) -> Result<Checked, YamlError> {
    let options = serde_saphyr::granit_parser::options! { emit_unresolved_aliases: true };
    Parser::new_from_str_with_options(text.as_str(), options)
        .try_fold(Walk::default(), |walk, item| match item {
            Ok((event, span)) => walk.step(&event, &span),
            Err(failure) => Err(scan_error(&failure)),
        })
        .map(|walk| Checked {
            positions: walk.positions,
        })
}

fn scan_error(failure: &ScanError) -> YamlError {
    // `ErrorKind` is open-ended, so one variant is picked out and the rest are plain syntax.
    let kind = if matches!(failure.kind(), ErrorKind::RecursionLimitExceeded) {
        YamlErrorKind::TooDeep
    } else {
        YamlErrorKind::Syntax
    };
    YamlError::new(kind, Some(position_of(failure.marker())))
}

/// A parser marker as a position: its line counts from 1 already and its column from 0.
pub fn position_of(marker: &Marker) -> Position {
    let line = u32::try_from(marker.line()).unwrap_or(u32::MAX);
    let column = u32::try_from(marker.col().saturating_add(1)).unwrap_or(u32::MAX);
    Position::new(line, column).unwrap_or(Position::FIRST)
}

/// What is known of an anchor: its node is still open, or it is closed with a size.
#[derive(Debug, Clone)]
enum Anchor {
    Open,
    Closed(Extent),
}

/// The size of a closed node.
#[derive(Debug, Clone)]
struct Extent {
    /// Events of the node, as the module documentation counts them.
    events: usize,
    /// Levels of mappings and sequences in it.
    height: usize,
    /// The part of the position list its nodes occupy.
    nodes: Range<usize>,
}

/// A mapping or sequence that is open.
#[derive(Debug)]
struct Frame {
    anchor: Option<usize>,
    events: usize,
    height: usize,
    first_node: usize,
}

/// The state of the walk over the events.
#[derive(Debug, Default)]
struct Walk {
    documents: usize,
    aliases: usize,
    nodes: usize,
    replayed: usize,
    open: Vec<Frame>,
    anchors: BTreeMap<usize, Anchor>,
    positions: Vec<Position>,
}

impl Walk {
    #[expect(
        clippy::wildcard_enum_match_arm,
        reason = "granit-parser's Event is non_exhaustive, so the compiler cannot check a closed match; stream markers, document ends and anchor metadata need no check"
    )]
    fn step(mut self, event: &Event<'_>, span: &Span) -> Result<Self, YamlError> {
        let at = position_of(&span.start);
        let refuse = |kind| YamlError::new(kind, Some(at));
        match event {
            Event::DocumentStart(..) => {
                self.documents = self.documents.saturating_add(1);
                if self.documents > 1 {
                    return Err(refuse(YamlErrorKind::MoreThanOneDocument));
                }
                Ok(self)
            }
            Event::Scalar(value, _style, anchor, tag) => {
                refuse_tag(tag.as_deref()).map_err(refuse)?;
                if value.chars().any(is_forbidden_when_decoded) {
                    return Err(refuse(YamlErrorKind::DecodedControlCharacter));
                }
                let mut walk = self.count_node().map_err(refuse)?;
                let first = walk.positions.len();
                walk.positions.push(at);
                let extent = Extent {
                    events: 1,
                    height: 0,
                    nodes: first..first.saturating_add(1),
                };
                Ok(walk.complete(anchor_of(*anchor), &extent))
            }
            Event::SequenceStart(_, anchor, tag) | Event::MappingStart(_, anchor, tag) => {
                refuse_tag(tag.as_deref()).map_err(refuse)?;
                let mut walk = self.count_node().map_err(refuse)?;
                if walk.open.len() >= MAX_DEPTH {
                    return Err(refuse(YamlErrorKind::TooDeep));
                }
                let anchor = anchor_of(*anchor);
                if let Some(id) = anchor {
                    walk.anchors.insert(id, Anchor::Open);
                }
                let first_node = walk.positions.len();
                walk.positions.push(at);
                walk.open.push(Frame {
                    anchor,
                    events: 1,
                    height: 0,
                    first_node,
                });
                Ok(walk)
            }
            Event::SequenceEnd | Event::MappingEnd => {
                let frame = self
                    .open
                    .pop()
                    .ok_or_else(|| refuse(YamlErrorKind::Syntax))?;
                let extent = Extent {
                    events: frame.events.saturating_add(1),
                    height: frame.height.saturating_add(1),
                    nodes: frame.first_node..self.positions.len(),
                };
                Ok(self.complete(frame.anchor, &extent))
            }
            Event::Alias(id) => self.alias(*id, at),
            Event::UnresolvedAlias(_) => Err(refuse(YamlErrorKind::UnresolvedAlias)),
            _ => Ok(self),
        }
    }

    /// Counts a node as written against the node budget.
    fn count_node(mut self) -> Result<Self, YamlErrorKind> {
        self.nodes = self.nodes.saturating_add(1);
        if self.nodes > MAX_NODES {
            Err(YamlErrorKind::TooManyNodes)
        } else {
            Ok(self)
        }
    }

    /// An alias: a node as written, an alias against the alias budget, and its anchor's events
    /// against the replay budget.
    fn alias(mut self, id: usize, at: Position) -> Result<Self, YamlError> {
        let refuse = |kind| YamlError::new(kind, Some(at));
        self.aliases = self.aliases.saturating_add(1);
        if self.aliases > MAX_ALIASES {
            return Err(refuse(YamlErrorKind::TooManyAliases));
        }
        let mut walk = self.count_node().map_err(refuse)?;
        let extent = match walk.anchors.get(&id) {
            None => return Err(refuse(YamlErrorKind::UnresolvedAlias)),
            Some(Anchor::Open) => return Err(refuse(YamlErrorKind::CyclicAlias)),
            Some(Anchor::Closed(extent)) => extent.clone(),
        };
        if walk.open.len().saturating_add(extent.height) > MAX_DEPTH {
            return Err(refuse(YamlErrorKind::TooDeep));
        }
        walk.replayed = walk.replayed.saturating_add(extent.events);
        if walk.replayed > MAX_REPLAYED_EVENTS {
            return Err(refuse(YamlErrorKind::TooManyReplayedEvents));
        }
        // The replay is within budget, so the extent is small: its nodes are fewer than its events.
        let first = walk.positions.len();
        walk.positions.extend(extent.nodes.clone().map(|_| at));
        let counted = Extent {
            events: 1_usize.saturating_add(extent.events),
            height: extent.height,
            nodes: first..walk.positions.len(),
        };
        Ok(walk.complete(None, &counted))
    }

    /// A node is closed: its anchor, if any, now has a size, and its parent has grown by it.
    fn complete(mut self, anchor: Option<usize>, extent: &Extent) -> Self {
        if let Some(id) = anchor {
            self.anchors.insert(id, Anchor::Closed(extent.clone()));
        }
        if let Some(parent) = self.open.last_mut() {
            parent.events = parent.events.saturating_add(extent.events);
            parent.height = parent.height.max(extent.height);
        }
        self
    }
}

/// The parser numbers anchors from 1; 0 means the node has none.
const fn anchor_of(id: usize) -> Option<usize> {
    if id == 0 { None } else { Some(id) }
}

/// An explicit tag is allowed only when it is one of `!!str !!int !!float !!bool !!null !!map
/// !!seq`: the `tag:yaml.org,2002:` namespace and one of those suffixes. A local tag such as `!foo`
/// and every other global tag are not.
fn refuse_tag(tag: Option<&Tag>) -> Result<(), YamlErrorKind> {
    let core = tag.is_none_or(|tag| {
        tag.handle() == "tag:yaml.org,2002:"
            && matches!(
                tag.suffix(),
                "str" | "int" | "float" | "bool" | "null" | "map" | "seq"
            )
    });
    if core {
        Ok(())
    } else {
        Err(YamlErrorKind::Tag)
    }
}
