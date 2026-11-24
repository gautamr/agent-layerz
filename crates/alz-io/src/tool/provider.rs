//! Which cloud provider the `claude` program is switched to, and so which of the provider's own
//! variables it may see (spec §10.3, target-facts §15). Claude Code reaches Amazon Bedrock or
//! Google Cloud's Agent Platform with the credentials in the environment of the process; without
//! the switch those variables are of no use to it, and are secrets for some other program.

use std::ffi::OsStr;
use std::os::unix::ffi::OsStrExt;

use alz_core::env::EnvVars;

/// The providers that are switched on in an environment. Each is on when its switch variable is
/// `1` or `true` (ASCII case-insensitive, white space not trimmed). The docs show `=1` and say an
/// empty value is unset; they name no other off value, so any other value is off: fewer
/// credentials reach the child when in doubt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Providers {
    bedrock: bool,
    vertex: bool,
}

impl Providers {
    /// What `vars` switches on.
    pub(super) fn switched_on_in(vars: &EnvVars) -> Self {
        let is_on = |name: &str| {
            vars.get(name).is_some_and(|value| {
                let bytes = value.as_bytes();
                bytes == b"1" || bytes.eq_ignore_ascii_case(b"true")
            })
        };
        Self {
            bedrock: is_on("CLAUDE_CODE_USE_BEDROCK"),
            vertex: is_on("CLAUDE_CODE_USE_VERTEX"),
        }
    }

    /// Whether `name` is a variable of a provider that is switched on: `AWS_*` for Bedrock;
    /// `GOOGLE_*`, `GCLOUD_PROJECT`, `VERTEX_REGION_CLAUDE_*`, `CLOUD_ML_REGION` and
    /// `ANTHROPIC_VERTEX_PROJECT_ID` for Vertex. The switch
    /// variables themselves are `CLAUDE_CODE_*` and pass with that prefix.
    pub(super) fn take(self, name: &OsStr) -> bool {
        let bytes = name.as_bytes();
        (self.bedrock && bytes.starts_with(b"AWS_"))
            || (self.vertex
                && (bytes.starts_with(b"GOOGLE_")
                    || bytes.starts_with(b"VERTEX_REGION_CLAUDE_")
                    || bytes == b"GCLOUD_PROJECT"
                    || Self::is_vertex_setting(bytes)))
    }

    /// Whether `name` is a variable of a provider that is switched off, and so is withheld even
    /// where another rule would pass it: the `ANTHROPIC_*` prefix would pass
    /// `ANTHROPIC_VERTEX_PROJECT_ID` without the Vertex switch.
    pub(super) fn withholds(self, name: &OsStr) -> bool {
        !self.vertex && Self::is_vertex_setting(name.as_bytes())
    }

    fn is_vertex_setting(bytes: &[u8]) -> bool {
        matches!(bytes, b"CLOUD_ML_REGION" | b"ANTHROPIC_VERTEX_PROJECT_ID")
    }
}

#[cfg(test)]
mod tests {
    use std::ffi::{OsStr, OsString};

    use alz_core::env::EnvVars;

    use super::Providers;

    fn vars(pairs: &[(&str, &str)]) -> EnvVars {
        pairs
            .iter()
            .map(|(name, value)| (OsString::from(name), OsString::from(value)))
            .collect()
    }

    /// The names of `NAMES` that `vars` lets through.
    fn passing(vars: &EnvVars) -> Vec<&'static str> {
        const NAMES: [&str; 10] = [
            "AWS_PROFILE",
            "AWS_BEARER_TOKEN_BEDROCK",
            "GOOGLE_APPLICATION_CREDENTIALS",
            "CLOUD_ML_REGION",
            "ANTHROPIC_VERTEX_PROJECT_ID",
            "GCLOUD_PROJECT",
            "VERTEX_REGION_CLAUDE_HAIKU_4_5",
            "GITHUB_TOKEN",
            "AWSX",
            "aws_profile",
        ];
        let providers = Providers::switched_on_in(vars);
        NAMES
            .into_iter()
            .filter(|name| providers.take(OsStr::new(name)))
            .collect()
    }

    #[test]
    fn with_no_switch_no_provider_variable_passes() {
        assert_eq!(passing(&vars(&[("AWS_PROFILE", "p")])), Vec::<&str>::new());
    }

    #[test]
    fn a_switch_set_to_anything_but_1_or_true_is_off() {
        for value in ["", "0", "false", "yes", "on", " 1", "1 ", "TRUE1", "2"] {
            let env = vars(&[
                ("CLAUDE_CODE_USE_BEDROCK", value),
                ("CLAUDE_CODE_USE_VERTEX", value),
            ]);
            assert_eq!(passing(&env), Vec::<&str>::new(), "value {value:?}");
        }
    }

    #[test]
    fn a_switch_set_to_1_or_true_in_any_case_is_on() {
        for value in ["1", "true", "TRUE", "True", "tRuE"] {
            assert_eq!(
                passing(&vars(&[("CLAUDE_CODE_USE_BEDROCK", value)])),
                ["AWS_PROFILE", "AWS_BEARER_TOKEN_BEDROCK"],
                "value {value:?}"
            );
        }
    }

    #[test]
    fn a_switched_off_vertex_withholds_its_project_even_from_the_anthropic_prefix() {
        let off = Providers::switched_on_in(&vars(&[]));
        let on = Providers::switched_on_in(&vars(&[("CLAUDE_CODE_USE_VERTEX", "1")]));
        let project = OsStr::new("ANTHROPIC_VERTEX_PROJECT_ID");
        assert!(off.withholds(project) && !on.withholds(project));
        assert!(!off.withholds(OsStr::new("ANTHROPIC_API_KEY")));
    }

    #[test]
    fn the_bedrock_switch_passes_aws_variables_only() {
        assert_eq!(
            passing(&vars(&[("CLAUDE_CODE_USE_BEDROCK", "1")])),
            ["AWS_PROFILE", "AWS_BEARER_TOKEN_BEDROCK"]
        );
    }

    #[test]
    fn the_vertex_switch_passes_the_vertex_set_only() {
        assert_eq!(
            passing(&vars(&[("CLAUDE_CODE_USE_VERTEX", "1")])),
            [
                "GOOGLE_APPLICATION_CREDENTIALS",
                "CLOUD_ML_REGION",
                "ANTHROPIC_VERTEX_PROJECT_ID",
                "GCLOUD_PROJECT",
                "VERTEX_REGION_CLAUDE_HAIKU_4_5"
            ]
        );
    }

    #[test]
    fn both_switches_pass_both_sets_and_never_an_unrelated_name() {
        assert_eq!(
            passing(&vars(&[
                ("CLAUDE_CODE_USE_BEDROCK", "1"),
                ("CLAUDE_CODE_USE_VERTEX", "1")
            ])),
            [
                "AWS_PROFILE",
                "AWS_BEARER_TOKEN_BEDROCK",
                "GOOGLE_APPLICATION_CREDENTIALS",
                "CLOUD_ML_REGION",
                "ANTHROPIC_VERTEX_PROJECT_ID",
                "GCLOUD_PROJECT",
                "VERTEX_REGION_CLAUDE_HAIKU_4_5"
            ]
        );
    }
}
