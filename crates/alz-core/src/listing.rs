//! What a build makes, by kind of item, for `alz ls` (spec §10.3): six sections, each with its
//! count and, per item, the name and the layer that decides it.
//!
//! The listing is read from the merged model alone. Every name is stripped before it is stored,
//! so the text a person sees cannot hold a control or format character, whatever a layer wrote.

use std::fmt;

use crate::item::Skill;
use crate::layer::LayerName;
use crate::merge::{Merged, Traced};
use crate::untrusted_text::Stripped;

/// The width the names are padded to (§10.3).
const NAME_WIDTH: usize = 28;
/// What stands for the layer of a slot stub, which no layer defines.
const SLOT_STUB: &str = "slot stub";

/// A section of the listing, in the order the listing prints them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Section {
    /// `agents`.
    Agents,
    /// `skills`, the slot stubs included.
    Skills,
    /// `commands`.
    Commands,
    /// `rules`.
    Rules,
    /// `hooks`.
    Hooks,
    /// `mcp`: the enabled servers.
    Mcp,
}

impl Section {
    /// The heading, as the listing prints it.
    #[must_use]
    pub const fn title(self) -> &'static str {
        match self {
            Self::Agents => "agents",
            Self::Skills => "skills",
            Self::Commands => "commands",
            Self::Rules => "rules",
            Self::Hooks => "hooks",
            Self::Mcp => "mcp",
        }
    }
}

/// Who decides an item: the most specific layer that contributed to it, or nobody for a slot
/// stub, which no layer defines.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Decider {
    /// A layer of the stack.
    Layer(LayerName),
    /// The stub of a slot no layer filled (§7.3).
    SlotStub,
}

impl fmt::Display for Decider {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Layer(layer) => layer.fmt(formatter),
            Self::SlotStub => formatter.write_str(SLOT_STUB),
        }
    }
}

/// One line of a section.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListedItem {
    name: Stripped,
    decider: Decider,
}

impl ListedItem {
    /// The item's name, stripped.
    #[must_use]
    pub fn name(&self) -> &str {
        self.name.as_str()
    }

    /// Who decides the item.
    #[must_use]
    pub const fn decider(&self) -> &Decider {
        &self.decider
    }

    /// An item decided by its most specific contribution; `None` for a provenance with none,
    /// which a merged item never has.
    fn of<T>(name: &str, traced: &Traced<T>) -> Option<Self> {
        traced.provenance().contributions().last().map(|last| Self {
            name: Stripped::from_untrusted(name),
            decider: Decider::Layer(last.layer().clone()),
        })
    }

    fn stub(name: &str) -> Self {
        Self {
            name: Stripped::from_untrusted(name),
            decider: Decider::SlotStub,
        }
    }
}

impl fmt::Display for ListedItem {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "  {:<width$} {}",
            self.name.as_str(),
            self.decider,
            width = NAME_WIDTH
        )
    }
}

/// What a build makes, by section.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Listing {
    agents: Vec<ListedItem>,
    skills: Vec<ListedItem>,
    commands: Vec<ListedItem>,
    rules: Vec<ListedItem>,
    hooks: Vec<ListedItem>,
    mcp: Vec<ListedItem>,
}

impl Listing {
    /// The listing of a merged stack: items in name order, hooks in stack and file order, and
    /// servers in the order of `mcp.enable`.
    #[must_use]
    pub fn of(merged: &Merged) -> Self {
        let defined = |directories: bool| {
            merged
                .skills()
                .iter()
                .filter(move |(_, traced)| {
                    matches!(traced.value(), Skill::Directory(_)) == directories
                })
                .filter_map(|(name, traced)| ListedItem::of(name.as_str(), traced))
        };
        let stubs = merged
            .slot_stubs()
            .keys()
            .map(|name| ListedItem::stub(name.as_str()));
        let mut skills: Vec<ListedItem> = defined(true).chain(stubs).collect();
        skills.sort_by(|first, second| first.name.cmp(&second.name));
        Self {
            agents: merged
                .agents()
                .iter()
                .filter_map(|(name, traced)| ListedItem::of(name.as_str(), traced))
                .collect(),
            skills,
            commands: defined(false).collect(),
            rules: merged
                .rules()
                .iter()
                .filter_map(|(name, traced)| ListedItem::of(&name.to_string(), traced))
                .collect(),
            hooks: merged
                .hooks()
                .iter()
                .filter_map(|traced| ListedItem::of(traced.value().id().as_str(), traced))
                .collect(),
            mcp: merged
                .mcp()
                .enabled_servers()
                .filter_map(|(id, traced)| ListedItem::of(id.as_str(), traced))
                .collect(),
        }
    }

    /// The sections in print order, each with its items.
    #[must_use]
    pub fn sections(&self) -> [(Section, &[ListedItem]); 6] {
        [
            (Section::Agents, &self.agents),
            (Section::Skills, &self.skills),
            (Section::Commands, &self.commands),
            (Section::Rules, &self.rules),
            (Section::Hooks, &self.hooks),
            (Section::Mcp, &self.mcp),
        ]
    }
}

impl fmt::Display for Listing {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.sections()
            .into_iter()
            .try_for_each(|(section, items)| {
                writeln!(formatter, "{} ({})", section.title(), items.len())?;
                items
                    .iter()
                    .try_for_each(|item| writeln!(formatter, "{item}"))
            })
    }
}

#[cfg(test)]
mod tests;
