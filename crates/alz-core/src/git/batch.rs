//! The output of `git cat-file --batch-check` and `--batch` (spec §6 "Git subprocesses").
//! Both answer each requested object in the order it was asked for.

use crate::git::object_kind::{ObjectKind, UnreadableOutput};
use crate::lock::ObjectId;

/// What `--batch-check` says about one requested object.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObjectCheck {
    /// The repository holds the object.
    Present {
        /// The object's id.
        id: ObjectId,
        /// Its type.
        kind: ObjectKind,
        /// Its size in bytes.
        size: u64,
    },
    /// The repository does not hold the object.
    Missing {
        /// The id that was asked for.
        id: ObjectId,
    },
}

/// What `--batch` returns for one requested object.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ObjectContent {
    /// The repository holds the object.
    Present {
        /// The object's id.
        id: ObjectId,
        /// Its type.
        kind: ObjectKind,
        /// Its content.
        bytes: Vec<u8>,
    },
    /// The repository does not hold the object.
    Missing {
        /// The id that was asked for.
        id: ObjectId,
    },
}

/// The header line of one answer: `<object> SP missing`, or `<object> SP <type> SP <size>`.
enum Header {
    Missing(ObjectId),
    Present {
        id: ObjectId,
        kind: ObjectKind,
        size: u64,
    },
}

impl Header {
    fn parse(line: &[u8]) -> Result<Self, UnreadableOutput> {
        let line = std::str::from_utf8(line)
            .map_err(|_cause| UnreadableOutput::new("a cat-file header that is not ASCII"))?;
        let id = |text: &str| {
            ObjectId::try_from(text).map_err(|_cause| UnreadableOutput::new("a cat-file object id"))
        };
        let mut words = line.split(' ');
        match (words.next(), words.next(), words.next(), words.next()) {
            (Some(object), Some("missing"), None, None) => id(object).map(Self::Missing),
            (Some(object), Some(kind), Some(size), None) => Ok(Self::Present {
                id: id(object)?,
                kind: ObjectKind::from_word(kind)
                    .ok_or(UnreadableOutput::new("a cat-file object type"))?,
                size: size
                    .parse()
                    .map_err(|_cause| UnreadableOutput::new("a cat-file size"))?,
            }),
            _ => Err(UnreadableOutput::new("a cat-file header")),
        }
    }
}

impl ObjectCheck {
    /// Every answer of `git cat-file --batch-check`, one per line.
    ///
    /// # Errors
    ///
    /// [`UnreadableOutput`] when a line is cut short or does not read.
    pub fn parse_all(output: &[u8]) -> Result<Vec<Self>, UnreadableOutput> {
        output
            .split_inclusive(|byte| *byte == b'\n')
            .map(|line| {
                line.strip_suffix(b"\n")
                    .ok_or(UnreadableOutput::new("a cat-file line without its newline"))
                    .and_then(Header::parse)
                    .map(|header| match header {
                        Header::Missing(id) => Self::Missing { id },
                        Header::Present { id, kind, size } => Self::Present { id, kind, size },
                    })
            })
            .collect()
    }
}

impl ObjectContent {
    /// Every answer of `git cat-file --batch`: a header line, then the content, then a newline.
    ///
    /// # Errors
    ///
    /// [`UnreadableOutput`] when a header does not read or the output ends inside a content.
    pub fn parse_all(output: &[u8]) -> Result<Vec<Self>, UnreadableOutput> {
        let mut rest = output;
        std::iter::from_fn(|| {
            (!rest.is_empty()).then(|| {
                let (answer, tail) = Self::parse_one(rest)?;
                rest = tail;
                Ok(answer)
            })
        })
        .collect()
    }

    /// The first answer in `output` and what follows it.
    fn parse_one(output: &[u8]) -> Result<(Self, &[u8]), UnreadableOutput> {
        let newline =
            output
                .iter()
                .position(|byte| *byte == b'\n')
                .ok_or(UnreadableOutput::new(
                    "a cat-file header without its newline",
                ))?;
        let (line, after) = output
            .split_at_checked(newline)
            .ok_or(UnreadableOutput::new("a cat-file header"))?;
        let after = after.get(1..).unwrap_or_default();
        match Header::parse(line)? {
            Header::Missing(id) => Ok((Self::Missing { id }, after)),
            Header::Present { id, kind, size } => {
                let size = usize::try_from(size)
                    .map_err(|_cause| UnreadableOutput::new("a cat-file size"))?;
                let (bytes, tail) = after.split_at_checked(size).ok_or(UnreadableOutput::new(
                    "a cat-file content that is cut short",
                ))?;
                let tail = tail.strip_prefix(b"\n").ok_or(UnreadableOutput::new(
                    "a cat-file content without its newline",
                ))?;
                Ok((
                    Self::Present {
                        id,
                        kind,
                        bytes: bytes.to_vec(),
                    },
                    tail,
                ))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{ObjectCheck, ObjectContent};
    use crate::git::object_kind::ObjectKind;
    use crate::lock::ObjectId;

    const A: &str = "b8b1dadfa8feded6f6d93583dd449585eaafb9f8";
    const B: &str = "0123456789abcdef0123456789abcdef01234567";

    fn id(text: &str) -> ObjectId {
        ObjectId::try_from(text).unwrap()
    }

    #[test]
    fn a_batch_check_names_present_and_missing_objects_in_order() {
        let output = format!("{A} blob 12\n{B} missing\n{A} tree 0\n");
        assert_eq!(
            ObjectCheck::parse_all(output.as_bytes()),
            Ok(vec![
                ObjectCheck::Present {
                    id: id(A),
                    kind: ObjectKind::Blob,
                    size: 12
                },
                ObjectCheck::Missing { id: id(B) },
                ObjectCheck::Present {
                    id: id(A),
                    kind: ObjectKind::Tree,
                    size: 0
                },
            ])
        );
    }

    #[test]
    fn a_batch_check_that_does_not_read_is_refused() {
        for case in [
            format!("{A} blob 12"),
            format!("{A} blobby 12\n"),
            format!("{A} blob twelve\n"),
            format!("{A} blob\n"),
            "nothex blob 1\n".to_owned(),
            format!("{A} missing extra\n"),
        ] {
            assert!(ObjectCheck::parse_all(case.as_bytes()).is_err(), "{case:?}");
        }
    }

    #[test]
    fn a_batch_returns_each_content_even_with_newlines_and_binary_bytes() {
        let output = [
            format!("{A} blob 5\n").as_bytes(),
            b"ab\n\0\xff\n",
            format!("{B} missing\n{A} blob 0\n\n").as_bytes(),
        ]
        .concat();
        assert_eq!(
            ObjectContent::parse_all(&output),
            Ok(vec![
                ObjectContent::Present {
                    id: id(A),
                    kind: ObjectKind::Blob,
                    bytes: b"ab\n\0\xff".to_vec()
                },
                ObjectContent::Missing { id: id(B) },
                ObjectContent::Present {
                    id: id(A),
                    kind: ObjectKind::Blob,
                    bytes: vec![]
                },
            ])
        );
    }

    #[test]
    fn a_batch_cut_short_is_refused() {
        for case in [
            format!("{A} blob 5\nab"),
            format!("{A} blob 2\nabX"),
            format!("{A} blob 2"),
            format!("{A} blob 2\nab"),
        ] {
            assert!(
                ObjectContent::parse_all(case.as_bytes()).is_err(),
                "{case:?}"
            );
        }
    }
}
