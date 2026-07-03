//! Help renderer — called from generated `help()` method.
//!
//! You don't normally use this module directly; it is driven by
//! the `#[derive(CliOptions)]` macro.

use crate::styles::*;
use anstyle::Style;

/// A named section in the help output (e.g. "Arguments", "Assets").
pub struct HelpSection {
    /// Section title shown after the `◆` symbol.
    pub title: &'static str,
    /// Entries within the section.
    pub entries: Vec<HelpEntry>,
}

/// A single line within a [`HelpSection`].
pub enum HelpEntry {
    /// A key/description pair, aligned in columns across the section.
    Pair {
        /// Left column — e.g. `--port <N>`.
        key: String,
        /// Right column description.
        desc: String,
    },
    /// A free-form dimmed line.
    Detail(String),
}

/// Render a complete help page to stderr.
///
/// Called by the generated `YourStruct::help()` method.
/// All styling is controlled by `badge` and `accent` from the app's theme.
#[allow(clippy::too_many_arguments)]
pub fn render(
    name: &str,
    version: &str,
    about: &str,
    tagline: &str,
    url: &str,
    usage: &str,
    sections: &[HelpSection],
    examples: &[&str],
    hint: &str,
    badge: Style,
    accent: Style,
) {
    let bar = paint(DIM, BAR);
    let _ = badge; // theme already conveyed via accent color

    // ── header ────────────────────────────────────────────────────────
    eprintln!();
    eprintln!(
        "{} {} {}",
        paint(accent, DIAMOND),
        paint(WHITE_BOLD, name),
        paint(accent, &format!("v{version}")),
    );
    let subtitle = match (about.is_empty(), tagline.is_empty()) {
        (false, false) => format!("{about} · {tagline}"),
        (false, true) => about.to_string(),
        (true, false) => tagline.to_string(),
        (true, true) => String::new(),
    };
    if !subtitle.is_empty() {
        eprintln!("{}  {}", bar, paint(DIM, &subtitle));
    }
    eprintln!();

    // ── usage ─────────────────────────────────────────────────────────
    print_section_title(accent, "Usage");
    eprintln!("{}  {}", bar, style_usage(usage, accent));
    eprintln!();

    // ── sections ──────────────────────────────────────────────────────
    for section in sections {
        print_section_title(accent, section.title);

        let max_key = section
            .entries
            .iter()
            .filter_map(|e| {
                if let HelpEntry::Pair { key, .. } = e {
                    Some(visible_width(key))
                } else {
                    None
                }
            })
            .max()
            .unwrap_or(0);

        for entry in &section.entries {
            match entry {
                HelpEntry::Pair { key, desc } => {
                    let pad = " ".repeat(max_key - visible_width(key) + 2);
                    let styled_key = style_key(key, accent);
                    let styled_desc = style_desc(desc);
                    eprintln!("{}  {}{}{}", bar, styled_key, pad, styled_desc);
                }
                HelpEntry::Detail(line) => {
                    eprintln!("{}  {}", bar, paint(DIM, line));
                }
            }
        }
        eprintln!();
    }

    // ── examples ──────────────────────────────────────────────────────
    if !examples.is_empty() {
        print_section_title(accent, "Examples");
        for ex in examples {
            eprintln!("{}  {} {}", bar, paint(DIM, "$"), paint(WHITE_BOLD, ex));
        }
        eprintln!();
    }

    // ── notes (hint) ──────────────────────────────────────────────────
    if !hint.is_empty() {
        print_section_title(accent, "Notes");
        eprintln!("{}  {} {}", bar, paint(accent, ARROW), paint(WHITE, hint));
        eprintln!();
    }

    // ── url ───────────────────────────────────────────────────────────
    if !url.is_empty() {
        let link = Style::new()
            .fg_color(Some(anstyle::Color::Ansi(anstyle::AnsiColor::BrightCyan)))
            .underline();
        print_section_title(accent, "Link");
        eprintln!("{}  {}", bar, paint(link, url));
        eprintln!();
    }
}

// ── helpers ───────────────────────────────────────────────────────────

fn print_section_title(accent: Style, title: &str) {
    eprintln!("{} {}", paint(accent, DIAMOND), paint(accent, title));
}

