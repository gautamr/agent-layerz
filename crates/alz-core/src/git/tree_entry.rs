//! The output of `git ls-tree -z` (spec §6 "Git subprocesses"): one record per entry, each
//! `<mode> SP <type> SP <object> TAB <path> NUL`.

use crate::git::object_kind::{ObjectKind, UnreadableOutput};
use crate::lock::ObjectId;

/// What kind of entry a tree holds, from its mode. The mode and the object type of git's output
/// must agree, so a value of this type cannot say a directory is a blob.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EntryMode {
    /// A subtree (mode `040000`, type `tree`).
    Directory,
    /// A regular file (mode `100644`, or any `100xxx`), with its owner-execute bit.
    File {
        /// Whether the owner-execute bit is set (mode `100755`).
        executable: bool,
    },
    /// A symbolic link (mode `120000`).
    Link,
    /// A submodule (mode `160000`, type `commit`).
    Submodule,
}

impl EntryMode {
    /// The entry kind for the octal `mode` text and the type word that git printed together,
    /// or `None` when either is unknown or they disagree.
    fn from_words(mode: &str, kind: &str) -> Option<Self> {
        let bits = u32::from_str_radix(mode, 8).ok()?;
        let kind = ObjectKind::from_word(kind)?;
        match (bits & 0o170_000, kind) {
            (0o040_000, ObjectKind::Tree) => Some(Self::Directory),
            (0o100_000, ObjectKind::Blob) => Some(Self::File {
                executable: bits & 0o100 != 0,
            }),
            (0o120_000, ObjectKind::Blob) => Some(Self::Link),
            (0o160_000, ObjectKind::Commit) => Some(Self::Submodule),
            _ => None,
        }
    }
}

/// One entry of `git ls-tree`: its kind, its object id and its path as git printed it (bytes:
/// whether they are UTF-8 and a safe path is for the caller to judge).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TreeEntry {
    /// What the entry is.
    pub mode: EntryMode,
    /// The object the entry names.
    pub id: ObjectId,
    /// The path, relative to the listed tree.
    pub path: Vec<u8>,
}

impl TreeEntry {
    /// One record, without its terminating NUL.
    fn parse(record: &[u8]) -> Result<Self, UnreadableOutput> {
        let tab = record
            .iter()
            .position(|byte| *byte == b'\t')
            .ok_or(UnreadableOutput::new("an ls-tree record without a tab"))?;
        let (meta, path) = record
            .split_at_checked(tab)
            .ok_or(UnreadableOutput::new("an ls-tree record without a tab"))?;
        let meta = std::str::from_utf8(meta)
            .map_err(|_cause| UnreadableOutput::new("an ls-tree record that is not ASCII"))?;
        let mut words = meta.split(' ');
        let (Some(mode), Some(kind), Some(id), None) =
            (words.next(), words.next(), words.next(), words.next())
        else {
            return Err(UnreadableOutput::new(
                "an ls-tree record without mode, type and object",
            ));
        };
        Ok(Self {
            mode: EntryMode::from_words(mode, kind)
                .ok_or(UnreadableOutput::new("an ls-tree mode or type"))?,
            id: ObjectId::try_from(id)
                .map_err(|_cause| UnreadableOutput::new("an ls-tree object id"))?,
            path: path.get(1..).unwrap_or_default().to_vec(),
        })
    }

    /// Every entry of the output of `git ls-tree -z`, in git's order.
    ///
    /// # Errors
    ///
    /// [`UnreadableOutput`] when a record is cut short, has an unknown mode or type or has a
    /// malformed object id.
    pub fn parse_all(output: &[u8]) -> Result<Vec<Self>, UnreadableOutput> {
        output
            .split_inclusive(|byte| *byte == 0)
            .map(|record| {
                record
                    .strip_suffix(&[0])
                    .ok_or(UnreadableOutput::new("an ls-tree record without its NUL"))
                    .and_then(Self::parse)
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::{EntryMode, TreeEntry};
    use crate::lock::ObjectId;

    const A: &str = "b8b1dadfa8feded6f6d93583dd449585eaafb9f8";
    const B: &str = "0123456789abcdef0123456789abcdef01234567";

    fn id(text: &str) -> ObjectId {
        ObjectId::try_from(text).unwrap()
    }

    #[test]
    fn each_kind_of_entry_is_read_with_its_path_bytes() {
        let output = format!(
            "040000 tree {A}\tdir\0100644 blob {B}\ta b.md\0100755 blob {A}\trun\0\
             120000 blob {B}\tlink\0160000 commit {A}\tsub\0100664 blob {B}\told\0"
        );
        let entries = TreeEntry::parse_all(output.as_bytes()).unwrap();
        let modes: Vec<_> = entries.iter().map(|entry| entry.mode).collect();
        assert_eq!(
            modes,
            [
                EntryMode::Directory,
                EntryMode::File { executable: false },
                EntryMode::File { executable: true },
                EntryMode::Link,
                EntryMode::Submodule,
                EntryMode::File { executable: false },
            ]
        );
        assert_eq!(entries[1].path, b"a b.md");
        assert_eq!(entries[1].id, id(B));
    }

    #[test]
    fn a_path_may_hold_a_newline_and_bytes_that_are_not_utf8() {
        let output = [format!("100644 blob {A}\tx\ny").as_bytes(), b"\xff\0"].concat();
        let entries = TreeEntry::parse_all(&output).unwrap();
        assert_eq!(entries[0].path, b"x\ny\xff");
    }

    #[test]
    fn no_output_is_no_entries() {
        assert_eq!(TreeEntry::parse_all(b""), Ok(vec![]));
    }

    #[test]
    fn a_record_that_does_not_read_is_refused() {
        let cases = [
            format!("100644 blob {A}\tno-nul"),
            format!("100644 blob {A} no-tab\0"),
            format!("100644 {A}\tx\0"),
            format!("100644 tree {A}\tx\0"),
            format!("040000 blob {A}\tx\0"),
            format!("160000 blob {A}\tx\0"),
            format!("777777 blob {A}\tx\0"),
            "100644 blob nothex\tx\0".to_owned(),
            format!("100644 blob {A} extra\tx\0"),
        ];
        for case in cases {
            assert!(TreeEntry::parse_all(case.as_bytes()).is_err(), "{case:?}");
        }
    }
}
