//! Diagnostics as values (decision-26, spec §10.1): a level, a code, a place and a message, with
//! a `Display` that gives the one line `<level> [<code>] <where>: <message>`. Nothing here prints.
//!
//! A diagnostic takes untrusted text only through the text classes of [`crate::Stripped`],
//! [`crate::QuotedKey`] and [`crate::Escaped`]: a [`Message`] is built from literals (`&'static
//! str`), those three types, whole numbers and [`Printable`] domain values, and a [`Where`] holds
//! stripped text. So a message that holds raw input does not compile, and the rendered line holds
//! no control or format character and no line feed.

use std::fmt;
use std::num::NonZeroU32;

use crate::untrusted_text::{Escaped, QuotedKey, Stripped};

/// How serious a diagnostic is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Level {
    /// A problem that fails the command (exit code 1).
    Error,
    /// A problem that is reported and does not fail the command.
    Warning,
}

impl Level {
    /// The word the diagnostic line starts with.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Error => "error",
            Self::Warning => "warning",
        }
    }
}

impl fmt::Display for Level {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// The short kebab-case name of a kind of problem. The spec gives each code and the code is part
/// of the contract (§10.1). A later phase adds a variant for each code it emits, and the
/// exhaustive match in [`Code::as_str`] makes that a compile error wherever it must be handled.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Code {
    /// A YAML file that cannot be read or breaks a limit (§7.13 "YAML limits").
    Yaml,
    /// A YAML tag outside the 1.2 core set (§7.13 "YAML limits").
    YamlTag,
    /// A layer that is missing, malformed or whose `layer.yaml` breaks the schema (§5.1).
    Layer,
    /// A repository's stack manifest that breaks the schema (§5.2).
    Manifest,
    /// A repository without a `layer.yaml` (§5.2).
    NoManifest,
    /// An invalid `layer.lock` (§5.3).
    Lock,
    /// A layer source that cannot be used, such as a `git:` URL that holds credentials (§5.2).
    Source,
    /// A layer cache that cannot be used or whose entry no longer matches (§5.3, §6).
    Cache,
    /// An index entry of the layer cache that cannot be written, a warning (§6).
    CacheIndex,
    /// A `.env` file in a layer (§5.3 "`.env` files").
    EnvFile,
    /// A problem in the owner's `accounts.yaml`, which alz ignores in part or whole (§7.13).
    AccountConfig,
    /// A problem in the allow state `allowed.yaml`, which alz ignores in part or whole (§7.13).
    AccountAllowed,
    /// A git that alz refuses: older than the minimum version, or not started (§6 "Git
    /// subprocesses").
    Git,
    /// Two layers of the stack with one name (§6 step 2).
    DuplicateLayer,
    /// A layer that is more public than one before it in the stack (§3.1, §6 step 2).
    Visibility,
    /// A Markdown file whose frontmatter block is not closed or is not a mapping (§4.1, §7.13).
    Frontmatter,
    /// An instruction file's unknown field or `mode` (§7.1).
    Instructions,
    /// An agent's schema: name, description, a tool or skill entry, a native key with a neutral
    /// field (§7.2).
    Agent,
    /// A second base definition of an agent (§7.2).
    AgentDuplicate,
    /// An overlay's `extends` or `mode`, or an edit that cannot apply (§7.2).
    AgentOverlay,
    /// An overlay that replaces the whole body of an agent, a warning (§7.2).
    AgentReplace,
    /// An overlay that widens a tool, or `allow_widen` in a public layer (§7.2).
    ToolWiden,
    /// A skill directory without `SKILL.md`, a name that is not kebab-case or not the directory's,
    /// or a missing description (§7.3).
    Skill,
    /// A command's schema (§7.4).
    Command,
    /// A skill and a command with one name in one layer (§7.4).
    SkillCommandClash,
    /// A `provides_slots` entry without its skill, or a `requires_slots` entry that is not a skill
    /// name (§7.3).
    Slot,
    /// A slot that no layer fills: an error with `strict_slots: true`, else a warning (§7.3).
    SlotMissing,
    /// An agent that preloads a command (§7.3, §7.4).
    SlotCommand,
    /// A rule's schema (§7.10).
    Rule,
    /// A hook's schema, an id that a more general layer already uses, or a disable entry with
    /// other fields (§7.5).
    Hooks,
    /// A locked hook that a more specific layer disables (§7.5).
    HookLocked,
    /// The schema of `permissions.yaml` (§7.6).
    Permissions,
    /// A deny rule of a more general layer that a more specific layer removes, or a `default_mode`
    /// looser than a floor (§7.6).
    PermissionFloor,
    /// An allow rule that the merged deny list holds too, a warning: deny wins (§7.6).
    PermissionShadowed,
    /// The schema of `settings.yaml`: a top-level key that is not a target, a target whose value is
    /// not a mapping, or a key of a first-class item (§7.9).
    Settings,
    /// The schema of `mcp.yaml`: a key that is not a server id, a definition that breaks its
    /// schema, or an `mcp.enable` entry that no layer defines (§7.7).
    Mcp,
    /// The schema of `models.yaml`: an unknown field, or an alias key that is not a target (§7.8).
    Models,
    /// A value that looks like a token (§7.7, §9.2).
    LeakSecret,
    /// A protected or malformed variable name, or a list or map of variables that cannot be
    /// checked (§7.9).
    ProtectedVariable,
    /// A passthrough file, a warning for each use, or a file below a folder of `targets/` that is
    /// not a known target, an error (§7.11, §4.1).
    Passthrough,
    /// A reference to a more general layer that is not there: an overlay or `extend` rule without
    /// a base, and the like (§10.1). `alz lint` checks one layer alone and leaves it out.
    Unresolved,
}

