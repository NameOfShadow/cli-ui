//! Free-text and masked password prompts — built on the [`Prompt`] trait.

use crate::styles::{paint, ACCENT, DIM, WHITE};
use std::io::Write;

use super::core::{Frame, Key, Prompt, RenderCtx, Step};
use super::cursor::LineBuf;
use super::error::{PromptError, Result};
use super::theme;
use super::validate::Validator;

// ─────────────────────────────────────────────────────────────────────────────
// TextPrompt
// ─────────────────────────────────────────────────────────────────────────────

/// Builder returned by [`text()`]. Configure with the chainable methods,
/// then call [`run`](Self::run) to read a line from the user.
pub struct TextPrompt {
    label: String,
    default: Option<String>,
    placeholder: Option<String>,
    hint: Option<String>,
    validate: Option<Validator>,
    buf: LineBuf,
}

/// Ask the user for one line of text.
///
/// ```no_run
/// use cli_ui::prompt::text;
///
/// let name = text("Your name")
///     .placeholder("Anya")
///     .default("Alice")
///     .run()?;
/// # Ok::<(), cli_ui::prompt::PromptError>(())
/// ```
pub fn text(label: impl Into<String>) -> TextPrompt {
    TextPrompt {
        label: label.into(),
        default: None,
        placeholder: None,
        hint: None,
        validate: None,
        buf: LineBuf::new(),
    }
}

impl TextPrompt {
    /// Value used when the user submits an empty buffer.
    pub fn default(mut self, v: impl Into<String>) -> Self {
        self.default = Some(v.into());
        self
    }
    /// Dim text shown in the input area before the user types anything.
    pub fn placeholder(mut self, v: impl Into<String>) -> Self {
        self.placeholder = Some(v.into());
        self
    }
    /// One-line hint rendered under the label. Only visible when
    /// [`settings::Settings::show_hints`](super::settings::Settings::show_hints)
    /// is `true`.
    pub fn hint(mut self, v: impl Into<String>) -> Self {
        self.hint = Some(v.into());
        self
    }
    /// Validate with an ad-hoc closure. Return `Ok(())` to accept,
    /// `Err(msg)` to reject and show `msg` as the error.
    pub fn validate<F>(mut self, f: F) -> Self
    where
        F: Fn(&str) -> std::result::Result<(), String> + Send + Sync + 'static,
    {
        self.validate = Some(Validator::new(f));
        self
    }
    /// Apply a composed [`Validator`] from the rules library — e.g.
    /// `.rule(min_chars(8).and(has_upper()))`. Reads as "this prompt obeys
    /// these rules."
    pub fn rule(mut self, v: Validator) -> Self {
        self.validate = Some(v);
        self
    }

    /// Deprecated alias for [`rule`](Self::rule).
    #[deprecated(since = "0.1.0", note = "use `.rule(...)` instead")]
    pub fn validate_with(self, v: Validator) -> Self {
        self.rule(v)
    }

    /// Run the prompt to completion.
    pub fn run(self) -> Result<String> {
        super::core::run(self)
    }

    fn submit_value(&self) -> String {
        if self.buf.is_empty() {
            self.default.clone().unwrap_or_default()
        } else {
            self.buf.value().to_string()
        }
    }
}

impl Prompt for TextPrompt {
    type Output = String;

    fn handle(&mut self, key: Key) -> Step<String> {
        match key {
            Key::Enter => {
                let value = self.submit_value();
                match self.validate.as_ref().map(|v| v.check(&value)) {
                    Some(Err(msg)) => Step::Reject(msg),
                    _ => Step::Submit(value),
                }
            }
            Key::Char(c) => {
                self.buf.insert(c);
                Step::Continue
            }
            Key::Backspace => {
                self.buf.backspace();
                Step::Continue
            }
            Key::Delete => {
                self.buf.delete();
                Step::Continue
            }
            Key::DeleteWordBack => {
                self.buf.delete_word_back();
                Step::Continue
            }
            Key::Left => {
                self.buf.left();
                Step::Continue
            }
            Key::Right => {
                self.buf.right();
                Step::Continue
            }
            Key::Home => {
                self.buf.home();
                Step::Continue
            }
            Key::End => {
                self.buf.end();
                Step::Continue
            }
            Key::Escape | Key::Interrupt => Step::Cancel,
            _ => Step::Continue,
        }
    }

    fn render(&self, ctx: RenderCtx) -> Frame {
        let mut f: Frame = Vec::with_capacity(4);
        let err = ctx.error;
        f.push(theme::label_state(&self.label, err.is_some()));
        if super::settings::get().show_hints {
            if let Some(ref h) = self.hint {
                f.push(theme::hint(h));
            }
        }
        let placeholder = self
            .placeholder
            .as_deref()
            .or(self.default.as_deref())
            .unwrap_or("");
        f.push(format!(
            "{}  {}",
            theme::input_bar(err.is_some()),
            self.buf.with_placeholder(placeholder)
        ));
        f.push(theme::frame_bot(err));
        f
    }

    fn render_answered(&self, value: &String) -> Frame {
        theme::answered(&self.label, value)
            .split("\r\n")
            .map(String::from)
            .collect()
    }

