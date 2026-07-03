//! Global prompt settings — ports `@clack/core`'s `settings.ts`.
//!
//! Right now this is a thin holder for the messages that the prompts use when
//! the user cancels or an error stops a spinner. The defaults match clack.
//! Override via [`update`] before running any prompts:
//!
//! ```no_run
//! cli_ui::prompt::settings::update(|s| s.cancel = "Aborted.".into());
//! ```

use crate::styles::{BADGE_INTRO, BOLD, CYAN, DIM, ERR, OK, WHITE, WHITE_BOLD, YELLOW};
use anstyle::Style;
use std::sync::RwLock;

/// Every color slot the prompt theme reads. Override via [`update_colors`].
#[derive(Clone, Copy)]
pub struct Colors {
    /// Small accent glyphs — radio dots (`●`/`○`), checkboxes (`◼`/`◻`),
    /// scroll arrows (`❯`), the active prompt diamond (`◆`).
    pub accent: Style,
    /// Highlight color for the *currently active* prompt frame — the `│`
    /// running down the side of the input line and the closing `└` glyph.
    /// Once the prompt is submitted, the frame fades back to `dim`.
    pub active: Style,
    /// Live user input — typed text inside text/secret/multiline/path/auto,
    /// plus the inline cursor block. White by default so what you type looks
    /// like what you typed.
    pub input: Style,
    /// Answered prompt glyph (`◇`) and success markers.
    pub success: Style,
    /// Error glyph (`▲`), error frame bar, error message text.
    pub error: Style,
    /// Cancel glyph (`■`) and cancel banner — used by Ctrl+C handler.
    pub cancel: Style,
    /// Frame bars (`│`, `┌`, `└`), placeholder text, idle option labels.
    pub dim: Style,
    /// Active question/answer text style — applied to the header line.
    pub header: Style,
    /// Fallback foreground when bold headers are off.
    pub header_plain: Style,
    /// Section title style — used by [`outro`](super::outro) and
    /// [`note`](super::note). Defaults to a pure bold attribute (no fg
    /// override) so it stays visibly bolder than regular text on every
    /// terminal that does not collapse `bold` on bright colors.
    pub title: Style,
    /// Style for the [`intro`](super::intro) pill — the clack-style badge
    /// that frames the session title. Defaults to cyan background, black
    /// text, bold. The intro renderer pads the text with one space on each
    /// side, so the badge always reads as ` <title> `.
    pub intro_badge: Style,
}

impl Default for Colors {
    fn default() -> Self {
        Self {
            accent: CYAN,
            active: CYAN,
            input: WHITE,
            success: OK,
            error: YELLOW,
            cancel: ERR,
            dim: DIM,
            header: WHITE_BOLD,
            header_plain: WHITE,
            title: BOLD,
            intro_badge: BADGE_INTRO,
        }
    }
}

/// Convenience: pull just the [`Colors`] from the current settings.
pub fn colors() -> Colors {
    get().colors
}

/// Mutate just the [`Colors`] block without touching other settings.
pub fn update_colors<F: FnOnce(&mut Colors)>(f: F) {
    update(|s| f(&mut s.colors))
}

/// Replace the colour palette wholesale — handy for shipping named themes.
///
/// ```no_run
/// use cli_ui::prompt::settings::{set_colors, Colors};
///
/// // A monochrome "no-color" palette built off the default and then tweaked.
/// let mut c = Colors::default();
/// let bold = anstyle::Style::new().bold();
/// c.accent = bold; c.success = bold; c.error = bold;
/// c.cancel = bold; c.title = bold;  c.header = bold;
/// set_colors(c);
/// ```
pub fn set_colors(c: Colors) {
    update(|s| s.colors = c);
}

/// Global prompt settings — colour palette, default messages, key
/// behaviours. Read with [`get`], mutate with [`update`] or
/// [`update_colors`], replace the colours wholesale with [`set_colors`].
#[derive(Clone)]
pub struct Settings {
    /// Default message printed when a spinner is cancelled (Ctrl-C, etc.).
    pub cancel: String,
    /// Default message printed when a spinner stops with an error.
    pub error: String,
    /// When `false`, prompts skip the connector `│` bars between rows.
    pub with_guide: bool,
    /// When `true`, h/j/k/l act as left/down/up/right inside list prompts.
    pub vim_keys: bool,
    /// When `true`, `.hint(...)` text is rendered under each prompt.
    /// Off by default — call `update(|s| s.show_hints = true)` to opt in.
    pub show_hints: bool,
    /// When `true`, prompt question text (the `◆  <header>` line) is bold.
    /// On by default — matches clack, gum, and inquirer.
    pub bold_header: bool,
    /// Color palette used by the prompt renderer.
    pub colors: Colors,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            cancel: "Operation cancelled.".into(),
            error: "Something went wrong.".into(),
            with_guide: true,
            vim_keys: true,
            show_hints: false,
            bold_header: true,
            colors: Colors::default(),
        }
    }
}

static SETTINGS: RwLock<Option<Settings>> = RwLock::new(None);

/// Get a snapshot of the current settings.
pub fn get() -> Settings {
    SETTINGS.read().unwrap().clone().unwrap_or_default()
}

/// Mutate the global settings in place.
pub fn update<F: FnOnce(&mut Settings)>(f: F) {
    let mut guard = SETTINGS.write().unwrap();
    let mut s = guard.clone().unwrap_or_default();
    f(&mut s);
    *guard = Some(s);
}
