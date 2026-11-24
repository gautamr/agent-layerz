//! A fake machine for the account-file tests: a scratch folder with a home folder, a repository
//! and a folder for everything else, all under the real path of the scratch folder.

use std::fs;
use std::path::PathBuf;

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
}
