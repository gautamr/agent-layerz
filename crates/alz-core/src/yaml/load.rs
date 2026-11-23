//! The one place serde-saphyr is called (decision-28, TASK-62): its parse runs with the options
//! decision-28 names and reads the document into alz's tree through `deserialize_any` and a
//! hand-written visitor. serde-saphyr's own budgets stay on as defence in depth, set to what the
//! pre-pass of [`crate::yaml::events`] already enforces; the pre-pass decides what spec §7.13
//! counts. serde-saphyr's error text is never read, only which error it is.

use std::cell::{Cell, RefCell};
use std::fmt;

use serde::de::{self, DeserializeSeed, MapAccess, SeqAccess, Visitor};
use serde_saphyr::{
    Budget, DuplicateKeyPolicy, Error, MergeKeyPolicy, NonFiniteFloatPolicy, Options,
    budget::BudgetBreach,
};

use crate::diagnostic::Position;
use crate::untrusted_text::QuotedKey;
use crate::yaml::error::{YamlError, YamlErrorKind};
use crate::yaml::events::{self, MAX_ALIASES, MAX_DEPTH, MAX_NODES, MAX_REPLAYED_EVENTS};
use crate::yaml::text::YamlText;
use crate::yaml::tree::{Mapping, Node, Value};

/// Parses `text` into alz's tree: the event pre-pass, then serde-saphyr.
/// Every YAML file alz reads goes through here.
///
/// A text without a document (empty, or only comments) is a null node at line 1, column 1.
///
/// # Errors
///
/// The first problem found, as a [`YamlError`]; its text never holds input except one quoted
/// duplicate key.
pub fn parse(text: &YamlText) -> Result<Node, YamlError> {
    let checked = events::check(text)?;
    if checked.is_empty() {
        return Ok(Node::new(Value::Null, Position::FIRST));
    }
    let context = Context {
        positions: checked.positions(),
        next: Cell::new(0),
        failure: RefCell::new(None),
    };
    read(text, &context)
        .map_err(|failure| context.failure.take().unwrap_or_else(|| classify(&failure)))
}

/// The options of decision-28.
fn options() -> Options {
    let mut options = Options::default();
    options.strict_booleans = true;
    options.merge_keys = MergeKeyPolicy::AsOrdinary;
    options.duplicate_keys = DuplicateKeyPolicy::Error;
    options.legacy_octal_numbers = false;
    options.with_snippet = false;
    options.non_finite_float_policy = NonFiniteFloatPolicy::PassThrough;
    options.underscored_numbers_as_strings = true;
    options.alias_limits.max_total_replayed_events = MAX_REPLAYED_EVENTS;
    let mut budget = Budget::default();
    budget.max_aliases = MAX_ALIASES;
    budget.max_depth = MAX_DEPTH;
    // serde-saphyr charges replayed events to its node budget, so the budget it enforces is the
    // nodes as written plus the events replayed through aliases.
    budget.max_nodes = MAX_NODES.saturating_add(MAX_REPLAYED_EVENTS);
    // Its alias-to-anchor ratio rule is not in spec §7.13: the spec counts aliases alone.
    budget.enforce_alias_anchor_ratio = false;
    options.budget = Some(budget);
    options
}

#[expect(
    clippy::disallowed_methods,
    reason = "the one entry point to serde-saphyr (decision-28, TASK-62): every YAML file alz reads goes through yaml::parse"
)]
fn read(text: &YamlText, context: &Context<'_>) -> Result<Node, Error> {
    serde_saphyr::with_deserializer_from_str_with_options(text.as_str(), options(), |reader| {
        NodeSeed(context).deserialize(reader)
    })
}

/// Which error serde-saphyr gave, as the kind of problem and where. Its text is not read.
fn classify(failure: &Error) -> YamlError {
    let at = failure.location().and_then(|place| {
        Position::new(
            u32::try_from(place.line()).unwrap_or(u32::MAX),
            u32::try_from(place.column()).unwrap_or(u32::MAX),
        )
    });
    YamlError::new(kind_of(failure), at)
}

/// The error and the budget breach are open-ended enums, so the ones alz names are picked out and
/// every other error is plain syntax.
fn kind_of(failure: &Error) -> YamlErrorKind {
    if let Error::DuplicateMappingKey { key, .. } = failure {
        YamlErrorKind::DuplicateKey(key.as_deref().map(QuotedKey::from_untrusted))
    } else if matches!(failure, Error::MultipleDocuments { .. }) {
        YamlErrorKind::MoreThanOneDocument
    } else if matches!(failure, Error::UnknownAnchor { .. }) {
        YamlErrorKind::UnresolvedAlias
    } else if matches!(
        failure,
        Error::AliasReplayLimitExceeded { .. }
            | Error::AliasReplayCounterOverflow { .. }
            | Error::AliasExpansionLimitExceeded { .. }
    ) {
        YamlErrorKind::TooManyReplayedEvents
    } else if matches!(failure, Error::AliasReplayStackDepthExceeded { .. }) {
        YamlErrorKind::TooDeep
    } else if let Error::Budget { breach, .. } = failure {
        budget_kind(breach)
    } else {
        YamlErrorKind::Syntax
    }
}

