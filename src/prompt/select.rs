//! Single-choice select — built on the [`Prompt`] trait.

use super::core::{Frame, Key, Prompt, RenderCtx, Step};
use super::error::{PromptError, Result};
use super::limit_options::limit_options;
use super::theme;

#[derive(Clone)]
struct SelectOption {
    value: String,
    label: String,
    hint: Option<String>,
}

/// Returned by [`select`] — the chosen option's index, machine-readable
/// `value`, and human-readable `label`.
#[derive(Debug, Clone)]
pub struct Selected {
    /// Zero-based position of the chosen option in the order it was added.
    pub index: usize,
    /// The first argument to `.option(value, label)` — typically used
    /// programmatically (e.g. as a config key).
    pub value: String,
    /// The second argument — the human label.
    pub label: String,
}

impl Selected {
    /// Position of the chosen option (zero-based).
    pub fn index(&self) -> usize {
        self.index
    }
    /// The chosen option's machine value.
    pub fn value(&self) -> &str {
        &self.value
    }
    /// The chosen option's human label.
    pub fn label(&self) -> &str {
        &self.label
    }
}

/// Builder returned by [`select()`].
pub struct SelectPrompt {
    label: String,
    options: Vec<SelectOption>,
    prompt_hint: Option<String>,
    max_items: usize,
    pos: usize,
}

/// Ask the user to pick one option from a list.
///
/// ```no_run
/// use cli_ui::prompt::select;
///
/// let stack = select("Pick a stack")
///     .option("rust", "Rust")
///     .option("ts",   "TypeScript")
///     .run()?;
/// # Ok::<(), cli_ui::prompt::PromptError>(())
/// ```
pub fn select(label: impl Into<String>) -> SelectPrompt {
    SelectPrompt {
        label: label.into(),
        options: Vec::new(),
        prompt_hint: None,
        max_items: 10,
        pos: 0,
    }
}

impl SelectPrompt {
    /// Append an option. `value` is the machine identifier, `label` is what
    /// the user sees. Chain `.hint("…")` immediately after to attach a hint
    /// to this option.
    pub fn option(mut self, value: impl Into<String>, label: impl Into<String>) -> Self {
        self.options.push(SelectOption {
            value: value.into(),
            label: label.into(),
            hint: None,
        });
        self
    }
    /// Attach a hint to the **most recently added** option. Renders inline
    /// next to the option label, e.g. `Rust (systems programming)`.
    pub fn hint(mut self, text: impl Into<String>) -> Self {
        if let Some(last) = self.options.last_mut() {
            last.hint = Some(text.into());
        }
        self
    }
    /// One-line hint shown under the prompt label (not per-option).
    pub fn prompt_hint(mut self, text: impl Into<String>) -> Self {
        self.prompt_hint = Some(text.into());
        self
    }
    /// Index of the initially selected option.
    pub fn default(mut self, index: usize) -> Self {
        self.pos = index;
        self
    }
    /// Number of option rows visible at once. Longer lists scroll with
    /// `↑ N more` / `↓ N more` indicators.
    pub fn max_items(mut self, n: usize) -> Self {
        self.max_items = n.max(1);
        self
    }

    /// Run the prompt to completion. Panics if no options were added.
    pub fn run(mut self) -> Result<Selected> {
        assert!(!self.options.is_empty(), "select: no options added");
        self.pos = self.pos.min(self.options.len() - 1);
        super::core::run(self)
    }
}

impl Prompt for SelectPrompt {
    type Output = Selected;

    fn handle(&mut self, key: Key) -> Step<Selected> {
        let vim = super::settings::get().vim_keys;
        let last = self.options.len() - 1;
        match key {
            Key::Up | Key::Char('k') if vim => {
                if self.pos > 0 {
                    self.pos -= 1;
                }
                Step::Continue
            }
            Key::Down | Key::Char('j') if vim => {
                if self.pos < last {
                    self.pos += 1;
                }
                Step::Continue
            }
            Key::Up => {
                if self.pos > 0 {
                    self.pos -= 1;
                }
                Step::Continue
            }
            Key::Down => {
                if self.pos < last {
                    self.pos += 1;
                }
                Step::Continue
            }
            Key::Char('q') if vim => Step::Cancel,
            Key::Enter => {
                let opt = &self.options[self.pos];
                Step::Submit(Selected {
                    index: self.pos,
                    value: opt.value.clone(),
                    label: opt.label.clone(),
                })
            }
            Key::Escape | Key::Interrupt => Step::Cancel,
            _ => Step::Continue,
        }
    }

    fn render(&self, _ctx: RenderCtx) -> Frame {
        let mut f: Frame = Vec::with_capacity(self.max_items + 4);
        f.push(theme::label(&self.label));
        if let Some(ref h) = self.prompt_hint {
            f.push(theme::hint(h));
        }
        f.push(theme::hint(""));
        let vp = limit_options(self.options.len(), self.pos, self.max_items);
        if vp.above > 0 {
            f.push(theme::hint(&format!("↑ {} more", vp.above)));
        }
        for i in vp.start..vp.end {
            let opt = &self.options[i];
            f.push(theme::cursor(
                i == self.pos,
                &opt.label,
                opt.hint.as_deref(),
            ));
        }
        if vp.below > 0 {
            f.push(theme::hint(&format!("↓ {} more", vp.below)));
        }
        f.push(theme::frame_bot(None));
        f
    }

    fn render_answered(&self, value: &Selected) -> Frame {
        theme::answered(&self.label, &value.label)
            .split("\r\n")
            .map(String::from)
            .collect()
    }

    fn run_fallback(self) -> Result<Selected> {
        use std::io::Write;
        let mut out = std::io::stderr();
        writeln!(out, "  {}", self.label).map_err(PromptError::Io)?;
        for (i, opt) in self.options.iter().enumerate() {
            match &opt.hint {
                Some(h) => writeln!(out, "  {}. {} ({})", i + 1, opt.label, h),
                None => writeln!(out, "  {}. {}", i + 1, opt.label),
            }
            .map_err(PromptError::Io)?;
        }
        loop {
            write!(out, "  Enter number (1–{}): ", self.options.len()).map_err(PromptError::Io)?;
            out.flush().map_err(PromptError::Io)?;
            let line = super::engine::fallback::read_line_raw()?;
            if let Ok(n) = line.parse::<usize>() {
                if n >= 1 && n <= self.options.len() {
                    let opt = &self.options[n - 1];
                    return Ok(Selected {
                        index: n - 1,
                        value: opt.value.clone(),
                        label: opt.label.clone(),
                    });
                }
            }
            writeln!(
                out,
                "  Please enter a number between 1 and {}.",
                self.options.len()
            )
            .map_err(PromptError::Io)?;
        }
    }
}
