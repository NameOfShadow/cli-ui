//! Grouped multi-select prompt — built on the [`Prompt`] trait.

use crate::styles::{paint, ACCENT, DIM, OK, WHITE};

use super::core::{Frame, Key, Prompt, RenderCtx, Step};
use super::error::{PromptError, Result};
use super::theme;

#[derive(Clone, Copy)]
enum Row {
    Header { group_idx: usize },
    Item { group_idx: usize, item_idx: usize },
}

#[derive(Default)]
struct ItemData {
    value: String,
    hint: Option<String>,
    checked: bool,
}

struct GroupData {
    name: String,
    items: Vec<ItemData>,
}

impl GroupData {
    fn all_checked(&self) -> bool {
        self.items.iter().all(|i| i.checked)
    }
    fn any_checked(&self) -> bool {
        self.items.iter().any(|i| i.checked)
    }
    fn toggle_all(&mut self) {
        let t = !self.all_checked();
        for item in &mut self.items {
            item.checked = t;
        }
    }
}

/// One checked item inside a [`GroupSelected`] result.
#[derive(Debug, Clone)]
pub struct GroupItem {
    /// Name of the group the item belongs to.
    pub group: String,
    /// Machine value of the item (the argument to `.item(value)`).
    pub value: String,
    /// Zero-based position within its group.
    pub index: usize,
}

/// Result of [`groupmultiselect`] — every item the user checked.
#[derive(Debug, Clone, Default)]
pub struct GroupSelected {
    items: Vec<GroupItem>,
}

impl GroupSelected {
    /// Iterate over every checked [`GroupItem`].
    pub fn iter(&self) -> impl Iterator<Item = &GroupItem> {
        self.items.iter()
    }
    /// `true` if the user submitted with nothing checked.
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
    /// Number of checked items across all groups.
    pub fn len(&self) -> usize {
        self.items.len()
    }
    /// Machine values of every checked item.
    pub fn values(&self) -> Vec<&str> {
        self.items.iter().map(|i| i.value.as_str()).collect()
    }
    /// Machine values of checked items in a single group.
    pub fn group_values(&self, group: &str) -> Vec<&str> {
        self.items
            .iter()
            .filter(|i| i.group == group)
            .map(|i| i.value.as_str())
            .collect()
    }
}

/// Builder returned by [`groupmultiselect()`].
pub struct GroupMultiSelectPrompt {
    label: String,
    groups: Vec<GroupData>,
    required: bool,
    current_grp: Option<usize>,
    rows: Vec<Row>,
    cur: usize,
}

/// Multi-select prompt with grouped options. Each group can be toggled
/// in bulk by hitting Space on the group header.
///
/// ```no_run
/// use cli_ui::prompt::groupmultiselect;
///
/// let tools = groupmultiselect("Tools")
///     .group("Lint")
///         .item("clippy").checked()
///     .group("Format")
///         .item("rustfmt")
///     .run()?;
/// # Ok::<(), cli_ui::prompt::PromptError>(())
/// ```
pub fn groupmultiselect(label: impl Into<String>) -> GroupMultiSelectPrompt {
    GroupMultiSelectPrompt {
        label: label.into(),
        groups: Vec::new(),
        required: false,
        current_grp: None,
        rows: Vec::new(),
        cur: 0,
    }
}

impl GroupMultiSelectPrompt {
    /// Start a new group. Subsequent [`item`](Self::item) calls add items
    /// to this group.
    pub fn group(mut self, name: impl Into<String>) -> Self {
        self.groups.push(GroupData {
            name: name.into(),
            items: Vec::new(),
        });
        self.current_grp = Some(self.groups.len() - 1);
        self
    }
    /// Append an item to the current group. Panics if [`group`](Self::group)
    /// was never called.
    pub fn item(mut self, value: impl Into<String>) -> Self {
        let idx = self.current_grp.expect("call .group() before .item()");
        self.groups[idx].items.push(ItemData {
            value: value.into(),
            hint: None,
            checked: false,
        });
        self
    }
    /// Attach a hint to the most recently added item.
    pub fn hint(mut self, text: impl Into<String>) -> Self {
        if let Some(idx) = self.current_grp {
            if let Some(last) = self.groups[idx].items.last_mut() {
                last.hint = Some(text.into());
            }
        }
        self
    }
    /// Mark the most recently added item as initially checked.
    pub fn checked(mut self) -> Self {
        if let Some(idx) = self.current_grp {
            if let Some(last) = self.groups[idx].items.last_mut() {
                last.checked = true;
            }
        }
        self
    }
    /// When `true`, refuse to submit until at least one item is checked.
    pub fn required(mut self, r: bool) -> Self {
        self.required = r;
        self
    }

    /// Run the prompt. Panics if any group has no items.
    pub fn run(mut self) -> Result<GroupSelected> {
        assert!(!self.groups.is_empty(), "groupmultiselect: no groups added");
        assert!(
            self.groups.iter().all(|g| !g.items.is_empty()),
            "groupmultiselect: every group must have at least one item"
        );
        self.rows = build_rows(&self.groups);
        super::core::run(self)
    }

