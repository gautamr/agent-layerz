//! What the `claude` program sees: nothing but what the call needs, taken from alz's own
//! environment (spec §10.3). Variables are chosen by name, so a variable that holds a secret for
//! some other program is never handed over.

use std::ffi::{OsStr, OsString};
use std::os::unix::ffi::OsStrExt;
use std::time::Duration;

use alz_core::env::EnvVars;

use super::locate::absolute_entries;

/// What a run is for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Purpose {
    /// `claude --version` for `alz doctor`: it needs no login and no network.
    Version,
    /// The one short print-mode session of `alz ls --live`: it needs the login, which is the
    /// account folder, the keychain or an API key, and the proxy and certificate settings.
    Probe,
}

impl Purpose {
    /// What the version call may take.
    const VERSION_DEADLINE: Duration = Duration::from_secs(10);
    /// What the one model call may take.
    const PROBE_DEADLINE: Duration = Duration::from_secs(180);
    /// How much of standard error is kept.
    pub(super) const STDERR_CAP: usize = 64 * 1024;

    pub(super) const fn deadline(self) -> Duration {
        match self {
            Self::Version => Self::VERSION_DEADLINE,
            Self::Probe => Self::PROBE_DEADLINE,
        }
    }

    /// How much of standard output is kept: the first events of a one-turn session fit easily.
    pub(super) const fn stdout_cap(self) -> usize {
        match self {
            Self::Version => 64 * 1024,
            Self::Probe => 4 * 1024 * 1024,
        }
    }

    fn takes(self, name: &OsStr) -> bool {
        let bytes = name.as_bytes();
        let common = matches!(
            bytes,
            b"HOME"
                | b"USER"
                | b"LOGNAME"
                | b"TMPDIR"
                | b"TERM"
                | b"LANG"
                | b"CLAUDE_CONFIG_DIR"
                | b"XDG_CONFIG_HOME"
                | b"XDG_DATA_HOME"
                | b"XDG_STATE_HOME"
                | b"XDG_CACHE_HOME"
        ) || bytes.starts_with(b"LC_");
        match self {
            Self::Version => common,
            Self::Probe => {
                common
                    || [
                        b"ANTHROPIC_".as_slice(),
                        b"CLAUDE_CODE_",
                        b"SSL_CERT_",
                        b"NODE_EXTRA_CA_CERTS",
                    ]
                    .iter()
                    .any(|prefix| bytes.starts_with(prefix))
                    || matches!(
                        bytes.to_ascii_uppercase().as_slice(),
                        b"HTTP_PROXY" | b"HTTPS_PROXY" | b"ALL_PROXY" | b"NO_PROXY"
                    )
            }
        }
    }
}

/// The environment of the program: the variables `purpose` takes, and a `PATH` with only the
/// absolute entries of alz's (the program may start others, such as `node`).
pub(super) fn child_environment(purpose: Purpose, vars: &EnvVars) -> EnvVars {
    let path = vars.get("PATH").map(|path| {
        absolute_entries(path).fold(OsString::new(), |joined, entry| {
            let separator = if joined.is_empty() { "" } else { ":" };
            [joined.as_os_str(), OsStr::new(separator), entry.as_os_str()]
                .into_iter()
                .collect()
        })
    });
    path.into_iter()
        .map(|path| ("PATH".into(), path))
        .chain(
            vars.iter()
                .filter(|(name, _)| purpose.takes(name))
                .map(|(name, value)| (name.to_owned(), value.to_owned())),
        )
        .collect()
}

#[cfg(test)]
mod tests {
    use std::ffi::OsString;

    use alz_core::env::EnvVars;

    use super::{Purpose, child_environment};

    fn vars(pairs: &[(&str, &str)]) -> EnvVars {
        pairs
            .iter()
            .map(|(name, value)| (OsString::from(name), OsString::from(value)))
            .collect()
    }

    fn names(vars: &EnvVars) -> Vec<String> {
        vars.iter()
            .map(|(name, _)| name.to_string_lossy().into_owned())
            .collect()
    }

    const PROCESS: &[(&str, &str)] = &[
        ("PATH", "/usr/bin:.:rel:/bin"),
        ("HOME", "/h"),
        ("CLAUDE_CONFIG_DIR", "/c"),
        ("ANTHROPIC_API_KEY", "k"),
        ("https_proxy", "p"),
        ("AWS_SECRET_ACCESS_KEY", "s"),
        ("GITHUB_TOKEN", "t"),
        ("LC_ALL", "C"),
    ];

    #[test]
    fn the_version_call_gets_the_home_and_locale_and_a_path_of_absolute_entries() {
        let env = child_environment(Purpose::Version, &vars(PROCESS));
        assert_eq!(names(&env), ["CLAUDE_CONFIG_DIR", "HOME", "LC_ALL", "PATH"]);
        assert_eq!(
            env.get("PATH").and_then(|path| path.to_str()),
            Some("/usr/bin:/bin")
        );
    }

    #[test]
    fn the_probe_also_gets_the_login_and_the_proxy_but_no_unrelated_secret() {
        let env = child_environment(Purpose::Probe, &vars(PROCESS));
        assert_eq!(
            names(&env),
            [
                "ANTHROPIC_API_KEY",
                "CLAUDE_CONFIG_DIR",
                "HOME",
                "LC_ALL",
                "PATH",
                "https_proxy"
            ]
        );
    }

    #[test]
    fn without_a_path_the_program_gets_none() {
        let env = child_environment(Purpose::Version, &vars(&[("HOME", "/h")]));
        assert_eq!(names(&env), ["HOME"]);
    }
}
