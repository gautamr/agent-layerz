//! The types of native values (spec §7.9, §7.2): how a descriptor states which keys of its tool's
//! settings and of an agent's native fields alz accepts, and of what type each value is.
//!
//! A [`Shape`] is the type of a value. A [`Shape::Object`] lists its keys, so a key that it does not
//! list is an error at any depth, and a [`Shape::Map`] takes any key with values of one shape. The
//! tables are static data: a descriptor holds them as `&'static` slices, and the core reads a YAML
//! node against them.

/// The type of a native value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shape {
    /// `true` or `false`.
    Bool,
    /// The value `false` and nothing else.
    False,
    /// Text.
    Text,
    /// A whole number.
    Whole,
    /// A whole number within the span.
    WholeIn(Span),
    /// A finite whole or fractional number: `.nan` and the infinities are not numbers here.
    Number,
    /// A number between 0 and 1.
    Fraction(Ends),
    /// The name of an MCP server in an allowlist entry: letters, digits, hyphens and
    /// underscores, and at least one.
    McpServerName,
    /// One of these texts.
    Choice(&'static [&'static str]),
    /// A list whose items all have the shape.
    List(&'static Self),
    /// A list with at least one item, whose items all have the shape.
    NonEmptyList(&'static Self),
    /// A mapping with any text keys, whose values all have the shape.
    Map(&'static Self),
    /// A mapping with these keys, each optional unless required, and no other.
    Object(&'static [Field]),
    /// A mapping with exactly one key, which is one of these.
    Single(&'static [Field]),
    /// A value of one of these shapes.
    Either(&'static [Self]),
}

/// The whole numbers a [`Shape::WholeIn`] accepts: from `min`, up to `max` when there is one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    min: i128,
    max: Option<i128>,
}

impl Span {
    /// Every whole number from `min` up.
    #[must_use]
    pub const fn at_least(min: i128) -> Self {
        Self { min, max: None }
    }

    /// Every whole number from `min` to `max`, both included.
    #[must_use]
    pub const fn between(min: i128, max: i128) -> Self {
        Self {
            min,
            max: Some(max),
        }
    }

    /// The least whole number of the span.
    #[must_use]
    pub const fn min(self) -> i128 {
        self.min
    }

    /// The greatest whole number of the span, when it has one.
    #[must_use]
    pub const fn max(self) -> Option<i128> {
        self.max
    }

    /// Whether `number` is in the span.
    #[must_use]
    pub const fn holds(self, number: i128) -> bool {
        number >= self.min
            && match self.max {
                Some(max) => number <= max,
                None => true,
            }
    }
}

/// How a [`Shape::Fraction`] treats 0: its upper end is always 1, included.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ends {
    /// Above 0 and at most 1.
    AboveZero,
    /// From 0 to 1, both included.
    FromZero,
}

impl Ends {
    /// Whether `number` is in the range.
    #[must_use]
    pub fn holds(self, number: f64) -> bool {
        let above = match self {
            Self::AboveZero => number > 0.0,
            Self::FromZero => number >= 0.0,
        };
        above && number <= 1.0
    }
}

/// A key of a [`Shape::Object`] and the shape of its value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Field {
    name: &'static str,
    shape: Shape,
    required: bool,
}

impl Field {
    /// The key `name`, holding a value of `shape`.
    #[must_use]
    pub const fn new(name: &'static str, shape: Shape) -> Self {
        Self {
            name,
            shape,
            required: false,
        }
    }

    /// The key `name`, holding a value of `shape`, which the mapping must have.
    #[must_use]
    pub const fn required(name: &'static str, shape: Shape) -> Self {
        Self {
            name,
            shape,
            required: true,
        }
    }

    /// The key.
    #[must_use]
    pub const fn name(&self) -> &'static str {
        self.name
    }

    /// The shape of the value.
    #[must_use]
    pub const fn shape(&self) -> Shape {
        self.shape
    }

    /// Whether a mapping that has this key's object must hold the key.
    #[must_use]
    pub const fn is_required(&self) -> bool {
        self.required
    }
}

/// The typed keys of one place that holds native keys (§7.9 **Typed keys**, §7.2): the table, and
/// the words the message about an unlisted key needs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct TypedKeys {
    /// The keys that the place accepts, with their types.
    pub fields: &'static [Field],
    /// The name of the tool, as `Claude Code`, for the message about an unlisted settings key.
    pub tool: &'static str,
    /// Where the rows are recorded, as `docs/target-facts.md section 14`, which the message about
    /// an unlisted key points to.
    pub recorded_in: &'static str,
}

impl TypedKeys {
    /// The typed keys `fields` of the tool `tool`, recorded in `recorded_in`.
    #[must_use]
    pub const fn new(
        fields: &'static [Field],
        tool: &'static str,
        recorded_in: &'static str,
    ) -> Self {
        Self {
            fields,
            tool,
            recorded_in,
        }
    }
}
