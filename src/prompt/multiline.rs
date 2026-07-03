//! Multi-line text input — built on the [`Prompt`] trait.

use crate::styles::{paint, DIM};

use super::core::{Frame, Key, Prompt, RenderCtx, Step};
use super::error::{PromptError, Result};
use super::theme;
use super::validate::Validator;

/// Builder returned by [`multiline()`].
pub struct MultilinePrompt {
    label: String,
    placeholder: Option<String>,
    show_submit: bool,
    validate: Option<Validator>,
    buf: Vec<String>,
    focus_submit: bool,
}

/// Ask the user for multiple lines of text. Enter inserts a newline; submit
/// by pressing Enter on an empty line, or by Tab-focusing the on-screen
/// `[ submit ]` button when [`MultilinePrompt::show_submit`] is `true`.
///
/// ```no_run
/// use cli_ui::prompt::multiline;
///
/// let bio = multiline("Tell us about yourself")
///     .placeholder("…")
///     .show_submit(true)
///     .run()?;
/// # Ok::<(), cli_ui::prompt::PromptError>(())
/// ```
pub fn multiline(label: impl Into<String>) -> MultilinePrompt {
    MultilinePrompt {
        label: label.into(),
        placeholder: None,
        show_submit: false,
        validate: None,
        buf: vec![String::new()],
        focus_submit: false,
    }
}

impl MultilinePrompt {
    /// Dim text shown in the input area before the user types.
    pub fn placeholder(mut self, v: impl Into<String>) -> Self {
        self.placeholder = Some(v.into());
        self
    }
    /// Show a `[ submit ]` button focused with Tab (default: `false`,
    /// which means double-Enter submits).
    pub fn show_submit(mut self, v: bool) -> Self {
        self.show_submit = v;
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

    fn try_submit(&self) -> Option<String> {
        let trailing_empty = self.buf.last().map(|s| s.is_empty()).unwrap_or(false)
            && self.buf.len() >= 2
            && self.buf[self.buf.len() - 2].is_empty();
        if !(self.focus_submit || trailing_empty) {
            return None;
        }
        let mut joined: Vec<&str> = self.buf.iter().map(String::as_str).collect();
        if !self.focus_submit {
            joined.truncate(joined.len() - 2);
        }
        Some(joined.join("\n"))
    }
}

impl Prompt for MultilinePrompt {
    type Output = String;

    fn handle(&mut self, key: Key) -> Step<String> {
        match key {
            Key::Char('\t') if self.show_submit => {
                self.focus_submit = !self.focus_submit;
                Step::Continue
            }
            Key::Char(c) => {
                self.focus_submit = false;
                self.buf.last_mut().unwrap().push(c);
                Step::Continue
            }
            Key::Backspace => {
                if let Some(last) = self.buf.last_mut() {
                    if last.pop().is_none() && self.buf.len() > 1 {
                        self.buf.pop();
                    }
                }
                Step::Continue
            }
            Key::Enter => {
                if let Some(value) = self.try_submit() {
                    if let Some(ref v) = self.validate {
                        if let Err(msg) = v.check(&value) {
                            return Step::Reject(msg);
                        }
                    }
                    return Step::Submit(value);
                }
                self.buf.push(String::new());
                Step::Continue
            }
            Key::Escape | Key::Interrupt => Step::Cancel,
            _ => Step::Continue,
        }
    }

    fn render(&self, _ctx: RenderCtx) -> Frame {
        let mut f: Frame = Vec::with_capacity(self.buf.len() + 4);
        f.push(theme::label(&self.label));
        let placeholder = self.placeholder.as_deref().unwrap_or("");
        let is_empty = self.buf.len() == 1 && self.buf[0].is_empty();
        if is_empty && !placeholder.is_empty() {
            f.push(format!("{}  {}", paint(DIM, "│"), paint(DIM, placeholder)));
        } else {
            for line in &self.buf {
                f.push(format!(
                    "{}  {}",
                    paint(DIM, "│"),
                    paint(super::settings::colors().input, line)
                ));
            }
        }
        if self.show_submit {
            let c = super::settings::colors();
            let style = if self.focus_submit { c.accent } else { c.dim };
            f.push(format!(
                "{}  {}",
                paint(DIM, "│"),
                paint(style, "[ submit ]")
            ));
        }
        f.push(theme::hint(if self.show_submit {
            "Tab to focus submit · Enter to insert newline"
        } else {
            "Enter for newline · Enter on empty line to submit"
        }));
        f.push(theme::frame_bot(None));
        f
    }

    fn render_answered(&self, value: &String) -> Frame {
        let snippet: String = value
            .lines()
            .next()
            .unwrap_or("")
            .chars()
            .take(40)
            .collect();
        let suffix = if value.lines().count() > 1 {
            " …"
        } else {
            ""
        };
        theme::answered(&self.label, &format!("{snippet}{suffix}"))
            .split("\r\n")
            .map(String::from)
            .collect()
    }

    fn run_fallback(self) -> Result<String> {
        use std::io::{BufRead, Write};
        let mut err = std::io::stderr();
        writeln!(err, "  {} (end with empty line)", self.label).map_err(PromptError::Io)?;
        err.flush().map_err(PromptError::Io)?;
        let stdin = std::io::stdin();
        let mut acc = String::new();
        for line in stdin.lock().lines() {
            let l = line.map_err(PromptError::Io)?;
            if l.is_empty() {
                break;
            }
            if !acc.is_empty() {
                acc.push('\n');
            }
            acc.push_str(&l);
        }
        Ok(acc)
    }
}
