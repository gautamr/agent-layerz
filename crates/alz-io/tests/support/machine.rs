//! A fake machine for the directory tests: a scratch folder with a home folder, a repository and
//! a folder for everything else, all under the real path of the scratch folder.

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use alz_core::account::AccountFolders;
use alz_core::path::AbsolutePath;
use alz_test_support::ScratchDir;

pub struct Machine {
    _scratch: ScratchDir,
    root: PathBuf,
}

impl Machine {
    /// `home`, `repo` and `other` exist, all mode 0755.
    pub fn new() -> Self {
        let scratch = ScratchDir::new().expect("a scratch directory");
        let root = fs::canonicalize(scratch.path()).expect("a real path");
        for name in ["home", "repo", "other"] {
            fs::create_dir(root.join(name)).expect("made");
        }
        Self {
            _scratch: scratch,
            root,
        }
    }

    pub fn path(&self, relative: &str) -> PathBuf {
        self.root.join(relative)
    }

    pub fn abs(&self, relative: &str) -> AbsolutePath {
        AbsolutePath::try_from(self.path(relative).to_str().expect("text")).expect("absolute")
    }

    pub fn make_dirs(&self, relative: &str) {
        fs::create_dir_all(self.path(relative)).expect("made");
    }

    pub fn write(&self, relative: &str, text: &str) {
        fs::write(self.path(relative), text).expect("written");
    }

    pub fn link(&self, relative: &str, target: &str) {
        std::os::unix::fs::symlink(target, self.path(relative)).expect("linked");
    }

    /// The account folders for `HOME=<root>/home` and the two variables, given relative to the
    /// root.
    pub fn folders(
        &self,
        claude_config_dir: Option<&str>,
        codex_home: Option<&str>,
    ) -> AccountFolders {
        AccountFolders::from_env(
            &self.abs("home"),
            claude_config_dir.map(|dir| self.abs(dir)).as_ref(),
            codex_home.map(|dir| self.abs(dir)).as_ref(),
        )
    }

    /// Makes the folder mode 000 until the returned guard is dropped, so any access inside it
    /// fails visibly and the scratch folder can still be removed.
    pub fn lock_down(&self, relative: &str) -> LockedDown {
        let path = self.path(relative);
        fs::set_permissions(&path, fs::Permissions::from_mode(0o000)).expect("locked");
        LockedDown(path)
    }
}

pub struct LockedDown(PathBuf);

impl Drop for LockedDown {
    fn drop(&mut self) {
        restore(&self.0);
    }
}

fn restore(path: &Path) {
    fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap_or_default();
}
