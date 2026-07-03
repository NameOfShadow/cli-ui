//! Framed log messages — ports `@clack/prompts` `log.{info,warn,error,success,step,message}`.
//!
//! Each call prints to stderr, prefixed with `│` (a connector bar) so consecutive
//! `log.*` calls render as a single connected column matching clack's frame style.

use crate::styles::{paint, DIM, WHITE};

const BAR: &str = "│";
const S_INFO: &str = "●";
const S_SUCCESS: &str = "◆";
const S_STEP: &str = "◇";
const S_WARN: &str = "▲";
const S_ERROR: &str = "■";

/// Print `message` with `symbol` as the first column for the first line,
/// and `│` for any additional lines. Adds one blank `│` connector line above.
pub fn message(msg: impl AsRef<str>, symbol: &str) {
    let s = msg.as_ref();
    eprintln!("{}", paint(DIM, BAR));
    let mut lines = s.lines();
    if let Some(first) = lines.next() {
        eprintln!("{}  {}", symbol, paint(WHITE, first));
    } else {
        eprintln!("{}", symbol);
    }
    for line in lines {
        if line.is_empty() {
            eprintln!("{}", paint(DIM, BAR));
        } else {
            eprintln!("{}  {}", paint(DIM, BAR), line);
        }
    }
}

/// Accent `●` — neutral info.
pub fn info(msg: impl AsRef<str>) {
    message(msg, &paint(super::settings::colors().accent, S_INFO));
}

/// Success `◆` — successful completion.
pub fn success(msg: impl AsRef<str>) {
    message(msg, &paint(super::settings::colors().success, S_SUCCESS));
}

/// Success `◇` — single step completed (between prompts).
pub fn step(msg: impl AsRef<str>) {
    message(msg, &paint(super::settings::colors().success, S_STEP));
}

/// Error palette `▲` — warning.
pub fn warn(msg: impl AsRef<str>) {
    message(msg, &paint(super::settings::colors().error, S_WARN));
}

/// Alias for [`warn`].
pub fn warning(msg: impl AsRef<str>) {
    warn(msg);
}

/// Cancel palette `■` — error.
pub fn error(msg: impl AsRef<str>) {
    message(msg, &paint(super::settings::colors().cancel, S_ERROR));
}
