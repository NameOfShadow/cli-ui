//! Filesystem path picker — built on the [`Prompt`] trait.

use crate::styles::{paint, DIM, WHITE};
use std::path::{Path, PathBuf};

use super::core::{Frame, Key, Prompt, RenderCtx, Step};
use super::error::{PromptError, Result};
use super::theme;

/// Builder returned by [`path()`].
pub struct PathPrompt {
    label: String,
    initial: Option<String>,
    directory: bool,
    max_items: usize,
    buf: String,
    cur: usize,
}

/// Pick a filesystem path with Tab completion. Defaults to the current
/// working directory.
///
/// ```no_run
/// use cli_ui::prompt::path::path;
///
/// let project = path("Where to scaffold?").directory(true).run()?;
/// # Ok::<(), cli_ui::prompt::PromptError>(())
/// ```
pub fn path(label: impl Into<String>) -> PathPrompt {
    PathPrompt {
        label: label.into(),
        initial: None,
        directory: false,
        max_items: 8,
        buf: String::new(),
        cur: 0,
    }
}

impl PathPrompt {
    /// Initial path shown — falls back to the current working directory.
    pub fn initial(mut self, v: impl Into<String>) -> Self {
        self.initial = Some(v.into());
        self
    }
    /// When `true`, only accept directories.
    pub fn directory(mut self, v: bool) -> Self {
        self.directory = v;
        self
    }
    /// Maximum number of completion suggestions shown at once.
    pub fn max_items(mut self, n: usize) -> Self {
        self.max_items = n.max(1);
        self
    }

    /// Run the prompt to completion.
    pub fn run(mut self) -> Result<PathBuf> {
        self.buf = self.initial.clone().unwrap_or_else(|| {
            std::env::current_dir()
                .map(|p| p.to_string_lossy().into_owned())
                .unwrap_or_default()
        });
        if !self.buf.ends_with('/') {
            self.buf.push('/');
        }
        super::core::run(self)
    }

    fn entries(&self) -> Vec<Entry> {
        let (dir, fragment) = split_path(&self.buf);
        let mut out = Vec::new();
        if let Ok(rd) = std::fs::read_dir(&dir) {
            for ent in rd.flatten() {
                let name = ent.file_name().to_string_lossy().into_owned();
                if !name.starts_with(fragment) {
                    continue;
                }
                let is_dir = ent.file_type().map(|t| t.is_dir()).unwrap_or(false);
                if self.directory && !is_dir {
                    continue;
                }
                out.push(Entry {
                    name,
                    path: ent.path(),
                    is_dir,
                });
            }
        }
        out.sort_by(|a, b| b.is_dir.cmp(&a.is_dir).then(a.name.cmp(&b.name)));
        out.truncate(self.max_items);
        out
    }
}

impl Prompt for PathPrompt {
    type Output = PathBuf;

    fn handle(&mut self, key: Key) -> Step<PathBuf> {
        match key {
            Key::Char('\t') => {
                let entries = self.entries();
                if let Some(e) = entries.get(self.cur) {
                    self.buf = e.path.to_string_lossy().into_owned();
                    if e.is_dir {
                        self.buf.push('/');
                    }
                    self.cur = 0;
                }
                Step::Continue
            }
            Key::Char(c) => {
                self.buf.push(c);
                self.cur = 0;
                Step::Continue
            }
            Key::Backspace => {
                self.buf.pop();
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
                if self.cur + 1 < self.entries().len() {
                    self.cur += 1;
                }
                Step::Continue
            }
            Key::Enter => {
                let p = PathBuf::from(&self.buf);
                if self.directory && !p.is_dir() {
                    return Step::Reject("Not a directory".into());
                }
                Step::Submit(p)
            }
            Key::Escape | Key::Interrupt => Step::Cancel,
            _ => Step::Continue,
        }
    }

    fn render(&self, _ctx: RenderCtx) -> Frame {
        let c = super::settings::colors();
        let mut f: Frame = Vec::new();
        f.push(theme::label(&self.label));
        f.push(format!(
            "{}  {}",
            paint(c.dim, "│"),
            paint(c.input, &self.buf)
        ));
        let entries = self.entries();
        if entries.is_empty() {
            f.push(format!(
                "{}  {}",
                paint(c.dim, "│"),
                paint(c.dim, "(no matches)")
            ));
        } else {
            for (i, e) in entries.iter().enumerate() {
                let active = i == self.cur;
                let glyph = if e.is_dir { "/" } else { " " };
                let style = if active { WHITE } else { DIM };
                let prefix = if active {
                    paint(c.accent, "❯")
                } else {
                    paint(c.dim, " ")
                };
                f.push(format!(
                    "{}  {} {}{}",
                    paint(c.dim, "│"),
                    prefix,
                    paint(style, &e.name),
                    paint(c.dim, glyph)
                ));
            }
        }
        f.push(theme::hint(
            "Tab to complete · ↑ ↓ to navigate · Enter to confirm",
        ));
        f.push(theme::frame_bot(None));
        f
    }

    fn render_answered(&self, value: &PathBuf) -> Frame {
        theme::answered(&self.label, &value.display().to_string())
            .split("\r\n")
            .map(String::from)
            .collect()
    }

    fn run_fallback(self) -> Result<PathBuf> {
        use std::io::Write;
        let mut out = std::io::stderr();
        write!(out, "  {}: ", self.label).map_err(PromptError::Io)?;
        out.flush().map_err(PromptError::Io)?;
        let line = super::engine::fallback::read_line_raw()?;
        Ok(PathBuf::from(line))
    }
}

struct Entry {
    name: String,
    path: PathBuf,
    is_dir: bool,
}

fn split_path(buf: &str) -> (PathBuf, &str) {
    if let Some(pos) = buf.rfind('/') {
        let (dir, rest) = buf.split_at(pos + 1);
        let dir = if dir.is_empty() {
            "/".into()
        } else {
            PathBuf::from(dir)
        };
        (dir, rest)
    } else {
        (Path::new(".").to_path_buf(), buf)
    }
}
