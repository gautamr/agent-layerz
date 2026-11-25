//! Spec §13 item 25, "Name order" (TASK-64), through the binary with the item's own names:
//! instruction files `z.md`, `｡.md` (U+FF61) and `😀.md` (U+1F600) give `AGENTS.md` blocks in that
//! order, the layer's content hash in `layer.lock` lists its files in that order (§5.3),
//! `.claude/settings.json` keys `-x`, `10`, `9`, `a`, `｡`, `😀` appear in that order, and
//! `alz check` then passes. UTF-16 order would put `😀` before `｡` in each place. Every run has
//! its own home, `XDG_*` folders and cache.

// The tests sit in a `mod tests` so that `clippy::tests_outside_test_module` holds for this file
// as for every other one.
#[cfg(test)]
mod tests {
    use std::time::Duration;

    use alz_test_support::ScratchDir;
    use assert_cmd::{Command, cargo::cargo_bin_cmd};

    /// The sha256 of the record text of §5.3 for the five files of the layer below, in code point
    /// order of the whole path (`instructions/z.md`, `instructions/｡.md`, `instructions/😀.md`,
    /// `layer.yaml`, `settings.yaml`), worked out with a separate sha256 outside alz. UTF-16 order
    /// would swap the second and the third record and give another hash.
    const CONTENT_HASH: &str = "d152999b484ef68c2ac42bad6d947aba97e8aa85ec28e38880244740eafe982f";

    const LAYER: &str = "name: names\nkind: repo\nvisibility: private\n";
    const SETTINGS: &str = "claude:\n  modelOverrides:\n    \"\u{FF61}\": m\n    \"\u{1F600}\": m\n    \"10\": m\n    \"9\": m\n    \"-x\": m\n    \"a\": m\n";

    fn repository() -> ScratchDir {
        let scratch = ScratchDir::new().expect("a scratch directory");
        for folder in [
            "home",
            "config",
            "state",
            "cache",
            "repo/.layer/instructions",
        ] {
            scratch.make_dirs(folder).expect("made");
        }
        let files = [
            (
                "repo/layer.yaml",
                "stack:\n  - source: path:./.layer\ntargets: [claude]\n",
            ),
            ("repo/.layer/layer.yaml", LAYER),
            ("repo/.layer/settings.yaml", SETTINGS),
            ("repo/.layer/instructions/z.md", "zed\n"),
            ("repo/.layer/instructions/\u{FF61}.md", "dot\n"),
            ("repo/.layer/instructions/\u{1F600}.md", "smile\n"),
        ];
        for (path, text) in files {
            scratch.write(path, text).expect("written");
        }
        scratch
    }

    fn alz(scratch: &ScratchDir, command: &str) -> (i32, String, String) {
        let mut alz: Command = cargo_bin_cmd!("alz");
        let output = alz
            .current_dir(scratch.join("repo"))
            .env_clear()
            .env("PATH", "/usr/bin:/bin")
            .env("HOME", scratch.join("home"))
            .env("XDG_CONFIG_HOME", scratch.join("config"))
            .env("XDG_STATE_HOME", scratch.join("state"))
            .env("XDG_CACHE_HOME", scratch.join("cache"))
            .timeout(Duration::from_secs(60))
            .arg(command)
            .output()
            .expect("alz runs and ends in time");
        (
            output.status.code().expect("an exit code"),
            String::from_utf8_lossy(&output.stdout).into_owned(),
            String::from_utf8_lossy(&output.stderr).into_owned(),
        )
    }

    /// Spec §13 item 25: the three blocks of `AGENTS.md` come in code point order of the file
    /// names, the lock's content hash is that of the files in code point order, the keys of
    /// `settings.json` are in code point order, and `alz check` passes.
    #[test]
    fn instruction_files_the_layer_hash_and_settings_keys_follow_code_point_order_and_check_passes()
    {
        let scratch = repository();
        let (code, stdout, stderr) = alz(&scratch, "build");
        assert_eq!(code, 0, "{stdout}{stderr}");

        let agents = scratch.read_text("repo/AGENTS.md").expect("AGENTS.md");
        let position = |word: &str| {
            agents
                .find(word)
                .unwrap_or_else(|| panic!("{word} is in AGENTS.md: {agents}"))
        };
        assert!(
            position("zed") < position("dot") && position("dot") < position("smile"),
            "{agents}"
        );

        let lock = scratch.read_text("repo/layer.lock").expect("layer.lock");
        assert!(
            lock.contains(&format!("\"contentHash\": \"sha256:{CONTENT_HASH}\"")),
            "{lock}"
        );

        let settings = scratch
            .read_text("repo/.claude/settings.json")
            .expect("settings.json");
        let keys: Vec<&str> = settings
            .lines()
            .filter_map(|line| line.strip_prefix("    \""))
            .filter_map(|line| line.split_once("\": "))
            .map(|(key, _)| key)
            .collect();
        assert_eq!(keys, ["-x", "10", "9", "a", "\u{FF61}", "\u{1F600}"]);

        let (code, stdout, stderr) = alz(&scratch, "check");
        assert_eq!(code, 0, "{stdout}{stderr}");
    }
}
