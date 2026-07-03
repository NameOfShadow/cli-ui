//! Download progress tracking.
//!
//! [`Progress`] tracks concurrent downloads and prints each result as it
//! completes. URLs are truncated to fit the terminal width automatically.
//!
//! # Example
//! ```rust,no_run
//! use cli_ui::Progress;
//!
//! let pb = Progress::new(13);
//! pb.ok("style", "remote", "https://example.com/style.css", "./css/style.css", 6041);
//! pb.fail("script", "https://example.com/app.js", "connection refused");
//! pb.finish();
//! ```

use crate::styles::*;
use crate::term;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

/// Tracks download progress and prints styled result lines.
pub struct Progress {
    total: usize,
    current: Arc<AtomicUsize>,
}

impl Progress {
    /// Create a new progress tracker for `total` items.
    pub fn new(total: usize) -> Self {
        Self {
            total,
            current: Arc::new(AtomicUsize::new(0)),
        }
    }

    /// Print a successful download line.
    ///
    /// ```text
    ///   ✓  [style ][remote]  https://…  →  ./css/file.css  [1.37 KB]
    /// ```
    pub fn ok(&self, kind: &str, source: &str, url: &str, local: &str, bytes: usize) {
        let n = self.current.fetch_add(1, Ordering::Relaxed) + 1;
        let total = self.total;
        let size = format_bytes(bytes);
        let term_w = term::width();

        // counter prefix: (5/13)
        let counter = paint(DIM, &format!("({n}/{total})"));
        let kind_s = paint(DIM, &format!("[{:<6}]", kind));
        let src_s = paint(DIM, &format!("[{:<6}]", source));
        let arrow = paint(DIM, "→");
        let local_s = paint(CYAN, local);
        let size_s = paint(DIM, &format!("[{size}]"));

        // truncate URL to fit terminal
        let prefix_len = 3 + 8 + 8 + 4 + local.len() + size.len() + 12;
        let url_budget = term_w.saturating_sub(prefix_len).max(20);
        let url_display = truncate_url(url, url_budget);
        let url_s = paint(DIM, &url_display);

        eprintln!(
            "   {}  {}  {}  {}  {}  {}  {}",
            counter, kind_s, src_s, url_s, arrow, local_s, size_s
        );
    }

    /// Print a failed download line, aligned with `ok()` lines.
    ///
    /// ```text
    ///   (4/5)  [file  ][error ]  https://…  (connection timeout)
    /// ```
    pub fn fail(&self, kind: &str, url: &str, error: &str) {
        let n = self.current.fetch_add(1, Ordering::Relaxed) + 1;
        let total = self.total;
        let counter = paint(DIM, &format!("({n}/{total})"));
        let term_w = term::width();
        let prefix_len = 3 + 8 + 8 + 4 + error.len() + 6;
        let url_budget = term_w.saturating_sub(prefix_len).max(20);
        let url_s = paint(DIM, &truncate_url(url, url_budget));
        eprintln!(
            "   {}  {}  {}  {}  {}",
            counter,
            paint(ERR, &format!("[{:<6}]", kind)),
            paint(ERR, "[error ]"),
            url_s,
            paint(ERR, &format!("({error})")),
        );
    }

    /// Print a blank line after the download block.
    pub fn finish(&self) {
        eprintln!();
    }
}

/// Print a CSS sub-step line.
///
/// ```text
///      └─  https://fonts.gstatic.com/…  →  ./fonts/file.ttf  [317 KB]
/// ```
pub fn substep(url: &str, local: &str) {
    eprintln!(
        "      {}  {}  {}  {}",
        paint(DIM, BRANCH),
        paint(DIM, url),
        paint(DIM, "→"),
        paint(CYAN, local),
    );
}

/// Format a byte count as a human-readable string.
///
/// # Examples
/// ```
/// use cli_ui::progress::format_bytes;
/// assert_eq!(format_bytes(512),       "512 B");
/// assert_eq!(format_bytes(1500),      "1.46 KB");
/// assert_eq!(format_bytes(1_500_000), "1.43 MB");
/// ```
pub fn format_bytes(bytes: usize) -> String {
    if bytes >= 1_048_576 {
        format!("{:.2} MB", bytes as f64 / 1_048_576.0)
    } else if bytes >= 1_024 {
        format!("{:.2} KB", bytes as f64 / 1_024.0)
    } else {
        format!("{bytes} B")
    }
}

/// Truncate a URL to `max_len` characters, keeping the end visible.
///
/// `"https://fonts.gstatic.com/s/inter/v20/UcCO…ttf"` → `"…/UcCO…ttf"`
fn truncate_url(url: &str, max_len: usize) -> String {
    if url.len() <= max_len {
        return url.to_string();
    }
    let keep = max_len.saturating_sub(1);
    format!("…{}", &url[url.len() - keep..])
}
