//! Whether a person is at the keyboard (spec §7.13 "Trust and `alz account allow`").
//!
//! `alz account allow` is a decision only a person may take, so it refuses when standard input is
//! not a terminal or when an agent session is running. This module is the pure part: the six
//! agent-session markers, and the decision from the two facts. Reading the terminal and the
//! environment is `alz-io`'s `PersonAtTerminal`.

use crate::env::EnvVars;

/// A variable whose presence says an agent session is running (spec §7.13, §12 item 19). The
/// variants are in the order the spec names them, which is the order a message lists them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AgentSessionMarker {
    /// `CLAUDECODE`, which Claude Code also sets in an IDE's integrated terminal.
    ClaudeCode,
    /// `CLAUDE_CODE_CHILD_SESSION`.
    ClaudeCodeChildSession,
    /// `AI_AGENT`.
    AiAgent,
    /// `CODEX_THREAD_ID`.
    CodexThreadId,
    /// `CODEX_SESSION_ID`.
    CodexSessionId,
    /// `CODEX_CI`.
    CodexCi,
}

impl AgentSessionMarker {
    /// Every marker, in the spec's order.
    pub const ALL: [Self; 6] = [
        Self::ClaudeCode,
        Self::ClaudeCodeChildSession,
        Self::AiAgent,
        Self::CodexThreadId,
        Self::CodexSessionId,
        Self::CodexCi,
    ];

    /// The variable's name. Only the name is ever printed, never the value.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::ClaudeCode => "CLAUDECODE",
            Self::ClaudeCodeChildSession => "CLAUDE_CODE_CHILD_SESSION",
            Self::AiAgent => "AI_AGENT",
            Self::CodexThreadId => "CODEX_THREAD_ID",
            Self::CodexSessionId => "CODEX_SESSION_ID",
            Self::CodexCi => "CODEX_CI",
        }
    }
}

/// The markers set in `env`, in the spec's order. A marker counts even with an empty value, and
/// no value is read.
#[must_use]
pub fn markers_set(env: &EnvVars) -> Vec<AgentSessionMarker> {
    AgentSessionMarker::ALL
        .into_iter()
        .filter(|marker| env.get(marker.name()).is_some())
        .collect()
}

/// An agent session: at least one marker is set. There is no value with none.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentSession {
    first: AgentSessionMarker,
    rest: Vec<AgentSessionMarker>,
}

impl AgentSession {
    /// The session `env` shows, or `None` when no marker is set.
    #[must_use]
    pub fn in_env(env: &EnvVars) -> Option<Self> {
        let mut markers = markers_set(env).into_iter();
        markers.next().map(|first| Self {
            first,
            rest: markers.collect(),
        })
    }

    /// The markers that are set, in the spec's order; never empty.
    pub fn markers(&self) -> impl Iterator<Item = AgentSessionMarker> + '_ {
        std::iter::once(self.first).chain(self.rest.iter().copied())
    }

    /// Whether `CLAUDECODE` is among them, for the note about IDE terminals.
    #[must_use]
    pub fn includes_claude_code(&self) -> bool {
        self.markers()
            .any(|marker| marker == AgentSessionMarker::ClaudeCode)
    }
}

/// Why nobody counts as a person at a terminal. Every reason that applies is held, so a message
/// names them all.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NotAPerson {
    /// Standard input is not a terminal.
    NoTerminal,
    /// An agent session is running.
    AgentSession(AgentSession),
    /// Both.
    NoTerminalInAgentSession(AgentSession),
}

impl NotAPerson {
    /// Whether standard input is not a terminal.
    #[must_use]
    pub const fn lacks_terminal(&self) -> bool {
        match self {
            Self::NoTerminal | Self::NoTerminalInAgentSession(_) => true,
            Self::AgentSession(_) => false,
        }
    }

    /// The agent session, when one is running.
    #[must_use]
    pub const fn agent_session(&self) -> Option<&AgentSession> {
        match self {
            Self::NoTerminal => None,
            Self::AgentSession(session) | Self::NoTerminalInAgentSession(session) => Some(session),
        }
    }
}

/// Decides whether a person is at the terminal: standard input is a terminal and no marker is
/// set.
///
/// # Errors
///
/// [`NotAPerson`], with every reason that applies.
pub fn decide(stdin_is_terminal: bool, env: &EnvVars) -> Result<(), NotAPerson> {
    match (stdin_is_terminal, AgentSession::in_env(env)) {
        (true, None) => Ok(()),
        (false, None) => Err(NotAPerson::NoTerminal),
        (true, Some(session)) => Err(NotAPerson::AgentSession(session)),
        (false, Some(session)) => Err(NotAPerson::NoTerminalInAgentSession(session)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env_with(names: &[&str]) -> EnvVars {
        names
            .iter()
            .fold(EnvVars::default(), |env, name| env.with(*name, ""))
    }

    #[test]
    fn a_terminal_and_no_marker_is_a_person() {
        assert_eq!(decide(true, &env_with(&["HOME", "PATH"])), Ok(()));
    }

    #[test]
    fn every_marker_alone_refuses_even_with_an_empty_value() {
        for marker in AgentSessionMarker::ALL {
            let refused = decide(true, &env_with(&[marker.name()])).expect_err("a marker is set");
            assert!(!refused.lacks_terminal());
            let session = refused.agent_session().expect("an agent session");
            assert_eq!(session.markers().collect::<Vec<_>>(), vec![marker]);
        }
    }

    #[test]
    fn markers_are_listed_in_the_specs_order_whatever_the_environment_order() {
        let env = env_with(&["CODEX_CI", "AI_AGENT", "CLAUDECODE"]);
        let refused = decide(true, &env).expect_err("markers are set");
        let session = refused.agent_session().expect("an agent session");
        assert_eq!(
            session
                .markers()
                .map(AgentSessionMarker::name)
                .collect::<Vec<_>>(),
            ["CLAUDECODE", "AI_AGENT", "CODEX_CI"]
        );
        assert!(session.includes_claude_code());
    }

    #[test]
    fn no_terminal_and_a_marker_name_both_reasons() {
        let refused = decide(false, &env_with(&["CODEX_THREAD_ID"])).expect_err("both apply");
        assert!(refused.lacks_terminal());
        assert!(
            !refused
                .agent_session()
                .expect("a session")
                .includes_claude_code()
        );
    }

    #[test]
    fn no_terminal_alone_names_only_that_reason() {
        assert_eq!(
            decide(false, &EnvVars::default()),
            Err(NotAPerson::NoTerminal)
        );
    }

    #[test]
    fn a_similar_variable_is_not_a_marker() {
        assert_eq!(
            decide(true, &env_with(&["CLAUDE_CODE", "CODEX_HOME"])),
            Ok(())
        );
    }
}
