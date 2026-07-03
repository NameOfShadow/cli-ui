//! Multi-choice select prompt — built on the [`Prompt`] trait.

use super::core::{Frame, Key, Prompt, RenderCtx, Step};
use super::error::{PromptError, Result};
use super::limit_options::limit_options;
use super::theme;

#[derive(Clone)]
struct MsOption {
    value: String,
    label: String,
    hint: Option<String>,
    checked: bool,
}

/// Set of options the user checked off, returned by [`multiselect`].
#[derive(Debug, Clone)]
pub struct MultiSelected {
    items: Vec<(usize, String)>,
}

impl MultiSelected {
    /// Iterate over `(index, value)` pairs for every checked option, in
    /// the order they were added.
    pub fn iter(&self) -> impl Iterator<Item = (usize, &str)> {
        self.items.iter().map(|(i, v)| (*i, v.as_str()))
    }
    /// Zero-based indices of every checked option.
    pub fn indices(&self) -> Vec<usize> {
        self.items.iter().map(|(i, _)| *i).collect()
    }
    /// Machine values (the first argument to `.option(...)`) of every
    /// checked option.
    pub fn values(&self) -> Vec<String> {
        self.items.iter().map(|(_, v)| v.clone()).collect()
    }
    /// `true` if the user submitted with nothing checked.
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
    /// Number of checked options.
    pub fn len(&self) -> usize {
        self.items.len()
    }
}

/// Builder returned by [`multiselect()`].
pub struct MultiSelectPrompt {
    label: String,
    options: Vec<MsOption>,
    required: bool,
    prompt_hint: Option<String>,
    max_items: usize,
    cur: usize,
}

/// Ask the user to check any number of options from a list. Space toggles,
/// Enter submits.
///
/// ```no_run
/// use cli_ui::prompt::multiselect;
///
/// let features = multiselect("Pick features")
///     .option("auth",   "Authentication").checked()
///     .option("admin",  "Admin panel")
///     .run()?;
/// # Ok::<(), cli_ui::prompt::PromptError>(())
/// ```
pub fn multiselect(label: impl Into<String>) -> MultiSelectPrompt {
    MultiSelectPrompt {
        label: label.into(),
        options: Vec::new(),
        required: false,
        prompt_hint: None,
        max_items: 10,
        cur: 0,
    }
}

impl MultiSelectPrompt {
    /// Append an option. Chain `.hint("…")` and/or `.checked()` immediately
    /// after to configure the option just added.
    pub fn option(mut self, value: impl Into<String>, label: impl Into<String>) -> Self {
        self.options.push(MsOption {
            value: value.into(),
            label: label.into(),
            hint: None,
            checked: false,
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
    /// Mark the most recently added option as initially checked.
    pub fn checked(mut self) -> Self {
        if let Some(last) = self.options.last_mut() {
            last.checked = true;
        }
        self
    }
    /// One-line hint shown under the prompt label.
    pub fn prompt_hint(mut self, t: impl Into<String>) -> Self {
        self.prompt_hint = Some(t.into());
        self
    }
    /// When `true`, refuse to submit until at least one option is checked.
    pub fn required(mut self, r: bool) -> Self {
        self.required = r;
        self
    }
    /// Number of option rows visible at once.
    pub fn max_items(mut self, n: usize) -> Self {
        self.max_items = n.max(1);
        self
    }

    /// Run the prompt to completion. Panics if no options were added.
    pub fn run(self) -> Result<MultiSelected> {
        assert!(!self.options.is_empty(), "multiselect: no options added");
        super::core::run(self)
    }

    fn collect(&self) -> MultiSelected {
        MultiSelected {
            items: self
                .options
                .iter()
                .enumerate()
                .filter(|(_, o)| o.checked)
                .map(|(i, o)| (i, o.value.clone()))
                .collect(),
        }
    }
}

impl Prompt for MultiSelectPrompt {
    type Output = MultiSelected;

    fn handle(&mut self, key: Key) -> Step<MultiSelected> {
        let vim = super::settings::get().vim_keys;
        let last = self.options.len() - 1;
        match key {
            Key::Up | Key::Char('k') if vim => {
                if self.cur > 0 {
                    self.cur -= 1;
                }
                Step::Continue
            }
            Key::Down | Key::Char('j') if vim => {
                if self.cur < last {
                    self.cur += 1;
                }
                Step::Continue
            }
            Key::Up => {
                if self.cur > 0 {
                    self.cur -= 1;
                }
                Step::Continue
            }
            Key::Down => {
                if self.cur < last {
                    self.cur += 1;
                }
                Step::Continue
            }
            Key::Char(' ') => {
                self.options[self.cur].checked ^= true;
                Step::Continue
            }
            Key::Char('q') if vim => Step::Cancel,
            Key::Enter => {
                let result = self.collect();
                if self.required && result.is_empty() {
                    Step::Reject("Select at least one option".into())
                } else {
                    Step::Submit(result)
                }
            }
            Key::Escape | Key::Interrupt => Step::Cancel,
            _ => Step::Continue,
        }
    }

    fn render(&self, _ctx: RenderCtx) -> Frame {
        let mut f: Frame = Vec::with_capacity(self.max_items + 5);
        f.push(theme::label(&self.label));
        if let Some(ref h) = self.prompt_hint {
            f.push(theme::hint(h));
        }
        f.push(theme::hint(""));
        let vp = limit_options(self.options.len(), self.cur, self.max_items);
        if vp.above > 0 {
            f.push(theme::hint(&format!("↑ {} more", vp.above)));
        }
        for i in vp.start..vp.end {
            let opt = &self.options[i];
            f.push(theme::multi_option(
                i == self.cur,
                opt.checked,
                &opt.label,
                opt.hint.as_deref(),
            ));
        }
        if vp.below > 0 {
            f.push(theme::hint(&format!("↓ {} more", vp.below)));
        }
        f.push(theme::hint("Space to toggle · Enter to confirm"));
        f.push(theme::frame_bot(None));
        f
    }

    fn render_answered(&self, value: &MultiSelected) -> Frame {
        let summary: Vec<&str> = value
            .items
            .iter()
            .map(|(i, _)| self.options[*i].label.as_str())
            .collect();
        let val = if summary.is_empty() {
            "none".to_string()
        } else {
            summary.join(", ")
        };
        theme::answered(&self.label, &val)
            .split("\r\n")
            .map(String::from)
            .collect()
    }

    fn run_fallback(self) -> Result<MultiSelected> {
        use std::io::Write;
        let mut out = std::io::stderr();
        writeln!(out, "  {}", self.label).map_err(PromptError::Io)?;
        for (i, opt) in self.options.iter().enumerate() {
            let mark = if opt.checked { "●" } else { "○" };
            match &opt.hint {
                Some(h) => writeln!(out, "  {}. {} {} ({})", i + 1, mark, opt.label, h),
                None => writeln!(out, "  {}. {} {}", i + 1, mark, opt.label),
            }
            .map_err(PromptError::Io)?;
        }
        writeln!(out, "  Enter numbers separated by spaces:").map_err(PromptError::Io)?;
        out.flush().map_err(PromptError::Io)?;
        let line = super::engine::fallback::read_line_raw()?;
        let indices: Vec<usize> = line
            .split_whitespace()
            .filter_map(|t| t.parse::<usize>().ok())
            .filter(|&n| n >= 1 && n <= self.options.len())
            .map(|n| n - 1)
            .collect();
        Ok(MultiSelected {
            items: indices
                .into_iter()
                .map(|i| (i, self.options[i].value.clone()))
                .collect(),
        })
    }
}
