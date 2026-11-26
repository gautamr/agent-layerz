//! `alz ls --live` (spec §10.3): what the build makes against what Claude says it loaded.
//!
//! The comparison is pure. [`Built`] is what a build makes for Claude, [`Probe`] what Claude
//! loaded, and [`Comparison`] the diagnostics and the lines that follow from the two.

use std::collections::BTreeSet;
use std::fmt;

use crate::diagnostic::{Code, Diagnostic, Diagnostics, Message};
use crate::merge::Merged;
use crate::probe::Probe;
use crate::untrusted_text::Stripped;

/// The agents and skills a build makes for Claude. The probe runs with MCP servers off (spec
/// §10.3), so the servers a build enables are not compared.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Built {
    agents: BTreeSet<String>,
    skills: BTreeSet<String>,
}

impl Built {
    /// The names of a merged stack: the agents that target Claude, every skill and command, the
    /// slot stubs.
    #[must_use]
    pub fn of(merged: &Merged) -> Self {
        Self {
            agents: merged
                .agents()
                .iter()
                .filter(|(_, agent)| {
                    agent
                        .value()
                        .targets()
                        .is_none_or(|targets| targets.contains(crate::target::claude()))
                })
                .map(|(name, _)| name.as_str().to_owned())
                .collect(),
            skills: merged
                .skills()
                .iter()
                .map(|(name, _)| name.as_str().to_owned())
                .chain(
                    merged
                        .slot_stubs()
                        .keys()
                        .map(|name| name.as_str().to_owned()),
                )
                .collect(),
        }
    }

    /// Built names, for a caller that holds them already.
    #[must_use]
    pub fn from_names(
        agents: impl IntoIterator<Item = String>,
        skills: impl IntoIterator<Item = String>,
    ) -> Self {
        Self {
            agents: agents.into_iter().collect(),
            skills: skills.into_iter().collect(),
        }
    }

    /// The agents built for Claude.
    #[must_use]
    pub const fn agents(&self) -> &BTreeSet<String> {
        &self.agents
    }

    /// The skills and commands built, slot stubs included.
    #[must_use]
    pub const fn skills(&self) -> &BTreeSet<String> {
        &self.skills
    }
}

/// The result of comparing a build with a probe.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Comparison {
    diagnostics: Diagnostics,
    loaded_agents: usize,
    loaded_skills: usize,
    extra_agents: Vec<Stripped>,
    extra_skills: Vec<Stripped>,
}

fn not_loaded<'a>(
    kind: &'static str,
    built: &'a BTreeSet<String>,
    loaded: &'a BTreeSet<String>,
) -> impl Iterator<Item = Diagnostic> + 'a {
    built.difference(loaded).map(move |name| {
        Diagnostic::error(
            Code::Ls,
            None,
            Message::new(kind)
                .then(" '")
                .then(Stripped::from_untrusted(name))
                .then("' was built but Claude did not load it"),
        )
    })
}

impl Comparison {
    /// What Claude loaded against what the build made. Items Claude loaded that no layer built
    /// are shown and not judged; skills with a `:` in the name (plugins) are left out of the
    /// skills line.
    #[must_use]
    pub fn of(built: &Built, probe: &Probe) -> Self {
        let diagnostics =
            not_loaded("agent", &built.agents, probe.agents())
                .chain(not_loaded("skill", &built.skills, probe.skills()))
                .chain(probe.warnings().iter().map(|line| {
                    Diagnostic::warning(Code::Claude, None, Message::new(line.clone()))
                }))
                .collect();
        let extras = |loaded: &BTreeSet<String>, built: &BTreeSet<String>, plugins: bool| {
            loaded
                .difference(built)
                .filter(|name| plugins || !name.contains(':'))
                .map(|name| Stripped::from_untrusted(name))
                .collect::<Vec<_>>()
        };
        Self {
            diagnostics,
            loaded_agents: probe.agents().len(),
            loaded_skills: probe.skills().len(),
            extra_agents: extras(probe.agents(), &built.agents, true),
            extra_skills: extras(probe.skills(), &built.skills, false),
        }
    }

    /// The errors and warnings, for standard error.
    #[must_use]
    pub const fn diagnostics(&self) -> &Diagnostics {
        &self.diagnostics
    }

    /// Whether Claude loaded everything the build made (a server that is not connected is a
    /// warning and does not count).
    #[must_use]
    pub fn matches(&self) -> bool {
        !self.diagnostics.has_errors()
    }
}

fn names(items: &[Stripped]) -> String {
    items
        .iter()
        .map(Stripped::as_str)
        .collect::<Vec<_>>()
        .join(", ")
}

impl fmt::Display for Comparison {
    /// The lines for standard output, each ending with a line feed.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(
            formatter,
            "claude loaded {} agents, {} skills and commands",
            self.loaded_agents, self.loaded_skills
        )?;
        if !self.extra_agents.is_empty() {
            writeln!(
                formatter,
                "  not from layers, agents: {}",
                names(&self.extra_agents)
            )?;
        }
        if !self.extra_skills.is_empty() {
            writeln!(
                formatter,
                "  not from layers, skills: {}",
                names(&self.extra_skills)
            )?;
        }
        writeln!(
            formatter,
            "{}",
            if self.matches() {
                "ls --live: Claude matches the build"
            } else {
                "ls --live: Claude does not match the build"
            }
        )
    }
}

#[cfg(test)]
mod tests;
