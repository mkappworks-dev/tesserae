//! A Mod's log: `host::log` and its captured stdout and stderr, by line, rate limited.

use std::collections::VecDeque;
use std::io;
use std::pin::Pin;
use std::sync::{Arc, Mutex, MutexGuard};
use std::task::{Context, Poll};
use std::time::Instant;

use bytes::Bytes;
use tokio::io::AsyncWrite;
use wasmtime_wasi::async_trait;
use wasmtime_wasi::cli::{IsTerminal, StdoutStream};
use wasmtime_wasi::p2::{OutputStream, Pollable, StreamResult};

use crate::bindings::tesserae::mods::types;

/// A burst of this many lines, then `LINES_PER_SECOND` more each second.
const BURST_LINES: f64 = 32.0;
const LINES_PER_SECOND: f64 = 32.0;
/// Lines waiting for the host to drain them; past it they count as suppressed.
const PENDING_LINES: usize = 256;
/// A longer line is cut here, and a stdout line with no newline is flushed here.
pub(crate) const MAX_LINE_BYTES: usize = 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

impl From<types::LogLevel> for LogLevel {
    fn from(level: types::LogLevel) -> Self {
        match level {
            types::LogLevel::Trace => LogLevel::Trace,
            types::LogLevel::Debug => LogLevel::Debug,
            types::LogLevel::Info => LogLevel::Info,
            types::LogLevel::Warn => LogLevel::Warn,
            types::LogLevel::Error => LogLevel::Error,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogLine {
    pub level: LogLevel,
    pub message: String,
}

/// What a Mod logged since the last drain, and how many lines the limits discarded.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct LogDrain {
    pub lines: Vec<LogLine>,
    pub suppressed: u64,
}

pub(crate) struct LogBuffer {
    lines: VecDeque<LogLine>,
    suppressed: u64,
    tokens: f64,
    refilled: Instant,
    /// Unfinished stdout and stderr lines.
    partial: [Vec<u8>; 2],
}

/// Reads a Mod's log; it outlives the `ModInstance`, so a fault's last lines can still be read.
#[derive(Clone)]
pub struct LogHandle(Arc<Mutex<LogBuffer>>);

impl LogBuffer {
    pub(crate) fn shared() -> LogHandle {
        LogHandle(Arc::new(Mutex::new(LogBuffer {
            lines: VecDeque::new(),
            suppressed: 0,
            tokens: BURST_LINES,
            refilled: Instant::now(),
            partial: [Vec::new(), Vec::new()],
        })))
    }

    fn push(&mut self, level: LogLevel, message: &str) {
        let now = Instant::now();
        let earned = now.duration_since(self.refilled).as_secs_f64() * LINES_PER_SECOND;
        self.tokens = (self.tokens + earned).min(BURST_LINES);
        self.refilled = now;
        if self.tokens < 1.0 || self.lines.len() >= PENDING_LINES {
            self.suppressed += 1;
            return;
        }
        self.tokens -= 1.0;
        self.lines.push_back(LogLine {
            level,
            message: truncated(message).to_owned(),
        });
    }
}

fn truncated(message: &str) -> &str {
    let mut end = message.len().min(MAX_LINE_BYTES);
    while !message.is_char_boundary(end) {
        end -= 1;
    }
    &message[..end]
}

impl LogHandle {
    fn lock(&self) -> MutexGuard<'_, LogBuffer> {
        // A panic while holding it cannot leave the buffer half-written.
        self.0.lock().unwrap_or_else(|e| e.into_inner())
    }

    pub(crate) fn push(&self, level: LogLevel, message: &str) {
        self.lock().push(level, message);
    }

    pub fn drain(&self) -> LogDrain {
        let mut buffer = self.lock();
        LogDrain {
            lines: buffer.lines.drain(..).collect(),
            suppressed: std::mem::take(&mut buffer.suppressed),
        }
    }
}

/// One of the guest's two output streams, turned into log lines as bytes arrive.
#[derive(Clone)]
pub(crate) struct Capture {
    log: LogHandle,
    level: LogLevel,
    slot: usize,
}

impl Capture {
    pub(crate) fn new(log: &LogHandle, level: LogLevel) -> Capture {
        Capture {
            log: log.clone(),
            level,
            slot: usize::from(level != LogLevel::Info),
        }
    }

    fn feed(&self, bytes: &[u8]) {
        let mut buffer = self.log.lock();
        for &byte in bytes {
            if byte != b'\n' {
                let partial = &mut buffer.partial[self.slot];
                partial.push(byte);
                if partial.len() < MAX_LINE_BYTES {
                    continue;
                }
            }
            let line = std::mem::take(&mut buffer.partial[self.slot]);
            buffer.push(self.level, &String::from_utf8_lossy(&line));
        }
    }
}

impl IsTerminal for Capture {
    fn is_terminal(&self) -> bool {
        false
    }
}

impl StdoutStream for Capture {
    fn async_stream(&self) -> Box<dyn AsyncWrite + Send + Sync> {
        Box::new(self.clone())
    }

    // The default adapter writes on a background task, so a line could land after the call returned.
    fn p2_stream(&self) -> Box<dyn OutputStream> {
        Box::new(self.clone())
    }
}

#[async_trait]
impl Pollable for Capture {
    async fn ready(&mut self) {}
}

impl OutputStream for Capture {
    fn write(&mut self, bytes: Bytes) -> StreamResult<()> {
        self.feed(&bytes);
        Ok(())
    }

    fn flush(&mut self) -> StreamResult<()> {
        Ok(())
    }

    // Always writable: past the limits a line is dropped and counted, never made to block the guest.
    fn check_write(&mut self) -> StreamResult<usize> {
        Ok(MAX_LINE_BYTES)
    }
}

impl AsyncWrite for Capture {
    fn poll_write(
        self: Pin<&mut Self>,
        _: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        self.feed(buf);
        Poll::Ready(Ok(buf.len()))
    }

    fn poll_flush(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }
}