impl Code {
    /// The code as the diagnostic line prints it.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Yaml => "yaml",
            Self::YamlTag => "yaml-tag",
            Self::Layer => "layer",
            Self::Manifest => "manifest",
            Self::NoManifest => "no-manifest",
            Self::Lock => "lock",
            Self::Source => "source",
            Self::Cache => "cache",
            Self::CacheIndex => "cache-index",
            Self::EnvFile => "env-file",
            Self::AccountConfig => "account-config",
            Self::AccountAllowed => "account-allowed",
            Self::Git => "git",
            Self::DuplicateLayer => "duplicate-layer",
            Self::Visibility => "visibility",
            Self::Frontmatter => "frontmatter",
            Self::Instructions => "instructions",
            Self::Agent => "agent",
            Self::AgentDuplicate => "agent-duplicate",
            Self::AgentOverlay => "agent-overlay",
            Self::AgentReplace => "agent-replace",
            Self::ToolWiden => "tool-widen",
            Self::Skill => "skill",
            Self::Command => "command",
            Self::SkillCommandClash => "skill-command-clash",
            Self::Slot => "slot",
            Self::SlotMissing => "slot-missing",
            Self::SlotCommand => "slot-command",
            Self::Rule => "rule",
            Self::Hooks => "hooks",
            Self::HookLocked => "hook-locked",
            Self::Permissions => "permissions",
            Self::PermissionFloor => "permission-floor",
            Self::PermissionShadowed => "permission-shadowed",
            Self::Settings => "settings",
            Self::Mcp => "mcp",
            Self::Models => "models",
            Self::LeakSecret => "leak-secret",
            Self::ProtectedVariable => "protected-variable",
            Self::Passthrough => "passthrough",
            Self::Unresolved => "unresolved",
        }
    }
}

impl fmt::Display for Code {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// A line and a column in a file, both counted from 1, the column in characters (§10.1). The
/// type holds no zero, so a position the spec cannot write is not a value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Position {
    line: NonZeroU32,
    column: NonZeroU32,
}

impl Position {
    /// Line 1, column 1.
    pub const FIRST: Self = Self {
        line: NonZeroU32::MIN,
        column: NonZeroU32::MIN,
    };

    /// The position, or `None` when `line` or `column` is 0.
    #[must_use]
    pub fn new(line: u32, column: u32) -> Option<Self> {
        Some(Self {
            line: NonZeroU32::new(line)?,
            column: NonZeroU32::new(column)?,
        })
    }
}

impl fmt::Display for Position {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}:{}", self.line, self.column)
    }
}

/// The file or path a diagnostic is about. Each name is stripped text: a layer name and a file
/// name come from the input until the validated types of R2.5 and R2.6 narrow them.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Place {
    /// A file of a layer, printed `<layer>:<path>`. `layer` is the layer's name, or, while the
    /// name is not known, the layer directory or stack entry as it was given.
    LayerFile {
        /// The layer's name, or the directory or entry as it was given.
        layer: Stripped,
        /// The file's path inside the layer.
        path: Stripped,
    },
    /// A file of the repository, such as `layer.yaml` or `layer.lock`, as a path from its root.
    RepositoryFile(Stripped),
    /// A path the person typed, as it was given.
    Given(Stripped),
}

