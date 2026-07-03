//! Visual tokens for prompt rendering — clack/prompts-style.
//!
//! Every color reference here is funneled through [`settings::colors()`] so
//! palette overrides via [`settings::set_colors`] / [`settings::update_colors`]
//! take effect everywhere without per-prompt patching.
#![allow(dead_code)]

use crate::styles::paint;

// ── Symbols ───────────────────────────────────────────────────────────────────

pub const TOP: &str = "┌";
pub const BAR: &str = "│";
pub const BOT: &str = "└";
pub const STEP_OK: &str = "◇";
pub const STEP: &str = "◆";
pub const STEP_ERROR: &str = "▲";
pub const CANCEL: &str = "■";

pub const CURSOR: &str = "❯";
pub const RADIO_ON: &str = "●";
pub const RADIO_OFF: &str = "○";
pub const CHECK_ON: &str = "◼";
pub const CHECK_OFF: &str = "◻";
pub const CHECK: &str = "✓";
pub const CROSS: &str = "✘";

fn c() -> super::settings::Colors {
    super::settings::colors()
}

fn header_style() -> anstyle::Style {
    let c = c();
    if super::settings::get().bold_header {
        c.header
    } else {
        c.header_plain
    }
}

// ── Frame helpers (intro / outro / note / cancel) ─────────────────────────────

pub fn intro(text: &str) -> String {
    let c = c();
    // Wrap the title in a clack-style pill: ` <title> ` painted with
    // `c.intro_badge` (cyan bg + black fg + bold by default).
    format!(
        "{}  {}\n{}",
        paint(c.dim, TOP),
        paint(c.intro_badge, &format!(" {} ", text)),
        paint(c.dim, BAR)
    )
}

pub fn outro(text: &str) -> String {
    let c = c();
    format!(
        "{}\n{}  {}",
        paint(c.dim, BAR),
        paint(c.dim, BOT),
        paint(c.title, text)
    )
}

pub fn cancel(text: &str) -> String {
    let c = c();
    format!(
        "{}\n{}  {}",
        paint(c.dim, BAR),
        paint(c.cancel, CANCEL),
        paint(c.cancel, text)
    )
}

pub fn note(title: &str, body: &str) -> String {
    let c = c();
    let body_lines: Vec<&str> = body.lines().collect();
    // Inner content width = the widest of body, title, and a 20-char floor.
    let inner = body_lines
        .iter()
        .map(|l| l.chars().count())
        .max()
        .unwrap_or(0)
        .max(title.chars().count())
        .max(20);
    let title_chars = title.chars().count();
    // Layout: `┌  <title> <rule>┐`. We pad the rule to keep the right rail
    // at column `inner + 6` (1 corner + 2 spaces + inner + 2 spaces + 1 corner).
    let top_rule_len = inner + 1 - title_chars;
    let blank = " ".repeat(inner + 4);
    let mut out = String::new();
    out.push_str(&format!(
        "{}  {} {}{}\n",
        paint(c.dim, TOP),
        paint(c.title, title),
        paint(c.dim, &"─".repeat(top_rule_len)),
        paint(c.dim, "┐")
    ));
    out.push_str(&format!(
        "{}{}{}\n",
        paint(c.dim, BAR),
        blank,
        paint(c.dim, BAR)
    ));
    for line in &body_lines {
        let pad = inner.saturating_sub(line.chars().count());
        out.push_str(&format!(
            "{}  {}{}  {}\n",
            paint(c.dim, BAR),
            line,
            " ".repeat(pad),
            paint(c.dim, BAR)
        ));
    }
    out.push_str(&format!(
        "{}{}{}\n",
        paint(c.dim, BAR),
        blank,
        paint(c.dim, BAR)
    ));
    out.push_str(&format!(
        "{}{}{}",
        paint(c.dim, BOT),
        paint(c.dim, &"─".repeat(inner + 4)),
        paint(c.dim, "┘")
    ));
    out
}

// ── Prompt header lines ───────────────────────────────────────────────────────

pub fn label(text: &str) -> String {
    label_state(text, false)
}

pub fn label_state(text: &str, error: bool) -> String {
    let c = c();
    let (glyph, glyph_color) = if error {
        (STEP_ERROR, c.error)
    } else {
        (STEP, c.accent)
    };
    format!(
        "{}  {}",
        paint(glyph_color, glyph),
        paint(header_style(), text)
    )
}

