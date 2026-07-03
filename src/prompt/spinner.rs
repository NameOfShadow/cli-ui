#![allow(missing_docs)]
//! Animated spinner — ports `@clack/prompts` `spinner()`.
//!
//! Runs an animation thread on stderr until `stop`, `cancel`, or `error` is
//! called. Supports a live message update via [`Spinner::message`].
//!
//! # Example
//! ```rust,no_run
//! use cli_ui::prompt::spinner;
//!
//! let s = spinner();
//! s.start("Installing dependencies");
//! std::thread::sleep(std::time::Duration::from_millis(800));
//! s.message("Resolving graph");
//! std::thread::sleep(std::time::Duration::from_millis(800));
//! s.stop("Installed 42 packages");
//! ```

use crate::styles::{paint, DIM, WHITE};
use std::io::Write;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

const FRAMES: &[&str] = &["◒", "◐", "◓", "◑"];
const FRAME_DELAY: Duration = Duration::from_millis(80);

const STEP_SUBMIT: &str = "◇";
const STEP_CANCEL: &str = "■";
const STEP_ERROR: &str = "▲";

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Indicator {
    Dots,
    Timer,
}

struct Inner {
    message: Mutex<String>,
    running: AtomicBool,
    cancelled: AtomicBool,
    indicator: AtomicU8, // 0 = Dots, 1 = Timer
}

/// Handle returned from [`spinner()`]. All methods are safe to call from any thread.
pub struct Spinner {
    inner: Arc<Inner>,
    handle: Mutex<Option<JoinHandle<()>>>,
    started: AtomicBool,
}

/// Create a new spinner. Call [`Spinner::start`] to begin animating.
pub fn spinner() -> Spinner {
    Spinner {
        inner: Arc::new(Inner {
            message: Mutex::new(String::new()),
            running: AtomicBool::new(false),
            cancelled: AtomicBool::new(false),
            indicator: AtomicU8::new(0),
        }),
        handle: Mutex::new(None),
        started: AtomicBool::new(false),
    }
}

impl Spinner {
    /// Switch to elapsed-time indicator (`[1m 2s]`) instead of trailing dots.
    pub fn with_timer(self) -> Self {
        self.inner.indicator.store(1, Ordering::Relaxed);
        self
    }

    pub fn is_cancelled(&self) -> bool {
        self.inner.cancelled.load(Ordering::Relaxed)
    }

    /// Start animation. Becomes a no-op if already started.
    pub fn start(&self, msg: impl Into<String>) {
        if self.started.swap(true, Ordering::SeqCst) {
            return;
        }
        *self.inner.message.lock().unwrap() = trim_dots(msg.into());
        self.inner.running.store(true, Ordering::SeqCst);
        let inner = self.inner.clone();
        let handle = thread::spawn(move || run_loop(inner));
        *self.handle.lock().unwrap() = Some(handle);
    }

    /// Update the message displayed next to the spinner.
    pub fn message(&self, msg: impl Into<String>) {
        *self.inner.message.lock().unwrap() = trim_dots(msg.into());
    }

    /// Stop animation, replace spinner with green `◇` success glyph + message.
    pub fn stop(&self, msg: impl Into<String>) {
        self.terminate(msg.into(), Status::Submit);
    }

    /// Stop animation as cancelled — red `■` glyph.
    pub fn cancel(&self, msg: impl Into<String>) {
        self.inner.cancelled.store(true, Ordering::SeqCst);
        self.terminate(msg.into(), Status::Cancel);
    }

    /// Stop animation as error — red `▲` glyph.
    pub fn error(&self, msg: impl Into<String>) {
        self.terminate(msg.into(), Status::Error);
    }

    /// Stop the animation without printing a final line.
    pub fn clear(&self) {
        self.inner.running.store(false, Ordering::SeqCst);
        if let Some(h) = self.handle.lock().unwrap().take() {
            let _ = h.join();
        }
        let mut out = std::io::stderr();
        let _ = write!(out, "\r\x1b[2K");
        let _ = out.flush();
    }

    fn terminate(&self, msg: String, status: Status) {
        self.inner.running.store(false, Ordering::SeqCst);
        if let Some(h) = self.handle.lock().unwrap().take() {
            let _ = h.join();
        }
        let final_msg = if msg.is_empty() {
            self.inner.message.lock().unwrap().clone()
        } else {
            msg
        };
        let c = super::settings::colors();
        let glyph = match status {
            Status::Submit => paint(c.success, STEP_SUBMIT),
            Status::Cancel => paint(c.cancel, STEP_CANCEL),
            Status::Error => paint(c.error, STEP_ERROR),
        };
        let mut out = std::io::stderr();
        let _ = write!(out, "\r\x1b[2K{}  {}\n", glyph, paint(WHITE, &final_msg));
        let _ = out.flush();
    }
}

impl Drop for Spinner {
    fn drop(&mut self) {
        if self.inner.running.load(Ordering::Relaxed) {
            self.clear();
        }
    }
}

enum Status {
    Submit,
    Cancel,
    Error,
}

fn trim_dots(mut s: String) -> String {
    while s.ends_with('.') {
        s.pop();
    }
    s
}

fn run_loop(inner: Arc<Inner>) {
    let mut idx = 0usize;
    let mut dot_phase = 0u8;
    let origin = Instant::now();
    let mut out = std::io::stderr();
    while inner.running.load(Ordering::Relaxed) {
        let frame = FRAMES[idx % FRAMES.len()];
        let msg = inner.message.lock().unwrap().clone();
        let indicator = inner.indicator.load(Ordering::Relaxed);
        let suffix = if indicator == 1 {
            format!(" {}", paint(DIM, &format_timer(origin.elapsed())))
        } else {
            let n = (dot_phase / 8) as usize;
            ".".repeat(n.min(3))
        };
        let _ = write!(
            out,
            "\r\x1b[2K{}  {}{}",
            paint(super::settings::colors().accent, frame),
            paint(WHITE, &msg),
            suffix
        );
        let _ = out.flush();
        idx = idx.wrapping_add(1);
        dot_phase = dot_phase.wrapping_add(1);
        thread::sleep(FRAME_DELAY);
    }
}

fn format_timer(d: Duration) -> String {
    let secs = d.as_secs();
    let m = secs / 60;
    let s = secs % 60;
    if m > 0 {
        format!("[{m}m {s}s]")
    } else {
        format!("[{s}s]")
    }
}