impl fmt::Display for Place {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::LayerFile { layer, path } => write!(formatter, "{layer}:{path}"),
            Self::RepositoryFile(path) | Self::Given(path) => path.fmt(formatter),
        }
    }
}

/// Where a problem is: a [`Place`] with, for a YAML or JSON error, a [`Position`] in it.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Where {
    place: Place,
    position: Option<Position>,
}

impl Where {
    /// A file of a layer, `<layer>:<path>`.
    #[must_use]
    pub fn layer_file(layer: Stripped, path: Stripped) -> Self {
        Self::of(Place::LayerFile { layer, path })
    }

    /// A file of the repository, by its path from the repository root.
    #[must_use]
    pub fn repository_file(path: Stripped) -> Self {
        Self::of(Place::RepositoryFile(path))
    }

    /// A path as the person typed it.
    #[must_use]
    pub fn given(path: Stripped) -> Self {
        Self::of(Place::Given(path))
    }

    /// The same place, at a line and a column.
    #[must_use]
    pub fn at(self, position: Position) -> Self {
        Self {
            position: Some(position),
            ..self
        }
    }

    const fn of(place: Place) -> Self {
        Self {
            place,
            position: None,
        }
    }
}

impl fmt::Display for Where {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.place.fmt(formatter)?;
        self.position
            .map_or(Ok(()), |position| write!(formatter, ":{position}"))
    }
}

pub(crate) mod sealed {
    /// Only this crate decides which types are printable.
    pub trait Sealed {}
}

/// A validated domain value whose alphabet is printable ASCII, such as a target, account or layer
/// name or a git ref, which can go into a message as it is. Sealed: only this crate implements
/// it, for types whose constructor refuses every other character.
pub trait Printable: sealed::Sealed {
    /// The value's text, which holds printable ASCII only.
    fn printable(&self) -> &str;
}

/// One piece of a [`Message`]. A value of this type comes from a literal, from one of the text
/// classes, from a whole number or from a [`Printable`] value, and from nothing else: there is no
/// conversion from `String`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MessagePart(Piece);

#[derive(Debug, Clone, PartialEq, Eq)]
enum Piece {
    Literal(Stripped),
    Stripped(Stripped),
    Key(QuotedKey),
    Escaped(Escaped),
    Number(u64),
}

impl MessagePart {
    /// A validated domain value, printed as it is.
    #[must_use]
    pub fn printable(value: &impl Printable) -> Self {
        // The value's alphabet is printable ASCII; stripping it again costs nothing and keeps the
        // guarantee unconditional.
        Self(Piece::Stripped(Stripped::from_untrusted(value.printable())))
    }
}

impl From<&'static str> for MessagePart {
    /// A literal of the program. Control and format characters in it are removed too, so a
    /// literal cannot break the one-line rule either.
    fn from(literal: &'static str) -> Self {
        Self(Piece::Literal(Stripped::from_untrusted(literal)))
    }
}

impl From<Stripped> for MessagePart {
    fn from(text: Stripped) -> Self {
        Self(Piece::Stripped(text))
    }
}

impl From<QuotedKey> for MessagePart {
    fn from(key: QuotedKey) -> Self {
        Self(Piece::Key(key))
    }
}

impl From<Escaped> for MessagePart {
    fn from(text: Escaped) -> Self {
        Self(Piece::Escaped(text))
    }
}

impl From<u64> for MessagePart {
    fn from(number: u64) -> Self {
        Self(Piece::Number(number))
    }
}

impl From<u32> for MessagePart {
    fn from(number: u32) -> Self {
        Self(Piece::Number(u64::from(number)))
    }
}

impl From<usize> for MessagePart {
    /// A count or an index; a value past `u64::MAX`, which no platform has, saturates.
    fn from(number: usize) -> Self {
        Self(Piece::Number(u64::try_from(number).unwrap_or(u64::MAX)))
    }
}

impl fmt::Display for MessagePart {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.0 {
            Piece::Literal(text) | Piece::Stripped(text) => text.fmt(formatter),
            Piece::Key(key) => key.fmt(formatter),
            Piece::Escaped(text) => text.fmt(formatter),
            Piece::Number(number) => number.fmt(formatter),
        }
    }
}

