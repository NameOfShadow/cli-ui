//! Boolean confirmation prompt — built on the [`Prompt`] trait.

use crate::styles::{paint, ACCENT, DIM, WHITE};
use std::io::Write;

use super::core::{Frame, Key, Prompt, RenderCtx, Step};
use super::error::{PromptError, Result};
use super::theme;

/// Builder returned by [`confirm()`].
pub struct ConfirmPrompt {
    label: String,
    default: Option<bool>,
    value: bool,
}

/// Ask the user a yes/no question. Returns `true` for Yes.
///
/// ```no_run
/// use cli_ui::prompt::confirm;
///
/// let init_git = confirm("Initialise git?").default(true).run()?;
/// # Ok::<(), cli_ui::prompt::PromptError>(())
/// ```
pub fn confirm(label: impl Into<String>) -> ConfirmPrompt {
    ConfirmPrompt {
        label: label.into(),
        default: None,
        value: true,
    }
}

impl ConfirmPrompt {
    /// Initial selection — also accepted as the value when the user just
    /// hits Enter without choosing.
    pub fn default(mut self, v: bool) -> Self {
        self.default = Some(v);
        self.value = v;
        self
    }

    /// Run the prompt to completion.
    pub fn run(self) -> Result<bool> {
        super::core::run(self)
    }
}

impl Prompt for ConfirmPrompt {
    type Output = bool;

    fn handle(&mut self, key: Key) -> Step<bool> {
        match key {
            Key::Left | Key::Right => {
                self.value = !self.value;
                Step::Continue
            }
            Key::Char('y') | Key::Char('Y') => Step::Submit(true),
            Key::Char('n') | Key::Char('N') => Step::Submit(false),
            Key::Enter => Step::Submit(self.value),
            Key::Escape | Key::Interrupt => Step::Cancel,
            _ => Step::Continue,
        }
    }

    fn render(&self, _ctx: RenderCtx) -> Frame {
        vec![
            theme::label(&self.label),
            theme::confirm_line(self.value),
            theme::frame_bot(None),
        ]
    }

    fn render_answered(&self, value: &bool) -> Frame {
        theme::answered(&self.label, if *value { "Yes" } else { "No" })
            .split("\r\n")
            .map(String::from)
            .collect()
    }

    fn run_fallback(self) -> Result<bool> {
        let hint = match self.default {
            Some(true) => "Y/n",
            Some(false) => "y/N",
            None => "y/n",
        };
        eprint!(
            "{}  {} [{}]: ",
            paint(ACCENT, "◆"),
            paint(WHITE, &self.label),
            paint(DIM, hint)
        );
        std::io::stderr().flush().map_err(PromptError::Io)?;
        loop {
            let line = super::engine::fallback::read_line_raw()?;
            match line.to_lowercase().as_str() {
                "y" | "yes" => return Ok(true),
                "n" | "no" => return Ok(false),
                "" if self.default.is_some() => return Ok(self.default.unwrap()),
                _ => {
                    eprint!("  Please enter y or n: ");
                    std::io::stderr().flush().map_err(PromptError::Io)?;
                }
            }
        }
    }
}