/// Style a `key` cell.
///
/// Rules:
/// - Positional `<NAME>` (whole key in angle brackets) → accent bold.
/// - Regular flag key → BOLD white, `<VALUE>` placeholders DIM, `/` separator DIM.
/// - Meta section → whole key DIM.
fn style_key(key: &str, accent: Style) -> String {
    let indent_end = key.len() - key.trim_start().len();
    let (indent, body) = key.split_at(indent_end);

    // Positional: `<INPUT>` — no spaces, wrapped in angle brackets.
    if body.starts_with('<') && body.ends_with('>') && !body.contains(' ') {
        return format!("{}{}", indent, paint(accent, body));
    }

    // Flag key: split by " / " (negatable pair), style each side.
    let parts: Vec<String> = body.split(" / ").map(style_flag_part).collect();
    let sep = format!(" {} ", paint(DIM, "/"));
    format!("{}{}", indent, parts.join(&sep))
}

/// Style a single flag chunk like `-j, --jobs` or `    --format <STR>`.
/// Angle-bracket runs → DIM, everything else → BOLD white.
fn style_flag_part(part: &str) -> String {
    let mut out = String::new();
    let mut buf = String::new();
    let mut chars = part.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '<' {
            if !buf.is_empty() {
                out.push_str(&paint(WHITE_BOLD, &buf));
                buf.clear();
            }
            let mut angle = String::from("<");
            for c2 in chars.by_ref() {
                angle.push(c2);
                if c2 == '>' {
                    break;
                }
            }
            // consume trailing `...` (multi flag) into the dim segment
            while chars.peek() == Some(&'.') {
                angle.push(chars.next().unwrap());
            }
            out.push_str(&paint(DIM, &angle));
        } else {
            buf.push(c);
        }
    }
    if !buf.is_empty() {
        out.push_str(&paint(WHITE_BOLD, &buf));
    }
    out
}

/// Style the description cell. Detects trailing `[default: ...]` and dims it.
fn style_desc(desc: &str) -> String {
    if let Some(idx) = desc.rfind("[default:") {
        if desc.trim_end().ends_with(']') {
            let head = desc[..idx].trim_end();
            let tail = &desc[idx..];
            return format!("{}  {}", paint(WHITE, head), paint(DIM, tail));
        }
    }
    paint(WHITE, desc)
}

/// Style the usage line: `name` bold, positional `<...>` accent, `[OPTIONS]` dim.
fn style_usage(usage: &str, accent: Style) -> String {
    let mut out = String::new();
    let mut buf = String::new();
    let mut chars = usage.chars().peekable();
    let mut first_word = true;

    let flush = |buf: &mut String, out: &mut String, first_word: &mut bool| {
        if buf.is_empty() {
            return;
        }
        // First word is the program name — BOLD white.
        if *first_word {
            let trimmed = buf.trim_end();
            let trailing = &buf[trimmed.len()..].to_string();
            out.push_str(&paint(WHITE_BOLD, trimmed));
            out.push_str(trailing);
            *first_word = false;
        } else {
            out.push_str(&paint(WHITE, buf));
        }
        buf.clear();
    };

    while let Some(c) = chars.next() {
        match c {
            '<' => {
                flush(&mut buf, &mut out, &mut first_word);
                let mut token = String::from("<");
                for c2 in chars.by_ref() {
                    token.push(c2);
                    if c2 == '>' {
                        break;
                    }
                }
                out.push_str(&paint(accent, &token));
            }
            '[' => {
                flush(&mut buf, &mut out, &mut first_word);
                let mut token = String::from("[");
                for c2 in chars.by_ref() {
                    token.push(c2);
                    if c2 == ']' {
                        break;
                    }
                }
                out.push_str(&paint(DIM, &token));
            }
            _ => buf.push(c),
        }
    }
    flush(&mut buf, &mut out, &mut first_word);
    out
}

/// Visible width of a raw (un-styled) help key. All keys are ASCII in practice,
/// so `len()` is fine — kept as a function for symmetry with future non-ASCII keys.
fn visible_width(s: &str) -> usize {
    s.chars().count()
}