/// What is wrong, on one line. It is built only from [`MessagePart`]s, so it takes untrusted text
/// only as [`Stripped`], [`QuotedKey`] or [`Escaped`], and a `String` is refused at compile time:
///
/// ```
/// use alz_core::{Message, QuotedKey};
///
/// let message = Message::new("unknown field ")
///     .then(QuotedKey::from_untrusted("colour"))
///     .then(" at item ")
///     .then(3_usize);
/// assert_eq!(message.to_string(), "unknown field 'colour' at item 3");
/// ```
///
/// ```compile_fail
/// use alz_core::Message;
///
/// let from_the_input: String = std::env::args().collect();
/// let _ = Message::new("unknown field ").then(from_the_input);
/// ```
///
/// ```compile_fail
/// use alz_core::Message;
///
/// let from_the_input = String::from("unknown field");
/// let _ = Message::new(from_the_input);
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Message(Vec<MessagePart>);

impl Message {
    /// A message that starts with `first`, usually a literal.
    #[must_use]
    pub fn new(first: impl Into<MessagePart>) -> Self {
        Self(vec![first.into()])
    }

    /// The message with `part` added at its end.
    #[must_use]
    pub fn then(mut self, part: impl Into<MessagePart>) -> Self {
        self.0.push(part.into());
        self
    }
}

impl From<&'static str> for Message {
    fn from(literal: &'static str) -> Self {
        Self::new(literal)
    }
}

impl fmt::Display for Message {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.iter().try_for_each(|part| part.fmt(formatter))
    }
}

/// One problem: `<level> [<code>] <where>: <message>`, without `<where>: ` when nothing can be
/// located.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    level: Level,
    code: Code,
    at: Option<Where>,
    message: Message,
}

impl Diagnostic {
    /// An error: the command fails.
    #[must_use]
    pub const fn error(code: Code, at: Option<Where>, message: Message) -> Self {
        Self {
            level: Level::Error,
            code,
            at,
            message,
        }
    }

    /// A warning: it is reported and the command goes on.
    #[must_use]
    pub const fn warning(code: Code, at: Option<Where>, message: Message) -> Self {
        Self {
            level: Level::Warning,
            code,
            at,
            message,
        }
    }

    /// The level of the diagnostic.
    #[must_use]
    pub const fn level(&self) -> Level {
        self.level
    }

    /// The code of the diagnostic.
    #[must_use]
    pub const fn code(&self) -> Code {
        self.code
    }
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{} [{}] ", self.level, self.code)?;
        if let Some(at) = &self.at {
            write!(formatter, "{at}: ")?;
        }
        self.message.fmt(formatter)
    }
}

/// Every problem one run found, in the order they were found, so one run reports them all.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Diagnostics(Vec<Diagnostic>);

impl Diagnostics {
    /// No problem.
    #[must_use]
    pub const fn new() -> Self {
        Self(Vec::new())
    }

    /// The collection with `diagnostic` added at its end.
    #[must_use]
    pub fn with(mut self, diagnostic: Diagnostic) -> Self {
        self.0.push(diagnostic);
        self
    }

    /// The collection followed by every diagnostic of `later`.
    #[must_use]
    pub fn merge(mut self, later: Self) -> Self {
        self.0.extend(later.0);
        self
    }

    /// The diagnostics in the order they were found.
    pub fn iter(&self) -> impl Iterator<Item = &Diagnostic> + '_ {
        self.0.iter()
    }

    /// Whether any diagnostic is an error, which fails the command.
    #[must_use]
    pub fn has_errors(&self) -> bool {
        self.count(Level::Error) > 0
    }

    /// How many diagnostics are errors.
    #[must_use]
    pub fn error_count(&self) -> usize {
        self.count(Level::Error)
    }

    /// How many diagnostics are warnings.
    #[must_use]
    pub fn warning_count(&self) -> usize {
        self.count(Level::Warning)
    }

    fn count(&self, level: Level) -> usize {
        self.iter().filter(|item| item.level == level).count()
    }
}

impl FromIterator<Diagnostic> for Diagnostics {
    fn from_iter<I: IntoIterator<Item = Diagnostic>>(items: I) -> Self {
        Self(items.into_iter().collect())
    }
}

impl IntoIterator for Diagnostics {
    type Item = Diagnostic;
    type IntoIter = std::vec::IntoIter<Diagnostic>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<'a> IntoIterator for &'a Diagnostics {
    type Item = &'a Diagnostic;
    type IntoIter = std::slice::Iter<'a, Diagnostic>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

impl fmt::Display for Diagnostics {
    /// Each diagnostic on a line of its own, each line ended by a line feed.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.iter()
            .try_for_each(|diagnostic| writeln!(formatter, "{diagnostic}"))
    }
}

#[cfg(test)]
mod tests {
    use proptest::prelude::{Strategy, any, prop_assert, prop_assert_eq, proptest};

