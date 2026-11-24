//! The pseudo-terminal pair and the session it belongs to.

use std::io;
use std::os::fd::OwnedFd;

use rustix::fs::{Mode, OFlags, open};
use rustix::io::{FdFlags, fcntl_setfd};
use rustix::process::{getpid, getsid, ioctl_tiocsctty, setsid};
use rustix::pty::{OpenptFlags, grantpt, openpt, ptsname, unlockpt};

/// The two ends of a pseudo-terminal: what the driver writes and reads, and what the program gets
/// as its standard input. Both are close-on-exec, so the program inherits only what it is given.
#[derive(Debug)]
pub struct Pair {
    /// The driver's end.
    pub master: OwnedFd,
    /// The program's end.
    pub slave: OwnedFd,
}

/// Opens a pseudo-terminal pair.
///
/// # Errors
///
/// The error of the first call that fails.
pub fn open_pair() -> io::Result<Pair> {
    let master = openpt(OpenptFlags::RDWR | OpenptFlags::NOCTTY)?;
    // Not every platform takes a close-on-exec flag at open; the program must never inherit the
    // driver's end, or the terminal would not hang up when the driver ends.
    fcntl_setfd(&master, FdFlags::CLOEXEC)?;
    grantpt(&master)?;
    unlockpt(&master)?;
    let name = ptsname(&master, Vec::new())?;
    let slave = open(
        name.as_c_str(),
        OFlags::RDWR | OFlags::NOCTTY | OFlags::CLOEXEC,
        Mode::empty(),
    )?;
    Ok(Pair { master, slave })
}

/// Makes this process the leader of a session of its own, if it is not one already (a process
/// that its parent started detached is), and makes `slave` its controlling terminal. Every
/// program the process starts afterwards is in that session, so it has the terminal as its
/// controlling terminal. (A program started before this call would not have it: macOS gives the
/// terminal only to processes started after.)
///
/// Call it only in a process made for the purpose, as `pty-run` is: it changes the session of the
/// whole process.
///
/// # Errors
///
/// The error of `setsid` (the process leads a process group but not a session) or of the
/// controlling-terminal call (the session already has a terminal).
pub fn take_as_controlling_terminal(slave: &OwnedFd) -> io::Result<()> {
    if getsid(None)? != getpid() {
        setsid()?;
    }
    ioctl_tiocsctty(slave)?;
    Ok(())
}