    fn collect(&self) -> GroupSelected {
        GroupSelected {
            items: self
                .groups
                .iter()
                .flat_map(|g| {
                    g.items
                        .iter()
                        .enumerate()
                        .filter(|(_, it)| it.checked)
                        .map(|(i, it)| GroupItem {
                            group: g.name.clone(),
                            value: it.value.clone(),
                            index: i,
                        })
                        .collect::<Vec<_>>()
                })
                .collect(),
        }
    }
}

impl Prompt for GroupMultiSelectPrompt {
    type Output = GroupSelected;

    fn handle(&mut self, key: Key) -> Step<GroupSelected> {
        let last = self.rows.len() - 1;
        match key {
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
                match self.rows[self.cur] {
                    Row::Header { group_idx } => self.groups[group_idx].toggle_all(),
                    Row::Item {
                        group_idx,
                        item_idx,
                    } => {
                        self.groups[group_idx].items[item_idx].checked ^= true;
                    }
                }
                Step::Continue
            }
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
        let mut f: Frame = Vec::new();
        f.push(theme::label(&self.label));
        f.push(theme::hint(""));
        let mut row_i = 0usize;
        for g in &self.groups {
            let header_active = row_i == self.cur;
            let gbullet = if g.all_checked() {
                paint(OK, "◼")
            } else if g.any_checked() {
                paint(ACCENT, "◼")
            } else {
                paint(DIM, "◻")
            };
            if header_active {
                f.push(format!(
                    "{}  {} {} {}",
                    paint(DIM, "│"),
                    paint(ACCENT, "❯"),
                    gbullet,
                    paint(WHITE, &g.name)
                ));
            } else {
                f.push(format!(
                    "{}    {} {}",
                    paint(DIM, "│"),
                    gbullet,
                    paint(DIM, &g.name)
                ));
            }
            row_i += 1;
            let count = g.items.len();
            for ii in 0..count {
                let it = &g.items[ii];
                let is_last = ii == count - 1;
                let is_cursor = row_i == self.cur;
                let connector = if is_last {
                    paint(DIM, "  └")
                } else {
                    paint(DIM, "  │")
                };
                let bullet = if it.checked {
                    paint(OK, "◼")
                } else {
                    paint(DIM, "◻")
                };
                let hint_sfx = theme::hint_inline(it.hint.as_deref());
                if is_cursor {
                    f.push(format!(
                        "{}  {} {} {} {}{}",
                        paint(DIM, "│"),
                        connector,
                        paint(ACCENT, "❯"),
                        bullet,
                        paint(WHITE, &it.value),
                        hint_sfx
                    ));
                } else {
                    f.push(format!(
                        "{}  {}   {} {}{}",
                        paint(DIM, "│"),
                        connector,
                        bullet,
                        paint(DIM, &it.value),
                        paint(DIM, &hint_sfx)
                    ));
                }
                row_i += 1;
            }
        }
        f.push(theme::hint(
            "Space to toggle · ↑/↓ to navigate · Enter to confirm",
        ));
        f.push(theme::frame_bot(None));
        f
    }

    fn render_answered(&self, value: &GroupSelected) -> Frame {
        let summary = if value.is_empty() {
            "none".to_string()
        } else {
            value
                .items
                .iter()
                .map(|i| i.value.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        };
        theme::answered(&self.label, &summary)
            .split("\r\n")
            .map(String::from)
            .collect()
    }

    fn run_fallback(self) -> Result<GroupSelected> {
        use std::io::Write;
        let mut out = std::io::stderr();
        writeln!(out, "  {}", self.label).map_err(PromptError::Io)?;
        let mut all: Vec<(String, String)> = Vec::new();
        let mut n = 1usize;
        for g in &self.groups {
            writeln!(out, "  [ {} ]", g.name).map_err(PromptError::Io)?;
            for it in &g.items {
                match &it.hint {
                    Some(h) => writeln!(out, "    {}. {} ({})", n, it.value, h),
                    None => writeln!(out, "    {}. {}", n, it.value),
                }
                .map_err(PromptError::Io)?;
                all.push((g.name.clone(), it.value.clone()));
                n += 1;
            }
        }
        write!(out, "  Enter numbers (space-separated): ").map_err(PromptError::Io)?;
        out.flush().map_err(PromptError::Io)?;
        let line = super::engine::fallback::read_line_raw()?;
        let indices: Vec<usize> = line
            .split_whitespace()
            .filter_map(|t| t.parse::<usize>().ok())
            .filter(|&i| i >= 1 && i < n)
            .map(|i| i - 1)
            .collect();
        Ok(GroupSelected {
            items: indices
                .into_iter()
                .map(|i| GroupItem {
                    group: all[i].0.clone(),
                    value: all[i].1.clone(),
                    index: i,
                })
                .collect(),
        })
    }
}

fn build_rows(groups: &[GroupData]) -> Vec<Row> {
    let mut rows = Vec::new();
    for (gi, g) in groups.iter().enumerate() {
        rows.push(Row::Header { group_idx: gi });
        for ii in 0..g.items.len() {
            rows.push(Row::Item {
                group_idx: gi,
                item_idx: ii,
            });
        }
    }
    rows
}