    use super::{
        Code, Diagnostic, Diagnostics, Level, Message, MessagePart, Position, Printable, Where,
        sealed,
    };
    use crate::untrusted_text::{Escaped, QuotedKey, Stripped};

    fn stripped(text: &str) -> Stripped {
        Stripped::from_untrusted(text)
    }

    fn position(line: u32, column: u32) -> Position {
        Position::new(line, column).expect("a position counted from 1")
    }

    #[test]
    fn an_error_without_a_location_has_no_where() {
        let diagnostic = Diagnostic::error(Code::NoManifest, None, Message::new("no layer.yaml"));
        assert_eq!(diagnostic.to_string(), "error [no-manifest] no layer.yaml");
    }

    #[test]
    fn a_warning_in_a_repository_file_names_the_path() {
        let diagnostic = Diagnostic::warning(
            Code::Lock,
            Some(Where::repository_file(stripped("layer.lock"))),
            Message::new("stale"),
        );
        assert_eq!(diagnostic.to_string(), "warning [lock] layer.lock: stale");
    }

    #[test]
    fn an_error_in_a_layer_file_prints_the_layer_and_the_path() {
        let diagnostic = Diagnostic::error(
            Code::Layer,
            Some(Where::layer_file(stripped("team"), stripped("layer.yaml"))),
            Message::new("unknown field ").then(QuotedKey::from_untrusted("colour")),
        );
        assert_eq!(
            diagnostic.to_string(),
            "error [layer] team:layer.yaml: unknown field 'colour'"
        );
    }

    #[test]
    fn a_yaml_error_adds_the_line_and_the_column() {
        let at = Where::layer_file(stripped("team"), stripped("layer.yaml")).at(position(3, 14));
        let diagnostic = Diagnostic::error(Code::Yaml, Some(at), Message::new("duplicate key"));
        assert_eq!(
            diagnostic.to_string(),
            "error [yaml] team:layer.yaml:3:14: duplicate key"
        );
    }

    #[test]
    fn a_path_the_person_typed_is_printed_as_given_with_its_position() {
        let at = Where::given(stripped("./some/layer")).at(position(1, 1));
        let diagnostic = Diagnostic::error(Code::YamlTag, Some(at), Message::new("tag"));
        assert_eq!(
            diagnostic.to_string(),
            "error [yaml-tag] ./some/layer:1:1: tag"
        );
    }

    #[test]
    fn a_position_counts_from_one() {
        assert_eq!(Position::new(0, 1), None);
        assert_eq!(Position::new(1, 0), None);
        assert_eq!(position(2, 5).to_string(), "2:5");
    }

    #[test]
    fn every_code_prints_in_kebab_case() {
        let printed: Vec<&str> = [
            Code::Yaml,
            Code::YamlTag,
            Code::Layer,
            Code::Manifest,
            Code::NoManifest,
            Code::Lock,
            Code::Source,
        ]
        .map(Code::as_str)
        .to_vec();
        assert_eq!(
            printed,
            [
                "yaml",
                "yaml-tag",
                "layer",
                "manifest",
                "no-manifest",
                "lock",
                "source"
            ]
        );
    }

    #[test]
    fn a_message_joins_its_parts_in_order() {
        let message = Message::new("expected ")
            .then(7_u32)
            .then(" of ")
            .then(9_u64)
            .then(" in ")
            .then(Escaped::from_untrusted("a\"b\u{202e}"))
            .then(" and ")
            .then(stripped("x\u{1b}y"));
        assert_eq!(
            message.to_string(),
            "expected 7 of 9 in \"a\\\"b\\u{202E}\" and xy"
        );
    }

    #[test]
    fn a_literal_cannot_break_the_one_line_rule() {
        assert_eq!(Message::new("a\nb\u{1b}c").to_string(), "abc");
    }

