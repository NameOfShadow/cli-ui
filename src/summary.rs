//! Summary block renderer.
//!
//! Used via the [`summary!`](macro@crate::summary) macro at the end of a command run.
//!
//! # Example
//! ```rust,no_run
//! use cli_ui::summary;
//! use cli_ui::styles::{paint, CYAN, YELLOW, DIM, OK};
//!
//! summary! {
//!     done: "All assets localized",
//!     "input"  => paint(CYAN, "index.html"),
//!     "output" => paint(CYAN, "dist/index.html"),
//!     section,
//!     "assets" => format!("{} remote · {} local", paint(OK, "19"), paint(OK, "0")),
//!     "size"   => paint(YELLOW, "1.64 MB"),
//!     "time"   => paint(DIM, "13034ms"),
//! }
//! ```
//!
//! Output:
//! ```text
//!  ────────────────────────────────────────────────
//!
//!     DONE   All assets localized
//!
//!    input   index.html
//!   output   dist/index.html
//!
//!   assets   19 remote · 0 local
//!     size   1.64 MB
//!     time   13034ms
//!
//!  ────────────────────────────────────────────────
//! ```

use crate::styles::*;
use crate::term;

/// Builder for the end-of-run summary block.
///
/// Construct via the [`summary!`](crate::summary!) macro rather than directly.
pub struct Summary {
    sections: Vec<SummarySection>,
}

struct SummarySection {
    entries: Vec<SummaryEntry>,
}

enum SummaryEntry {
    /// Green ` DONE ` badge with message.
    Done(String),
    /// Yellow ` WARN ` badge with message.
    Warn(String),
    /// Key/value stat — keys are right-aligned per section.
    Stat { key: String, val: String },
    /// Blank line separator.
    Blank,
}

impl Summary {
    /// Create an empty summary builder.
    pub fn new() -> Self {
        Self {
            sections: vec![SummarySection {
                entries: Vec::new(),
            }],
        }
    }

    /// Add a green `DONE` status line.
    pub fn done(mut self, msg: &str) -> Self {
        self.cur().entries.push(SummaryEntry::Done(msg.to_string()));
        self
    }

    /// Add a yellow `WARN` status line (used when there are errors).
    pub fn warn(mut self, msg: &str) -> Self {
        self.cur().entries.push(SummaryEntry::Warn(msg.to_string()));
        self
    }

    /// Add a key/value stat line.
    ///
    /// Keys are right-aligned to the longest key in the same section.
    /// Values are printed as-is (may contain ANSI codes from [`paint`]).
    pub fn stat(mut self, key: &str, val: &str) -> Self {
        self.cur().entries.push(SummaryEntry::Stat {
            key: key.to_string(),
            val: val.to_string(),
        });
        self
    }

    /// Add a blank line within the current section.
    pub fn blank(mut self) -> Self {
        self.cur().entries.push(SummaryEntry::Blank);
        self
    }

    /// Start a new alignment section.
    ///
    /// Keys in different sections are aligned independently, so a long key
    /// in the stats block doesn't push the files block out of alignment.
    pub fn section(mut self) -> Self {
        self.sections.push(SummarySection {
            entries: Vec::new(),
        });
        self
    }

    /// Render the summary to stderr.
    pub fn print(self) {
        let w = term::width().min(80);
        let rule = paint(DIM, &"─".repeat(w));

        eprintln!();
        eprintln!(" {rule}");
        eprintln!();

        for section in &self.sections {
            // right-align keys: find max width per section
            let max_key = section
                .entries
                .iter()
                .filter_map(|e| match e {
                    SummaryEntry::Stat { key, .. } => Some(key.len()),
                    _ => None,
                })
                .max()
                .unwrap_or(0);

            for entry in &section.entries {
                match entry {
                    SummaryEntry::Done(msg) => {
                        let badge = paint(
                            anstyle::Style::new()
                                .bg_color(Some(anstyle::Color::Ansi(anstyle::AnsiColor::Green)))
                                .fg_color(Some(anstyle::Color::Ansi(
                                    anstyle::AnsiColor::BrightWhite,
                                )))
                                .bold(),
                            " DONE ",
                        );
                        eprintln!("   {}  {}", badge, paint(OK, msg));
                    }
                    SummaryEntry::Warn(msg) => {
                        let badge = paint(
                            anstyle::Style::new()
                                .bg_color(Some(anstyle::Color::Ansi(anstyle::AnsiColor::Yellow)))
                                .fg_color(Some(anstyle::Color::Ansi(anstyle::AnsiColor::Black)))
                                .bold(),
                            " WARN ",
                        );
                        eprintln!("   {}  {}", badge, paint(YELLOW, msg));
                    }
                    SummaryEntry::Stat { key, val } => {
                        let pad = " ".repeat(max_key - key.len());
                        eprintln!("   {}{}   {}", pad, paint(DIM, key), val);
                    }
                    SummaryEntry::Blank => eprintln!(),
                }
            }
        }

        eprintln!();
        eprintln!(" {rule}");
        eprintln!();
    }

    fn cur(&mut self) -> &mut SummarySection {
        self.sections.last_mut().unwrap()
    }
}

impl Default for Summary {
    fn default() -> Self {
        Self::new()
    }
}
