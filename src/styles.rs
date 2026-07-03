//! ANSI style constants and theme support.
//!
//! All styles are zero-allocation [`anstyle::Style`] constants.
//! [`paint`] applies a style to a `&str` and returns an owned `String`.

use anstyle::{AnsiColor, Color, Style};

// ── base styles ───────────────────────────────────────────────────────────────

/// Bright magenta bold — default accent color.
pub const ACCENT: Style = Style::new()
    .fg_color(Some(Color::Ansi(AnsiColor::BrightMagenta)))
    .bold();

/// Bright black — dimmed / secondary text.
pub const DIM: Style = Style::new().fg_color(Some(Color::Ansi(AnsiColor::BrightBlack)));

/// Bold white.
pub const BOLD: Style = Style::new().bold();

/// Green — success indicators.
pub const OK: Style = Style::new().fg_color(Some(Color::Ansi(AnsiColor::Green)));

/// Bright red bold — error indicators.
pub const ERR: Style = Style::new()
    .fg_color(Some(Color::Ansi(AnsiColor::BrightRed)))
    .bold();

/// Cyan — paths and highlights.
pub const CYAN: Style = Style::new().fg_color(Some(Color::Ansi(AnsiColor::Cyan)));

/// Yellow — sizes and warnings.
pub const YELLOW: Style = Style::new().fg_color(Some(Color::Ansi(AnsiColor::Yellow)));

/// Bright white — section headers.
pub const WHITE: Style = Style::new().fg_color(Some(Color::Ansi(AnsiColor::BrightWhite)));

/// Bright white + bold — prompt question text.
pub const WHITE_BOLD: Style = Style::new()
    .fg_color(Some(Color::Ansi(AnsiColor::BrightWhite)))
    .bold();

// ── badge styles (bg + white fg + bold) ──────────────────────────────────────

/// Cyan background badge — default theme.
pub const BADGE_CYAN: Style = Style::new()
    .bg_color(Some(Color::Ansi(AnsiColor::Cyan)))
    .fg_color(Some(Color::Ansi(AnsiColor::BrightWhite)))
    .bold();

/// Cyan background, black text, bold — the clack-style intro pill.
pub const BADGE_INTRO: Style = Style::new()
    .bg_color(Some(Color::Ansi(AnsiColor::Cyan)))
    .fg_color(Some(Color::Ansi(AnsiColor::Black)))
    .bold();

/// Green background badge.
pub const BADGE_GREEN: Style = Style::new()
    .bg_color(Some(Color::Ansi(AnsiColor::Green)))
    .fg_color(Some(Color::Ansi(AnsiColor::BrightWhite)))
    .bold();

/// Yellow background badge (dark text for contrast).
pub const BADGE_YELLOW: Style = Style::new()
    .bg_color(Some(Color::Ansi(AnsiColor::Yellow)))
    .fg_color(Some(Color::Ansi(AnsiColor::Black)))
    .bold();

// ── symbols ───────────────────────────────────────────────────────────────────

/// Section diamond `◆`
pub const DIAMOND: &str = "◆";
/// Error cross `✘`
pub const CROSS: &str = "✘";
/// Vertical bar `│`
pub const BAR: &str = "│";
/// Hint arrow `➜`
pub const ARROW: &str = "➜";
/// Success check `✓`
pub const CHECK: &str = "✓";
/// Step bullet `▸`
pub const BULLET: &str = "▸";
/// Phase triangle `▶`
pub const PHASE: &str = "▶";
/// Sub-step branch `└─`
pub const BRANCH: &str = "└─";

// ── theme ─────────────────────────────────────────────────────────────────────

/// Returns `(badge_style, accent_style)` for the given theme name.
///
/// Available themes: `"cyan"` (default), `"magenta"`, `"green"`,
/// `"blue"`, `"yellow"`, `"red"`.
///
/// Used by the `#[cli(theme = "...")]` attribute — called from generated code.
///
/// # Example
/// ```rust
/// let (badge, accent) = cli_ui::styles::theme_styles("green");
/// let painted = cli_ui::styles::paint(badge, " mytool ");
/// ```
pub fn theme_styles(theme: &str) -> (Style, Style) {
    match theme {
        "magenta" | "purple" => (
            Style::new()
                .bg_color(Some(Color::Ansi(AnsiColor::Magenta)))
                .fg_color(Some(Color::Ansi(AnsiColor::BrightWhite)))
                .bold(),
            Style::new()
                .fg_color(Some(Color::Ansi(AnsiColor::BrightMagenta)))
                .bold(),
        ),
        "green" => (
            Style::new()
                .bg_color(Some(Color::Ansi(AnsiColor::Green)))
                .fg_color(Some(Color::Ansi(AnsiColor::BrightWhite)))
                .bold(),
            Style::new()
                .fg_color(Some(Color::Ansi(AnsiColor::Green)))
                .bold(),
        ),
        "blue" => (
            Style::new()
                .bg_color(Some(Color::Ansi(AnsiColor::Blue)))
                .fg_color(Some(Color::Ansi(AnsiColor::BrightWhite)))
                .bold(),
            Style::new()
                .fg_color(Some(Color::Ansi(AnsiColor::BrightBlue)))
                .bold(),
        ),
        "yellow" => (
            Style::new()
                .bg_color(Some(Color::Ansi(AnsiColor::Yellow)))
                .fg_color(Some(Color::Ansi(AnsiColor::Black)))
                .bold(),
            Style::new()
                .fg_color(Some(Color::Ansi(AnsiColor::Yellow)))
                .bold(),
        ),
        "red" => (
            Style::new()
                .bg_color(Some(Color::Ansi(AnsiColor::Red)))
                .fg_color(Some(Color::Ansi(AnsiColor::BrightWhite)))
                .bold(),
            Style::new()
                .fg_color(Some(Color::Ansi(AnsiColor::BrightRed)))
                .bold(),
        ),
        _ =>
        /* cyan default */
        {
            (
                Style::new()
                    .bg_color(Some(Color::Ansi(AnsiColor::Cyan)))
                    .fg_color(Some(Color::Ansi(AnsiColor::BrightWhite)))
                    .bold(),
                Style::new()
                    .fg_color(Some(Color::Ansi(AnsiColor::Cyan)))
                    .bold(),
            )
        }
    }
}

// ── paint ─────────────────────────────────────────────────────────────────────

/// Apply an [`anstyle::Style`] to `s` and return an owned `String`.
///
/// Respects `NO_COLOR` and pipe detection via `anstream`.
///
/// # Example
/// ```rust
/// use cli_ui::styles::{paint, OK, CHECK};
/// let line = format!("{}  done", paint(OK, CHECK));
/// ```
pub fn paint(style: Style, s: &str) -> String {
    format!("{style}{s}{style:#}")
}
