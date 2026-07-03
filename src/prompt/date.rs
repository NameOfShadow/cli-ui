//! Date picker — built on the [`Prompt`] trait.

use crate::styles::{paint, DIM, WHITE};

use super::core::{Frame, Key, Prompt, RenderCtx, Step};
use super::error::{PromptError, Result};
use super::theme;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Seg {
    Y,
    M,
    D,
}

/// Proleptic Gregorian calendar date — used by [`date()`] and [`DatePrompt`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Date {
    /// Year, e.g. `2026`.
    pub year: i32,
    /// Month, `1..=12`.
    pub month: u32,
    /// Day of month, `1..=31` (clamped to the month's actual length).
    pub day: u32,
}

impl Date {
    /// Today, computed from `SystemTime::now()` in UTC.
    pub fn today() -> Self {
        let secs = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        from_unix_days((secs / 86_400) as i64)
    }
    /// `yyyy-mm-dd` string, e.g. `2026-06-30`.
    pub fn iso(&self) -> String {
        format!("{:04}-{:02}-{:02}", self.year, self.month, self.day)
    }
    fn days_in_month(&self) -> u32 {
        match self.month {
            1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
            4 | 6 | 9 | 11 => 30,
            2 => {
                if is_leap(self.year) {
                    29
                } else {
                    28
                }
            }
            _ => 30,
        }
    }
    fn clamp(&mut self) {
        if self.month < 1 {
            self.month = 12;
            self.year -= 1;
        }
        if self.month > 12 {
            self.month = 1;
            self.year += 1;
        }
        let max = self.days_in_month();
        if self.day < 1 {
            self.day = max;
        }
        if self.day > max {
            self.day = max;
        }
    }
}

fn is_leap(y: i32) -> bool {
    (y % 4 == 0 && y % 100 != 0) || y % 400 == 0
}

fn from_unix_days(mut days: i64) -> Date {
    let mut y = 1970i32;
    loop {
        let in_year = if is_leap(y) { 366 } else { 365 };
        if days < in_year as i64 {
            break;
        }
        days -= in_year as i64;
        y += 1;
    }
    let mut m = 1u32;
    let months_len = |year: i32, mo: u32| -> i64 {
        match mo {
            1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
            4 | 6 | 9 | 11 => 30,
            2 => {
                if is_leap(year) {
                    29
                } else {
                    28
                }
            }
            _ => 0,
        }
    };
    while days >= months_len(y, m) {
        days -= months_len(y, m);
        m += 1;
    }
    Date {
        year: y,
        month: m,
        day: days as u32 + 1,
    }
}

/// Builder returned by [`date()`].
pub struct DatePrompt {
    label: String,
    value: Date,
    min: Option<Date>,
    max: Option<Date>,
    seg: Seg,
}

/// Pick a date with three editable segments (year / month / day). Navigate
/// segments with ←/→, adjust with ↑/↓.
///
/// ```no_run
/// use cli_ui::prompt::date::date;
///
/// let birthday = date("When were you born?").run()?;
/// # Ok::<(), cli_ui::prompt::PromptError>(())
/// ```
pub fn date(label: impl Into<String>) -> DatePrompt {
    DatePrompt {
        label: label.into(),
        value: Date::today(),
        min: None,
        max: None,
        seg: Seg::Y,
    }
}

impl DatePrompt {
    /// Initial date shown (default: today).
    pub fn initial(mut self, d: Date) -> Self {
        self.value = d;
        self
    }
    /// Reject dates earlier than `d`.
    pub fn min(mut self, d: Date) -> Self {
        self.min = Some(d);
        self
    }
    /// Reject dates later than `d`.
    pub fn max(mut self, d: Date) -> Self {
        self.max = Some(d);
        self
    }

    /// Run the prompt to completion.
    pub fn run(self) -> Result<Date> {
        super::core::run(self)
    }

    fn within_range(&self) -> bool {
        let iso = self.value.iso();
        if let Some(ref m) = self.min {
            if iso < m.iso() {
                return false;
            }
        }
        if let Some(ref m) = self.max {
            if iso > m.iso() {
                return false;
            }
        }
        true
    }
}

impl Prompt for DatePrompt {
    type Output = Date;

    fn handle(&mut self, key: Key) -> Step<Date> {
        match key {
            Key::Left => {
                self.seg = match self.seg {
                    Seg::Y => Seg::D,
                    Seg::M => Seg::Y,
                    Seg::D => Seg::M,
                };
                Step::Continue
            }
            Key::Right => {
                self.seg = match self.seg {
                    Seg::Y => Seg::M,
                    Seg::M => Seg::D,
                    Seg::D => Seg::Y,
                };
                Step::Continue
            }
            Key::Up => {
                adjust(&mut self.value, self.seg, 1);
                self.value.clamp();
                Step::Continue
            }
            Key::Down => {
                adjust(&mut self.value, self.seg, -1);
                self.value.clamp();
                Step::Continue
            }
            Key::Enter => {
                if !self.within_range() {
                    Step::Reject("Date out of range".into())
                } else {
                    Step::Submit(self.value)
                }
            }
            Key::Escape | Key::Interrupt => Step::Cancel,
            _ => Step::Continue,
        }
    }

    fn render(&self, _ctx: RenderCtx) -> Frame {
        let mut f: Frame = Vec::with_capacity(4);
        f.push(theme::label(&self.label));
        let seg_y = format_seg(format!("{:04}", self.value.year), self.seg == Seg::Y);
        let seg_m = format_seg(format!("{:02}", self.value.month), self.seg == Seg::M);
        let seg_d = format_seg(format!("{:02}", self.value.day), self.seg == Seg::D);
        let dash = paint(DIM, "-");
        f.push(format!(
            "{}  {}{dash}{}{dash}{}",
            paint(DIM, "│"),
            seg_y,
            seg_m,
            seg_d
        ));
        f.push(theme::hint("← → segment · ↑ ↓ adjust · Enter to confirm"));
        f.push(theme::frame_bot(None));
        let _ = WHITE;
        f
    }

    fn render_answered(&self, value: &Date) -> Frame {
        theme::answered(&self.label, &value.iso())
            .split("\r\n")
            .map(String::from)
            .collect()
    }

    fn run_fallback(self) -> Result<Date> {
        use std::io::Write;
        let mut out = std::io::stderr();
        write!(
            out,
            "  {} [yyyy-mm-dd, default {}]: ",
            self.label,
            self.value.iso()
        )
        .map_err(PromptError::Io)?;
        out.flush().map_err(PromptError::Io)?;
        let line = super::engine::fallback::read_line_raw()?;
        if line.is_empty() {
            return Ok(self.value);
        }
        let parts: Vec<&str> = line.split('-').collect();
        if parts.len() != 3 {
            return Err(PromptError::Interrupted);
        }
        Ok(Date {
            year: parts[0].parse().unwrap_or(self.value.year),
            month: parts[1].parse().unwrap_or(self.value.month),
            day: parts[2].parse().unwrap_or(self.value.day),
        })
    }
}

fn adjust(d: &mut Date, seg: Seg, by: i32) {
    match seg {
        Seg::Y => d.year = (d.year + by).max(1),
        Seg::M => d.month = (d.month as i32 + by).clamp(1, 12) as u32,
        Seg::D => d.day = (d.day as i32 + by).max(1) as u32,
    }
}

fn format_seg(value: String, active: bool) -> String {
    let c = super::settings::colors();
    if active {
        paint(c.accent, &format!("\x1b[7m{value}\x1b[27m"))
    } else {
        paint(c.input, &value)
    }
}
