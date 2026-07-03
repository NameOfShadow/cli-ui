//! Autocomplete prompt — type to filter, arrows to navigate.

use crate::styles::{paint, DIM, OK, WHITE};

use super::core::{Frame, Key, Prompt, RenderCtx, Step};
use super::error::{PromptError, Result};
use super::theme;

#[derive(Clone)]
struct AcOption {
    value: String,
    label: String,
    hint: Option<String>,
}

/// Returned by [`autocomplete`].
#[derive(Debug, Clone)]
pub struct AutocompleteSelected {
    value: String,
    label: String,
}
impl AutocompleteSelected {
    /// Machine value of the chosen option.
    pub fn value(&self) -> &str {
        &self.value
    }
    /// Human label of the chosen option.
    pub fn label(&self) -> &str {
        &self.label
    }
}

/// Builder returned by [`autocomplete()`].
pub struct AutocompletePrompt {
    label: String,
    options: Vec<AcOption>,
    placeholder: Option<String>,
    max_items: usize,
    query: String,
    cur: usize,
}

/// Pick one option, filtered live as the user types.
///
/// ```no_run
/// use cli_ui::prompt::autocomplete;
///
/// let pkg = autocomplete("Search for a package")
///     .option("axum",     "axum")
///     .option("rocket",   "rocket")
///     .placeholder("type to filter…")
///     .run()?;
/// # Ok::<(), cli_ui::prompt::PromptError>(())
/// ```
pub fn autocomplete(label: impl Into<String>) -> AutocompletePrompt {
    AutocompletePrompt {
        label: label.into(),
        options: Vec::new(),
        placeholder: None,
        max_items: 8,
        query: String::new(),
        cur: 0,
    }
}

impl AutocompletePrompt {
    /// Append an option.
    pub fn option(mut self, value: impl Into<String>, label: impl Into<String>) -> Self {
        self.options.push(AcOption {
            value: value.into(),
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
    /// Placeholder shown in the search area before the user types.
    pub fn placeholder(mut self, p: impl Into<String>) -> Self {
        self.placeholder = Some(p.into());
        self
    }
    /// Maximum number of filtered rows shown at once.
    pub fn max_items(mut self, n: usize) -> Self {
        self.max_items = n.max(1);
        self
    }

    /// Run the prompt. Panics if no options were added.
    pub fn run(self) -> Result<AutocompleteSelected> {
        assert!(!self.options.is_empty(), "autocomplete: no options added");
        super::core::run(self)
    }

    fn filter(&self) -> Vec<AcOption> {
        let q = self.query.to_lowercase();
        self.options
            .iter()
            .filter(|o| {
                q.is_empty()
                    || o.label.to_lowercase().contains(&q)
                    || o.value.to_lowercase().contains(&q)
            })
            .take(self.max_items)
            .cloned()
            .collect()
    }
}

impl Prompt for AutocompletePrompt {
    type Output = AutocompleteSelected;

    fn handle(&mut self, key: Key) -> Step<AutocompleteSelected> {
        match key {
            Key::Char(c) => {
                self.query.push(c);
                self.cur = 0;
                Step::Continue
            }
            Key::Backspace => {
                self.query.pop();
                self.cur = 0;
                Step::Continue
            }
            Key::Up => {
                if self.cur > 0 {
                    self.cur -= 1;
                }
                Step::Continue
            }
            Key::Down => {
                if self.cur + 1 < self.filter().len() {
                    self.cur += 1;
                }
                Step::Continue
            }
            Key::Enter => {
                let f = self.filter();
                if f.is_empty() {
                    Step::Continue
                } else {
                    let opt = &f[self.cur];
                    Step::Submit(AutocompleteSelected {
                        value: opt.value.clone(),
                        label: opt.label.clone(),
                    })
                }
            }
            Key::Escape | Key::Interrupt => Step::Cancel,
            _ => Step::Continue,
        }
    }

    fn render(&self, _ctx: RenderCtx) -> Frame {
        let mut f: Frame = Vec::with_capacity(self.max_items + 5);
        f.push(theme::label(&self.label));
        f.push(theme::hint(""));
        let ph = self.placeholder.as_deref().unwrap_or("Type to search...");
        let search = if self.query.is_empty() {
            format!(
                "{}  {} {}",
                paint(DIM, "│"),
                paint(DIM, "Search:"),
                paint(DIM, ph)
            )
        } else {
            format!(
                "{}  {} {}",
                paint(DIM, "│"),
                paint(DIM, "Search:"),
                paint(super::settings::colors().input, &self.query)
            )
        };
        f.push(search);
        let filtered = self.filter();
        if filtered.is_empty() {
            f.push(format!("{}  {}", paint(DIM, "│"), paint(DIM, "No matches")));
        } else {
            for (i, opt) in filtered.iter().enumerate() {
                let active = i == self.cur;
                let bullet = if active {
                    paint(OK, "●")
                } else {
                    paint(DIM, "○")
                };
                let label = if active {
                    paint(WHITE, &opt.label)
                } else {
                    paint(DIM, &opt.label)
                };
                let hint_sfx = theme::hint_inline(opt.hint.as_deref());
                let hint_col = if active {
                    hint_sfx
                } else {
                    paint(DIM, &hint_sfx)
                };
                f.push(format!(
                    "{}  {} {}{}",
                    paint(DIM, "│"),
                    bullet,
                    label,
                    hint_col
                ));
            }
        }
        f.push(theme::hint(
            "↑/↓ to navigate · Enter to confirm · type to search",
        ));
        f.push(theme::frame_bot(None));
        f
    }

    fn render_answered(&self, value: &AutocompleteSelected) -> Frame {
        theme::answered(&self.label, &value.label)
            .split("\r\n")
            .map(String::from)
            .collect()
    }

    fn run_fallback(self) -> Result<AutocompleteSelected> {
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
                    return Ok(AutocompleteSelected {
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