pub fn hint(text: &str) -> String {
    let c = c();
    // Bars rendered during a live prompt use `c.active` so the whole
    // left column reads as one highlighted stripe. Bars in intro / outro
    // / answered are emitted directly with `c.dim` by those helpers.
    if text.is_empty() {
        paint(c.active, BAR)
    } else {
        format!("{}  {}", paint(c.active, BAR), paint(c.dim, text))
    }
}

pub fn hint_inline(text: Option<&str>) -> String {
    let c = c();
    match text {
        Some(h) if !h.is_empty() => format!(" {}", paint(c.dim, &format!("({h})"))),
        _ => String::new(),
    }
}

// ── Option rows ───────────────────────────────────────────────────────────────

pub fn cursor(active: bool, text: &str, hint_text: Option<&str>) -> String {
    let c = c();
    let h = hint_inline(hint_text);
    if active {
        format!(
            "{}  {} {}{}",
            paint(c.active, BAR),
            paint(c.accent, RADIO_ON),
            paint(header_style(), text),
            h
        )
    } else {
        format!(
            "{}  {} {}",
            paint(c.active, BAR),
            paint(c.dim, RADIO_OFF),
            paint(c.dim, text)
        )
    }
}

pub fn multi_option(active: bool, checked: bool, text: &str, hint_text: Option<&str>) -> String {
    let c = c();
    let glyph = if checked { CHECK_ON } else { CHECK_OFF };
    let h = hint_inline(hint_text);
    if active {
        format!(
            "{}  {} {}{}",
            paint(c.active, BAR),
            paint(c.accent, glyph),
            paint(header_style(), text),
            h
        )
    } else if checked {
        format!(
            "{}  {} {}",
            paint(c.active, BAR),
            paint(c.success, glyph),
            paint(c.dim, text)
        )
    } else {
        format!(
            "{}  {} {}",
            paint(c.active, BAR),
            paint(c.dim, glyph),
            paint(c.dim, text)
        )
    }
}

// ── Input line ────────────────────────────────────────────────────────────────

pub fn text_input(value: &str, placeholder: &str) -> String {
    let c = c();
    if value.is_empty() {
        format!("{}  {}", paint(c.dim, BAR), paint(c.dim, placeholder))
    } else {
        format!("{}  {}", paint(c.dim, BAR), paint(c.input, value))
    }
}

// ── Confirm line ──────────────────────────────────────────────────────────────

pub fn confirm_line(value: bool) -> String {
    let c = c();
    let yes = if value {
        paint(c.accent, "● Yes")
    } else {
        paint(c.dim, "○ Yes")
    };
    let no = if !value {
        paint(c.accent, "● No")
    } else {
        paint(c.dim, "○ No")
    };
    format!("{}  {} / {}", paint(c.dim, BAR), yes, no)
}

// ── Status lines ──────────────────────────────────────────────────────────────

pub fn error_line(msg: &str) -> String {
    let c = c();
    format!(
        "{}  {} {}",
        paint(c.dim, BAR),
        paint(c.error, CROSS),
        paint(c.error, msg)
    )
}

pub fn input_bar(error: bool) -> String {
    let c = c();
    if error {
        paint(c.error, BAR)
    } else {
        paint(c.active, BAR)
    }
}

pub fn frame_bot(error: Option<&str>) -> String {
    let c = c();
    match error {
        Some(msg) => format!("{}  {}", paint(c.error, BOT), paint(c.error, msg)),
        None => paint(c.active, BOT),
    }
}

// ── Answered state ────────────────────────────────────────────────────────────

pub fn answered(label_text: &str, value: &str) -> String {
    let c = c();
    // After submission the prompt keeps both the question and the answer in
    // place so the conversation stays readable:
    //
    //   ◇  Question
    //   │  value
    //   │
    //
    // The trailing `│` bridges into the next prompt's frame.
    format!(
        "{}  {}\r\n{}  {}\r\n{}",
        paint(c.success, STEP_OK),
        paint(header_style(), label_text),
        paint(c.dim, BAR),
        paint(c.input, value),
        paint(c.dim, BAR),
    )
}

// ── Raw output ────────────────────────────────────────────────────────────────

pub fn eprintln_raw(line: &str) {
    use std::io::Write;
    let mut err = std::io::stderr();
    let _ = err.write_all(line.as_bytes());
    let _ = err.write_all(b"\r\n");
    let _ = err.flush();
}
