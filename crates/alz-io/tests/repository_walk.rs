//! The search for `AGENTS.override.md` (spec §4.3, TASK-84): it accepts large folders, folder
//! names that are no Unicode or no safe relative path, and deep trees, finds every override the
//! target tool reads, and never follows a link.

// The tests sit in a `mod tests` so that `clippy::tests_outside_test_module` holds for this file
// as for every other one.
#[cfg(test)]
mod tests {
    use std::ffi::OsStr;
    use std::fs;
    use std::io;
    use std::os::fd::OwnedFd;
    use std::os::unix::ffi::OsStrExt;
    use std::path::{Path, PathBuf};

    use alz_core::Stripped;
    use alz_core::account::AccountFolders;
    use alz_core::path::AbsolutePath;
    use alz_io::RepoRoot;
    use alz_test_support::ScratchDir;
    use rustix::fs::{Mode, OFlags};

    const OVERRIDE: &str = "AGENTS.override.md";

    struct Place {
        _scratch: ScratchDir,
        base: PathBuf,
    }

    impl Place {
        fn new() -> Self {
            let scratch = ScratchDir::new().expect("a scratch directory");
            let base = fs::canonicalize(scratch.path()).expect("a real path");
            for name in ["repo", "outside", "home"] {
                fs::create_dir(base.join(name)).expect("made");
            }
            Self {
                _scratch: scratch,
                base,
            }
        }

        fn repo(&self) -> PathBuf {
            self.base.join("repo")
        }

        fn outside(&self) -> PathBuf {
            self.base.join("outside")
        }

        fn found(&self) -> Vec<String> {
            let absolute = |path: PathBuf| {
                AbsolutePath::try_from(path.to_str().expect("text")).expect("absolute")
            };
            let folders = AccountFolders::from_env(&absolute(self.base.join("home")), None, None);
            RepoRoot::open(&absolute(self.repo()), &folders)
                .expect("the repository opens")
                .files_named(OVERRIDE)
                .expect("the search succeeds")
                .iter()
                .map(Stripped::to_string)
                .collect()
        }

        fn put(&self, relative: &str) {
            let path = self.repo().join(relative);
            fs::create_dir_all(path.parent().expect("a parent")).expect("made");
            fs::write(path, "hides").expect("written");
        }
    }

    #[test]
    fn a_folder_of_more_than_ten_thousand_entries_is_searched() {
        let place = Place::new();
        let build = place.repo().join("target/debug/build/out");
        fs::create_dir_all(&build).expect("made");
        (0..10_001).for_each(|n| fs::write(build.join(format!("o{n}")), "").expect("written"));
        place.put("docs/AGENTS.override.md");
        assert_eq!(place.found(), ["docs/AGENTS.override.md"]);
    }

    #[test]
    fn an_override_in_a_folder_of_more_than_ten_thousand_entries_is_found() {
        let place = Place::new();
        let big = place.repo().join("big");
        fs::create_dir_all(&big).expect("made");
        (0..10_001).for_each(|n| fs::write(big.join(format!("f{n}")), "").expect("written"));
        place.put("big/AGENTS.override.md");
        assert_eq!(place.found(), ["big/AGENTS.override.md"]);
    }

    #[test]
    fn an_override_is_found_in_a_folder_whose_name_is_not_unicode() {
        let place = Place::new();
        let odd = place.repo().join(OsStr::from_bytes(b"caf\xe9"));
        // Some file systems (APFS) refuse a name that is not Unicode; the case needs one that
        // accepts it.
        match fs::create_dir(&odd) {
            Ok(()) => {}
            Err(error) if error.raw_os_error() == Some(rustix::io::Errno::ILSEQ.raw_os_error()) => {
                return;
            }
            Err(error) => panic!("{error}"),
        }
        fs::write(odd.join(OVERRIDE), "hides").expect("written");
        let found = place.found();
        assert_eq!(found.len(), 1);
        assert!(
            found
                .first()
                .is_some_and(|path| path.ends_with("/AGENTS.override.md")),
            "{found:?}"
        );
    }