    struct Name(&'static str);

    impl sealed::Sealed for Name {}

    impl Printable for Name {
        fn printable(&self) -> &str {
            self.0
        }
    }

    #[test]
    fn a_printable_value_goes_into_a_message_as_it_is() {
        let message = Message::new("target ").then(MessagePart::printable(&Name("claude")));
        assert_eq!(message.to_string(), "target claude");
    }

    #[test]
    fn diagnostics_keep_their_order_and_count_by_level() {
        let diagnostics = Diagnostics::new()
            .with(Diagnostic::warning(Code::Lock, None, "first".into()))
            .with(Diagnostic::error(Code::Yaml, None, "second".into()))
            .merge(Diagnostics::new().with(Diagnostic::error(Code::Layer, None, "third".into())));
        assert!(diagnostics.has_errors());
        assert_eq!(diagnostics.error_count(), 2);
        assert_eq!(diagnostics.warning_count(), 1);
        assert_eq!(
            diagnostics.to_string(),
            "warning [lock] first\nerror [yaml] second\nerror [layer] third\n"
        );
        let levels: Vec<Level> = diagnostics.iter().map(Diagnostic::level).collect();
        assert_eq!(levels, [Level::Warning, Level::Error, Level::Error]);
    }

    #[test]
    fn warnings_alone_are_not_errors() {
        let diagnostics =
            Diagnostics::new().with(Diagnostic::warning(Code::Lock, None, "x".into()));
        assert!(!diagnostics.has_errors());
        assert_eq!(Diagnostics::new().to_string(), "");
    }

    /// Text drawn from every code point, and from the characters that matter to diagnostics.
    fn any_text() -> impl Strategy<Value = String> {
        proptest::prop_oneof![
            any::<String>(),
            "[a:\\[\\]'\"\\\\\u{e9}\u{200b}\u{202e}\u{2028}\u{1b}\r\n ]{0,12}"
        ]
    }

    fn any_where() -> impl Strategy<Value = Where> {
        (
            any_text(),
            any_text(),
            0..3_u8,
            proptest::option::of((1..=u32::MAX, 1..=u32::MAX)),
        )
            .prop_map(|(first, second, shape, position)| {
                let place = match shape {
                    0 => Where::layer_file(stripped(&first), stripped(&second)),
                    1 => Where::repository_file(stripped(&first)),
                    _ => Where::given(stripped(&first)),
                };
                position
                    .and_then(|(line, column)| Position::new(line, column))
                    .map_or(place.clone(), |position| place.at(position))
            })
    }

    fn any_message() -> impl Strategy<Value = Message> {
        (
            any_text(),
            any_text(),
            any_text(),
            any::<u64>(),
            any::<usize>(),
        )
            .prop_map(|(first, second, third, number, count)| {
                Message::new("problem with ")
                    .then(stripped(&first))
                    .then(" and ")
                    .then(QuotedKey::from_untrusted(&second))
                    .then(" and ")
                    .then(Escaped::from_untrusted(&third))
                    .then(": ")
                    .then(number)
                    .then(count)
            })
    }

    fn any_diagnostic() -> impl Strategy<Value = Diagnostic> {
        (
            any::<bool>(),
            0..7_u8,
            proptest::option::of(any_where()),
            any_message(),
        )
            .prop_map(|(is_error, code, at, message)| {
                let code = [
                    Code::Yaml,
                    Code::YamlTag,
                    Code::Layer,
                    Code::Manifest,
                    Code::NoManifest,
                    Code::Lock,
                    Code::Source,
                ]
                .get(usize::from(code))
                .copied()
                .unwrap_or(Code::Yaml);
                if is_error {
                    Diagnostic::error(code, at, message)
                } else {
                    Diagnostic::warning(code, at, message)
                }
            })
    }

    proptest! {
        #[test]
        fn a_rendered_diagnostic_holds_no_control_or_format_character_or_line_feed(
            diagnostic in any_diagnostic()
        ) {
            let line = diagnostic.to_string();
            // A line that stripping leaves as it is holds no control or format character.
            let restripped = Stripped::from_untrusted(&line);
            prop_assert_eq!(restripped.as_str(), line.as_str());
            prop_assert!(line.chars().all(|character| !character.is_control()));
            let line_breaks = ['\n', '\r', '\u{2028}', '\u{2029}'];
            prop_assert!(!line.contains(line_breaks));
        }

        #[test]
        fn a_rendered_diagnostic_starts_with_its_level_and_code(diagnostic in any_diagnostic()) {
            let line = diagnostic.to_string();
            let start = format!("{} [{}] ", diagnostic.level(), diagnostic.code());
            prop_assert!(line.starts_with(&start));
        }

        #[test]
        fn rendered_diagnostics_are_one_line_each(
            items in proptest::collection::vec(any_diagnostic(), 0..5)
        ) {
            let count = items.len();
            let text = items.into_iter().collect::<Diagnostics>().to_string();
            prop_assert_eq!(text.matches('\n').count(), count);
            prop_assert!(text.is_empty() || text.ends_with('\n'));
        }
    }
}
