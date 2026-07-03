//! Single-keypress option select — built on the [`Prompt`] trait.

use crate::styles::{paint, DIM, WHITE};

use super::core::{Frame, Key, Prompt, RenderCtx, Step};
use super::error::{PromptError, Result};
use super::theme;

#[derive(Clone)]
struct KeyOption {
    key: char,
    label: String,
    hint: Option<String>,
}

/// Returned by [`select_key`]: the character pressed and the option's label.
#[derive(Debug, Clone)]
pub struct KeySelected {
    /// The character the user pressed (matches one of the option keys).
    pub key: char,
    /// Human-readable label of the chosen option.
    pub label: String,
}

/// Builder returned by [`select_key()`].
pub struct SelectKeyPrompt {
    label: String,
    options: Vec<KeyOption>,
    case_sensitive: bool,
}

/// Pick one option by a single keypress — no Enter required.
///
/// ```no_run
/// use cli_ui::prompt::select_key;
///
/// let answer = select_key("Apply?")
///     .option('y', "Yes")
///     .option('n', "No")
///     .run()?;
/// # Ok::<(), cli_ui::prompt::PromptError>(())
/// ```
pub fn select_key(label: impl Into<String>) -> SelectKeyPrompt {
    SelectKeyPrompt {
        label: label.into(),
        options: Vec::new(),
        case_sensitive: false,
    }
}

impl SelectKeyPrompt {
    /// Append an option bound to `key`.
    pub fn option(mut self, key: char, label: impl Into<String>) -> Self {
        self.options.push(KeyOption {
            key,
            label: label.into(),
            hint: None,
        });
        self
    }
    /// Attach a hint to the most recently added option.
    pub fn hint(mut self, text: impl Into<String>) -> Self {
        if let Some(last) = self.options.last_mut() {
            last.hint = Some(text.into());
        }
        self
    }
    /// Require exact case match (default: `false` — `Y` accepts `y`).
    pub fn case_sensitive(mut self, v: bool) -> Self {
        self.case_sensitive = v;
        self
    }

    /// Run the prompt to completion. Panics if no options were added.
    pub fn run(self) -> Result<KeySelected> {
        assert!(!self.options.is_empty(), "select_key: no options added");
        super::core::run(self)
    }
}

impl Prompt for SelectKeyPrompt {
    type Output = KeySelected;

    fn handle(&mut self, key: Key) -> Step<KeySelected> {
        match key {
            Key::Char(c) => {
                let hit = self.options.iter().find(|o| {
                    if self.case_sensitive {
                        o.key == c
                    } else {
                        o.key.eq_ignore_ascii_case(&c)
                    }
                });
                match hit {
                    Some(o) => Step::Submit(KeySelected {
                        key: o.key,
                        label: o.label.clone(),
                    }),
                    None => Step::Continue,
                }
            }
            Key::Escape | Key::Interrupt => Step::Cancel,
            _ => Step::Continue,
        }
    }

    fn render(&self, _ctx: RenderCtx) -> Frame {
        let c = super::settings::colors();
        let mut f: Frame = Vec::with_capacity(self.options.len() + 4);
        f.push(theme::label(&self.label));
        f.push(theme::hint(""));
        // Use the active-frame colour for the in-prompt bars so the whole
        // option block reads as one highlighted column — matches the rest
        // of the prompt family (select, multiselect, …).
        let _ = DIM;
        for o in &self.options {
            let badge = paint(c.accent, &format!(" {} ", o.key));
            let label = paint(WHITE, &o.label);
            let hint = theme::hint_inline(o.hint.as_deref());
            f.push(format!(
                "{}  {} {}{}",
                paint(c.active, "│"),
                badge,
                label,
                hint
            ));
        }
        f.push(theme::hint("press the highlighted key"));
        f.push(theme::frame_bot(None));
        f
    }

    fn render_answered(&self, value: &KeySelected) -> Frame {
        theme::answered(&self.label, &value.label)
            .split("\r\n")
            .map(String::from)
            .collect()
    }

    fn run_fallback(self) -> Result<KeySelected> {
        use std::io::Write;
        let mut out = std::io::stderr();
        writeln!(out, "  {}", self.label).map_err(PromptError::Io)?;
        for o in &self.options {
            writeln!(out, "  [{}] {}", o.key, o.label).map_err(PromptError::Io)?;
        }
        write!(out, "  Press a key (and Enter): ").map_err(PromptError::Io)?;
        out.flush().map_err(PromptError::Io)?;
        let line = super::engine::fallback::read_line_raw()?;
        let ch = line.chars().next().ok_or(PromptError::Interrupted)?;
        let hit = self
            .options
            .iter()
            .find(|o| {
                if self.case_sensitive {
                    o.key == ch
                } else {
                    o.key.eq_ignore_ascii_case(&ch)
                }
            })
            .ok_or(PromptError::Interrupted)?;
        Ok(KeySelected {
            key: hit.key,
            label: hit.label.clone(),
        })
    }
}
