//! `~` for `$HOME` in the paths a report shows (spec §10.3).

use std::fmt;

use crate::account::resolution::{HomeFact, folder_name};
use crate::target::TargetName;
use crate::untrusted_text::Stripped;

/// `~` for `$HOME`, as given and as its real path.
pub(crate) struct Shower {
    bases: Vec<String>,
}

impl Shower {
    pub(crate) fn of(home: &HomeFact) -> Self {
        let bases = match home {
            HomeFact::Absolute { given, real } => [Some(given), real.as_ref()]
                .into_iter()
                .flatten()
                .map(|path| path.as_str().trim_end_matches('/').to_owned())
                .collect(),
            HomeFact::NotAbsolute => Vec::new(),
        };
        Self { bases }
    }

    pub(crate) fn text(&self, text: &str) -> String {
        self.bases
            .iter()
            .find_map(|base| {
                text.strip_prefix(base.as_str())
                    .filter(|rest| !base.is_empty() && (rest.is_empty() || rest.starts_with('/')))
                    .map(|rest| format!("~{rest}"))
            })
            .unwrap_or_else(|| text.to_owned())
    }

    pub(crate) fn path(&self, text: &str) -> Stripped {
        Stripped::from_untrusted(&self.text(text))
    }

    /// `~/.claude-<name>`, as `$HOME` as given would build it.
    pub(crate) fn folder(
        &self,
        home: &HomeFact,
        target: TargetName,
        name: &impl fmt::Display,
    ) -> Stripped {
        match home {
            HomeFact::Absolute { given, .. } => self.path(&format!(
                "{}/{}",
                given.as_str().trim_end_matches('/'),
                folder_name(target, name)
            )),
            HomeFact::NotAbsolute => Stripped::from_untrusted(&folder_name(target, name)),
        }
    }
}