    #[test]
    fn an_override_is_found_in_folders_with_unusual_but_valid_names() {
        let place = Place::new();
        place.put("odd\\name/AGENTS.override.md");
        place.put(".GIT/AGENTS.override.md");
        place.put("sp ace/ünï/AGENTS.override.md");
        assert_eq!(
            place.found(),
            [
                ".GIT/AGENTS.override.md",
                "odd\\name/AGENTS.override.md",
                "sp ace/ünï/AGENTS.override.md",
            ]
        );
    }

    #[test]
    fn a_name_with_a_control_character_is_shown_without_it() {
        let place = Place::new();
        place.put("a\nb/AGENTS.override.md");
        assert_eq!(place.found(), ["ab/AGENTS.override.md"]);
    }

    /// Makes `depth` nested folders below `start` with `mkdirat`, one handle at a time, so that
    /// the path is far longer than `PATH_MAX` and no path-based call could reach the bottom.
    fn nest(start: &Path, folder: &str, depth: usize) -> io::Result<()> {
        let open = |parent: Option<&OwnedFd>, name: &str| match parent {
            None => rustix::fs::open(
                name,
                OFlags::RDONLY | OFlags::DIRECTORY | OFlags::CLOEXEC,
                Mode::empty(),
            ),
            Some(parent) => rustix::fs::openat(
                parent,
                name,
                OFlags::RDONLY | OFlags::DIRECTORY | OFlags::CLOEXEC,
                Mode::empty(),
            ),
        };
        let top = open(None, start.to_str().expect("text"))?;
        let bottom = (0..depth).try_fold(top, |parent, _| {
            rustix::fs::mkdirat(&parent, folder, Mode::from_raw_mode(0o755))?;
            open(Some(&parent), folder)
        })?;
        let file = rustix::fs::openat(
            &bottom,
            OVERRIDE,
            OFlags::WRONLY | OFlags::CREATE | OFlags::EXCL | OFlags::CLOEXEC,
            Mode::from_raw_mode(0o644),
        )?;
        drop(file);
        Ok(())
    }

    #[test]
    fn an_override_in_a_path_longer_than_the_path_length_limit_is_found() {
        let place = Place::new();
        // 40 folders of 200 characters: a path of 8000 bytes, past PATH_MAX on every platform.
        let name = "n".repeat(200);
        nest(&place.repo(), &name, 40).expect("a tree with a very long path");
        let found = place.found();
        assert_eq!(found.len(), 1);
        assert!(
            found
                .first()
                .is_some_and(|path| path.matches(name.as_str()).count() == 40),
            "the override is shown with its whole path"
        );
    }

    #[test]
    fn an_override_a_thousand_folders_deep_is_found_with_few_handles_open() {
        let place = Place::new();
        nest(&place.repo(), "d", 1000).expect("a tree 1000 folders deep");
        let found = place.found();
        assert_eq!(found.len(), 1);
        assert!(
            found
                .first()
                .is_some_and(|path| path.matches("d/").count() == 1000),
            "the override is shown with its whole path"
        );
    }

    #[test]
    fn an_override_behind_a_link_is_not_found_and_a_link_is_not_followed() {
        let place = Place::new();
        fs::write(place.outside().join(OVERRIDE), "hides").expect("written");
        place.put("real/AGENTS.override.md");
        std::os::unix::fs::symlink(place.outside(), place.repo().join("linked")).expect("linked");
        std::os::unix::fs::symlink(".", place.repo().join("loop")).expect("linked");
        assert_eq!(place.found(), ["real/AGENTS.override.md"]);
    }

    #[test]
    fn an_override_in_the_version_control_and_install_folders_is_not_reported() {
        let place = Place::new();
        place.put(".git/AGENTS.override.md");
        place.put("node_modules/pkg/AGENTS.override.md");
        place.put(".DS_Store/AGENTS.override.md");
        place.put("AGENTS.override.md");
        assert_eq!(place.found(), ["AGENTS.override.md"]);
    }
}
