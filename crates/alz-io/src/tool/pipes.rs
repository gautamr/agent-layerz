//! Reading a program's output within a cap, on a thread of its own, so a full pipe never stops
//! the program and a program that writes without end cannot fill the memory.

use std::io::{self, Read};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::thread;
use std::time::Instant;

use super::ToolError;

/// What a read of a stream takes at a time.
const CHUNK: usize = 8 * 1024;

/// What a stream's thread hands over.
pub(super) enum Piece {
    /// The next bytes, within the cap.
    Bytes(Vec<u8>),
    /// The stream failed.
    Failed(io::Error),
}

/// A stream as far as it was read.
pub(super) struct Gathered {
    pub bytes: Vec<u8>,
}

/// Reads `pipe` to its end. The bytes within `cap` go to `pieces`, the rest is read and dropped.
/// Reading goes on after the caller stopped listening, so a writer that outlives the call is not
/// cut off.
fn pump(mut pipe: impl Read, cap: usize, pieces: &Sender<Piece>) {
    let mut room = cap;
    let mut buffer = vec![0_u8; CHUNK];
    loop {
        match pipe.read(&mut buffer) {
            Ok(0) => return,
            Ok(count) => {
                let chunk = buffer.get(..count).unwrap_or_default();
                let kept = chunk.len().min(room);
                room -= kept;
                if kept > 0 {
                    pieces
                        .send(Piece::Bytes(chunk.get(..kept).unwrap_or_default().to_vec()))
                        .unwrap_or_default();
                }
            }
            Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
            Err(error) => {
                pieces.send(Piece::Failed(error)).unwrap_or_default();
                return;
            }
        }
    }
}

/// Starts the thread that reads `pipe`.
pub(super) fn stream(pipe: impl Read + Send + 'static, cap: usize) -> io::Result<Receiver<Piece>> {
    let (sender, receiver) = mpsc::channel();
    thread::Builder::new().spawn(move || pump(pipe, cap, &sender))?;
    Ok(receiver)
}

/// Everything that arrives on `pieces` until the stream ends or `until` passes. A stream that has
/// not ended by then is taken as it is.
pub(super) fn gather(pieces: &Receiver<Piece>, until: Instant) -> Result<Gathered, ToolError> {
    let mut bytes = Vec::new();
    loop {
        match pieces.recv_timeout(until.saturating_duration_since(Instant::now())) {
            Ok(Piece::Bytes(more)) => bytes.extend_from_slice(&more),
            Ok(Piece::Failed(error)) => return Err(ToolError::Io(error)),
            Err(RecvTimeoutError::Timeout | RecvTimeoutError::Disconnected) => {
                return Ok(Gathered { bytes });
            }
        }
    }
}
