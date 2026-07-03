//! Terminal rendering helpers — interactive feature only.

use crossterm::{execute, QueueableCommand};
use std::io::{Stderr, Write};

pub fn clear_lines(out: &mut Stderr, n: usize) -> std::io::Result<()> {
    use crossterm::cursor::{MoveToColumn, MoveUp};
    use crossterm::terminal::{Clear, ClearType};
    // After a draw the cursor sits on the blank line below the last printed
    // row, so we step up `n` times — once for every line the prompt drew —
    // and then wipe everything from the cursor to the end of the screen.
    // FromCursorDown is the safest primitive here: it doesn't depend on
    // per-row clears succeeding in lockstep with row counts, which is what
    // bit us when ANSI styling or wide-char input shifted things by one.
    if n > 0 {
        out.queue(MoveUp(n as u16))?;
    }
    out.queue(MoveToColumn(0))?;
    out.queue(Clear(ClearType::FromCursorDown))?;
    out.flush()
}

pub fn hide_cursor(out: &mut Stderr) -> std::io::Result<()> {
    execute!(out, crossterm::cursor::Hide)
}

pub fn show_cursor(out: &mut Stderr) -> std::io::Result<()> {
    execute!(out, crossterm::cursor::Show)
}

/// Call on Interrupted / any early exit from raw mode: erases the prompt's
/// `drawn` rows so the cancel banner docks against the frame above, then
/// restores cursor visibility and leaves raw mode. Does NOT print any
/// trailing newline — the surrounding `cancel()` helper provides that.
pub fn cleanup(out: &mut Stderr, drawn: usize) {
    let _ = clear_lines(out, drawn);
    let _ = show_cursor(out);
    let _ = crossterm::terminal::disable_raw_mode();
    let _ = out.flush();
}