fn budget_kind(breach: &BudgetBreach) -> YamlErrorKind {
    if matches!(breach, BudgetBreach::Aliases { .. }) {
        YamlErrorKind::TooManyAliases
    } else if matches!(breach, BudgetBreach::Depth { .. }) {
        YamlErrorKind::TooDeep
    } else if matches!(breach, BudgetBreach::Nodes { .. }) {
        YamlErrorKind::TooManyNodes
    } else {
        YamlErrorKind::Syntax
    }
}

/// What the visitors share: the position of every node in reading order, the next one to give,
/// and the first problem the tree itself found, which serde-saphyr's error cannot carry.
struct Context<'a> {
    positions: &'a [Position],
    next: Cell<usize>,
    failure: RefCell<Option<YamlError>>,
}

impl Context<'_> {
    /// The position of the node being read now, and the cursor moved to the next.
    fn take_position(&self) -> Position {
        let index = self.next.get();
        self.next.set(index.saturating_add(1));
        self.positions
            .get(index)
            .or_else(|| self.positions.last())
            .copied()
            .unwrap_or(Position::FIRST)
    }

    /// Keeps the first problem and gives the error that stops serde-saphyr.
    fn fail<E: de::Error>(&self, failure: YamlError) -> E {
        self.failure.borrow_mut().get_or_insert(failure);
        E::custom("stopped")
    }
}

/// Reads one node, taking its position first, so a node's position comes before its children's.
struct NodeSeed<'a, 'c>(&'a Context<'c>);

impl<'de> DeserializeSeed<'de> for NodeSeed<'_, '_> {
    type Value = Node;

    fn deserialize<D: de::Deserializer<'de>>(self, reader: D) -> Result<Node, D::Error> {
        let at = self.0.take_position();
        reader.deserialize_any(NodeVisitor {
            context: self.0,
            at,
        })
    }
}

struct NodeVisitor<'a, 'c> {
    context: &'a Context<'c>,
    at: Position,
}

impl NodeVisitor<'_, '_> {
    fn node(&self, value: Value) -> Node {
        Node::new(value, self.at)
    }
}

impl<'de> Visitor<'de> for NodeVisitor<'_, '_> {
    type Value = Node;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a YAML node")
    }

    fn visit_unit<E: de::Error>(self) -> Result<Node, E> {
        Ok(self.node(Value::Null))
    }

    fn visit_none<E: de::Error>(self) -> Result<Node, E> {
        Ok(self.node(Value::Null))
    }

    fn visit_bool<E: de::Error>(self, flag: bool) -> Result<Node, E> {
        Ok(self.node(Value::Bool(flag)))
    }

    fn visit_i64<E: de::Error>(self, number: i64) -> Result<Node, E> {
        Ok(self.node(Value::Int(i128::from(number))))
    }

    fn visit_u64<E: de::Error>(self, number: u64) -> Result<Node, E> {
        Ok(self.node(Value::Int(i128::from(number))))
    }

    fn visit_i128<E: de::Error>(self, number: i128) -> Result<Node, E> {
        Ok(self.node(Value::Int(number)))
    }

    fn visit_u128<E: de::Error>(self, number: u128) -> Result<Node, E> {
        match i128::try_from(number) {
            Ok(number) => Ok(self.node(Value::Int(number))),
            Err(_) => Err(self
                .context
                .fail(YamlError::new(YamlErrorKind::Syntax, Some(self.at)))),
        }
    }

    fn visit_f64<E: de::Error>(self, number: f64) -> Result<Node, E> {
        Ok(self.node(Value::Float(number)))
    }

    fn visit_str<E: de::Error>(self, text: &str) -> Result<Node, E> {
        Ok(self.node(Value::Text(text.to_owned())))
    }

    fn visit_string<E: de::Error>(self, text: String) -> Result<Node, E> {
        Ok(self.node(Value::Text(text)))
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut items: A) -> Result<Node, A::Error> {
        let mut nodes = Vec::new();
        while let Some(node) = items.next_element_seed(NodeSeed(self.context))? {
            nodes.push(node);
        }
        Ok(self.node(Value::Seq(nodes)))
    }

    fn visit_map<A: MapAccess<'de>>(self, mut entries: A) -> Result<Node, A::Error> {
        let mut pairs = Vec::new();
        while let Some(key) = entries.next_key_seed(NodeSeed(self.context))? {
            let value = entries.next_value_seed(NodeSeed(self.context))?;
            pairs.push((key, value));
        }
        Mapping::from_entries(pairs)
            .map(|mapping| self.node(Value::Map(mapping)))
            .map_err(|failure| self.context.fail(failure))
    }
}
