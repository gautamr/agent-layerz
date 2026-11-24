//! The real path of an absolute path, found one component at a time (spec §6 "The layer cache"
//! and "Layers in or around an account folder", §9.1; TASK-52). A sans-IO state machine: it asks
//! for one `lstat` or `readlink` at a time and is given the answer, and never touches the file
//! system itself. alz-io answers the requests; account resolution (§7.13) drives the same walk.
//!
//! Before the walk asks about a name, it checks the path of that name against the account
//! folders. A name inside an account folder, or inside a folder that `CLAUDE_CONFIG_DIR` or
//! `CODEX_HOME` names, is never asked about, whatever the answers before it were: a link planted
//! in a repository that leads into `~/.claude-work` ends the walk at the link, before the first
//! `lstat` there. The real path that is found is checked as a whole too: a path that is `$HOME`
//! or above it, or that holds such a folder, is refused, so no caller opens a file there. The
//! refusal names the reason and never holds the path.
//!
//! The walk has to look at the names above `$HOME` (`/Users` on the way to `/Users/u/dev`),
//! because a path below `$HOME` is reached through them. That is a lookup of the name, which
//! reveals nothing of an account; what spec §9.1 forbids is a lookup *inside* an account folder,
//! and an open of a directory that touches one.

use thiserror::Error;

use super::AbsolutePath;
use crate::account::{AccountFolders, Relation};

/// Links followed before the walk gives up: the limit Linux itself uses.
const MAX_LINKS: u8 = 40;

/// One name of a path: not empty, not `.` or `..`, with no `/` and no NUL.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Name(String);

impl Name {
    /// The name as text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// One part of a path as written.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Part {
    Current,
    Parent,
    Name(Name),
}

/// The parts of `/`-separated text that holds no NUL: empty parts are left out.
fn parts(text: &str) -> impl DoubleEndedIterator<Item = Part> + '_ {
    text.split('/')
        .filter(|part| !part.is_empty())
        .map(|part| match part {
            "." => Part::Current,
            ".." => Part::Parent,
            name => Part::Name(Name(name.to_owned())),
        })
}

/// What a link points at, as `readlink` returns it: not empty, with no NUL.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinkTarget(String);

/// Text that cannot be a link target. The error holds no input.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
#[error("not a link target: it must not be empty or hold a NUL")]
pub struct LinkTargetError;

impl TryFrom<&str> for LinkTarget {
    type Error = LinkTargetError;

    fn try_from(text: &str) -> Result<Self, Self::Error> {
        if text.is_empty() || text.contains('\0') {
            Err(LinkTargetError)
        } else {
            Ok(Self(text.to_owned()))
        }
    }
}

/// What `lstat` found at a name. Nothing is followed: a link is a link.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Entry {
    /// Nothing is there.
    Missing,
    /// A directory.
    Directory,
    /// A symbolic link.
    Link,
    /// Anything else: a file, a pipe, a device.
    Other,
}

/// Where a walk ended without a real path.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum Stop {
    /// The path touches an account folder or the home folder (§9.1); the reason is the relation.
    #[error("{0}")]
    Refused(Relation),
    /// A walk that locates a directory (see [`RealPathWalk::locate`]) reached a name inside an
    /// account folder or a configured folder, which it never asks about: the relation, and the
    /// real path of the folder where it stopped.
    #[error("{relation}")]
    Inside {
        /// How the folder relates to the account folders; always [`Relation::is_inside`].
        relation: Relation,
        /// The real path of the first name inside, which is the account or configured folder.
        at: AbsolutePath,
    },
    /// More than 40 links were followed.
    #[error("too many links")]
    TooManyLinks,
    /// A name is missing and the path goes on with `..`, which a missing directory cannot resolve.
    #[error("a directory on the way does not exist")]
    NotFound,
    /// A name that is not a directory has more of the path below it.
    #[error("a name on the way is not a directory")]
    NotADirectory,
}

/// A real path that the walk found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Resolved {
    /// Every name exists, and no link is left in the path.
    Existing(AbsolutePath),
    /// The path exists up to `existing`; each name in `below` is missing, in order. Nothing was
    /// asked about them, so none of them is a link or a folder anyone planted.
    Missing {
        /// The real path of the deepest folder that exists.
        existing: AbsolutePath,
        /// The names that do not exist yet, outermost first.
        below: Vec<Name>,
    },
}

/// What the walk needs next, or how it ended.
#[derive(Debug)]
#[must_use = "a step is a request to answer, or the end of the walk"]
pub enum Step<'f> {
    /// Ask what `lstat` finds at the path.
    Lstat(Lstat<'f>),
    /// Ask what `readlink` returns for the path.
    Readlink(Readlink<'f>),
    /// The real path.
    Done(Resolved),
    /// No real path.
    Stopped(Stop),
}

/// The walk, waiting for an `lstat`. The only way to go on is to answer it.
#[derive(Debug)]
pub struct Lstat<'f> {
    walk: RealPathWalk<'f>,
    name: Name,
    prefix: AbsolutePath,
}

/// The walk, waiting for a `readlink`. The only way to go on is to answer it.
#[derive(Debug)]
pub struct Readlink<'f> {
    walk: RealPathWalk<'f>,
    prefix: AbsolutePath,
}

/// What the walk is for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Intent {
    /// The caller will open the real path: whatever touches an account folder is refused.
    Open,
    /// The caller only wants to know where the path is: a folder that holds a configured folder
    /// is a place like any other, and the stop inside an account folder says where it was.
    Locate,
}

/// A walk in progress: the real path of what is resolved, and the parts that are left.
#[derive(Debug)]
pub struct RealPathWalk<'f> {
    folders: &'f AccountFolders,
    intent: Intent,
    /// What is resolved. Every name in it is a directory, not a link.
    real: Vec<Name>,
    /// What is left, the next part last.
    pending: Vec<Part>,
    links: u8,
}