    fn run_fallback(self) -> Result<String> {
        let hint = self
            .default
            .as_deref()
            .map(|d| format!(" (default: {d})"))
            .unwrap_or_default();
        eprint!(
            "{}  {}{}: ",
            paint(ACCENT, "◇"),
            paint(WHITE, &self.label),
            paint(DIM, &hint)
        );
        std::io::stderr().flush().map_err(PromptError::Io)?;
        loop {
            let line = super::engine::fallback::read_line_raw()?;
            let value = if line.is_empty() {
                self.default.clone().unwrap_or_default()
            } else {
                line
            };
            if let Some(ref v) = self.validate {
                if let Err(msg) = v.check(&value) {
                    eprint!("  ✘ {msg}\n  Try again: ");
                    std::io::stderr().flush().map_err(PromptError::Io)?;
                    continue;
                }
            }
            return Ok(value);
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// SecretPrompt
// ─────────────────────────────────────────────────────────────────────────────

/// Builder returned by [`secret()`].
pub struct SecretPrompt {
    label: String,
    allow_empty: bool,
    hint: Option<String>,
    validate: Option<Validator>,
    buf: String,
}

/// Ask the user for a masked line of text (passwords, API keys).
///
/// Each character is shown as `•`. The submitted value contains the real
/// characters; only the on-screen rendering is masked.
///
/// ```no_run
/// use cli_ui::prompt::secret;
///
/// let token = secret("API token").run()?;
/// # Ok::<(), cli_ui::prompt::PromptError>(())
/// ```
pub fn secret(label: impl Into<String>) -> SecretPrompt {
    SecretPrompt {
        label: label.into(),
        allow_empty: false,
        hint: None,
        validate: None,
        buf: String::new(),
    }
}

impl SecretPrompt {
    /// Allow submitting an empty value (default: `false`).
    pub fn allow_empty(mut self, v: bool) -> Self {
        self.allow_empty = v;
        self
    }
    /// One-line hint rendered under the label (gated by
    /// [`settings::Settings::show_hints`](super::settings::Settings::show_hints)).
    pub fn hint(mut self, v: impl Into<String>) -> Self {
        self.hint = Some(v.into());
        self
    }
    /// Validate with an ad-hoc closure.
    pub fn validate<F>(mut self, f: F) -> Self
    where
        F: Fn(&str) -> std::result::Result<(), String> + Send + Sync + 'static,
    {
        self.validate = Some(Validator::new(f));
        self
    }
    /// Apply a composed [`Validator`] from the rules library.
    pub fn rule(mut self, v: Validator) -> Self {
        self.validate = Some(v);
        self
    }

    /// Deprecated alias for [`rule`](Self::rule).
    #[deprecated(since = "0.1.0", note = "use `.rule(...)` instead")]
    pub fn validate_with(self, v: Validator) -> Self {
        self.rule(v)
    }

    /// Run the prompt to completion.
    pub fn run(self) -> Result<String> {
        super::core::run(self)
    }
}

impl Prompt for SecretPrompt {
    type Output = String;

    fn handle(&mut self, key: Key) -> Step<String> {
        match key {
            Key::Enter => {
                if self.buf.is_empty() && !self.allow_empty {
                    return Step::Reject("Value is required".into());
                }
                match self.validate.as_ref().map(|v| v.check(&self.buf)) {
                    Some(Err(msg)) => Step::Reject(msg),
                    _ => Step::Submit(self.buf.clone()),
                }
            }
            Key::Char(c) => {
                self.buf.push(c);
                Step::Continue
            }
            Key::Backspace => {
                self.buf.pop();
                Step::Continue
            }
            Key::Escape | Key::Interrupt => Step::Cancel,
            _ => Step::Continue,
        }
    }

    fn render(&self, ctx: RenderCtx) -> Frame {
        let err = ctx.error;
        let mut f: Frame = Vec::with_capacity(4);
        f.push(theme::label_state(&self.label, err.is_some()));
        if super::settings::get().show_hints {
            if let Some(ref h) = self.hint {
                f.push(theme::hint(h));
            }
        }
        let dots = "•".repeat(self.buf.chars().count().min(32));
        f.push(format!(
            "{}  {}",
            theme::input_bar(err.is_some()),
            paint(super::settings::colors().input, &dots)
        ));
        f.push(theme::frame_bot(err));
        f
    }

    fn render_answered(&self, _value: &String) -> Frame {
        theme::answered(&self.label, "••••••••")
            .split("\r\n")
            .map(String::from)
            .collect()
    }

    fn run_fallback(self) -> Result<String> {
        eprint!(
            "{}  {} {}: ",
            paint(ACCENT, "◇"),
            paint(WHITE, &self.label),
            paint(DIM, "(input will be visible)"),
        );
        std::io::stderr().flush().map_err(PromptError::Io)?;
        loop {
            let line = super::engine::fallback::read_line_raw()?;
            if line.is_empty() && !self.allow_empty {
                eprint!("  Value cannot be empty. Try again: ");
                std::io::stderr().flush().map_err(PromptError::Io)?;
                continue;
            }
            if let Some(ref v) = self.validate {
                if let Err(msg) = v.check(&line) {
                    eprint!("  ✘ {msg}\n  Try again: ");
                    std::io::stderr().flush().map_err(PromptError::Io)?;
                    continue;
                }
            }
            return Ok(line);
        }
    }
}