/// The absolute path of names: `/` for none.
fn path_of<'n>(names: impl IntoIterator<Item = &'n Name>) -> AbsolutePath {
    let names = names.into_iter().map(Name::as_str).collect::<Vec<_>>();
    AbsolutePath::from_checked(format!("/{}", names.join("/")))
}

impl<'f> RealPathWalk<'f> {
    /// Starts at `/` and walks `path`. Nothing is asked about `/` itself.
    pub fn start(path: &AbsolutePath, folders: &'f AccountFolders) -> Step<'f> {
        Self::begin(path, folders, Intent::Open)
    }

    /// Like [`start`](Self::start), for a caller that only wants to know where `path` is, such as
    /// the working directory (spec §7.13 "Paths"). A path that holds a configured folder is a
    /// real path, not a refusal, because the repository there stays a root; a name inside an
    /// account folder or a configured folder is still never asked about, and the walk stops with
    /// [`Stop::Inside`], which names the folder. `$HOME` itself or above it is [`Stop::Refused`]
    /// with [`Relation::Home`].
    pub fn locate(path: &AbsolutePath, folders: &'f AccountFolders) -> Step<'f> {
        Self::begin(path, folders, Intent::Locate)
    }

    fn begin(path: &AbsolutePath, folders: &'f AccountFolders, intent: Intent) -> Step<'f> {
        let pending = path
            .components()
            .flat_map(parts)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect();
        Self {
            folders,
            intent,
            real: Vec::new(),
            pending,
            links: 0,
        }
        .advance()
    }

    /// Takes the next part. `.` and `..` need no question, because everything in `real` is a
    /// real directory; the next name is checked against the account folders and asked about.
    fn advance(mut self) -> Step<'f> {
        // The loop and the mutation stay inside this function: each pass consumes one part.
        while let Some(part) = self.pending.pop() {
            match part {
                Part::Current => {}
                Part::Parent => {
                    self.real.pop();
                }
                Part::Name(name) => return self.ask(name),
            }
        }
        self.finish()
    }

    fn ask(self, name: Name) -> Step<'f> {
        let prefix = path_of(self.real.iter().chain([&name]));
        match self.folders.relation(&prefix) {
            Some(relation) if relation.is_inside() => Step::Stopped(match self.intent {
                Intent::Open => Stop::Refused(relation),
                Intent::Locate => Stop::Inside {
                    relation,
                    at: prefix,
                },
            }),
            Some(_) | None => Step::Lstat(Lstat {
                walk: self,
                name,
                prefix,
            }),
        }
    }

    /// The whole real path is found: refused when it touches an account folder in any way.
    fn conclude(&self, whole: &AbsolutePath, resolved: Resolved) -> Step<'f> {
        match (self.folders.relation(whole), self.intent) {
            (None, _) | (Some(Relation::Holds), Intent::Locate) => Step::Done(resolved),
            (Some(relation), Intent::Locate) if relation.is_inside() => {
                Step::Stopped(Stop::Inside {
                    relation,
                    at: whole.clone(),
                })
            }
            (Some(relation), Intent::Open | Intent::Locate) => {
                Step::Stopped(Stop::Refused(relation))
            }
        }
    }

    fn finish(self) -> Step<'f> {
        let path = path_of(self.real.iter());
        self.conclude(&path.clone(), Resolved::Existing(path))
    }
}

impl<'f> Lstat<'f> {
    /// The path to `lstat`: the real path found so far and the next name. It is never inside an
    /// account folder.
    #[must_use]
    pub const fn path(&self) -> &AbsolutePath {
        &self.prefix
    }

    /// Goes on with what `lstat` found.
    pub fn answer(self, entry: Entry) -> Step<'f> {
        let Self {
            mut walk,
            name,
            prefix,
        } = self;
        match entry {
            Entry::Directory => {
                walk.real.push(name);
                walk.advance()
            }
            Entry::Link if walk.links >= MAX_LINKS => Step::Stopped(Stop::TooManyLinks),
            Entry::Link => Step::Readlink(Readlink { walk, prefix }),
            Entry::Other => {
                if walk.pending.iter().all(|part| *part == Part::Current) {
                    walk.real.push(name);
                    walk.finish()
                } else {
                    Step::Stopped(Stop::NotADirectory)
                }
            }
            Entry::Missing => {
                let rest = walk.pending.iter().rev().collect::<Vec<_>>();
                if rest.contains(&&Part::Parent) {
                    Step::Stopped(Stop::NotFound)
                } else {
                    let below: Vec<Name> = std::iter::once(name)
                        .chain(rest.into_iter().filter_map(|part| match part {
                            Part::Name(name) => Some(name.clone()),
                            Part::Current | Part::Parent => None,
                        }))
                        .collect();
                    let existing = path_of(walk.real.iter());
                    let whole = path_of(walk.real.iter().chain(below.iter()));
                    walk.conclude(&whole, Resolved::Missing { existing, below })
                }
            }
        }
    }
}

impl<'f> Readlink<'f> {
    /// The path to `readlink`: a link that `lstat` found, never inside an account folder.
    #[must_use]
    pub const fn path(&self) -> &AbsolutePath {
        &self.prefix
    }

    /// Goes on with where the link points: an absolute target restarts the walk from `/`, a
    /// relative one continues from the folder that holds the link.
    pub fn answer(self, target: &LinkTarget) -> Step<'f> {
        let mut walk = self.walk;
        walk.links = walk.links.saturating_add(1);
        if target.0.starts_with('/') {
            walk.real.clear();
        }
        walk.pending.extend(parts(&target.0).rev());
        walk.advance()
    }
}

#[cfg(test)]
mod tests;
