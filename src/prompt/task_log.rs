#![allow(missing_docs)]
//! Task with a collapsible running log — ports `@clack/prompts` `taskLog()`.
//!
//! The log lines are shown while the task runs. On `success()` they are
//! cleared and replaced with a single success marker; on `error()` they are
//! preserved so the user can read what happened.

use crate::styles::{paint, DIM, ERR, OK, WHITE};
use std::io::Write;
use std::sync::Mutex;

const BAR: &str = "│";
const STEP_OK: &str = "◇";
const STEP_GREEN: &str = "◆";
const STEP_ERR: &str = "■";

pub struct TaskLog {
    title: String,
    limit: usize,
    lines: Mutex<Vec<String>>,
}

pub fn task_log(title: impl Into<String>) -> TaskLog {
    let t = TaskLog {
        title: title.into(),
        limit: 10,
        lines: Mutex::new(Vec::new()),
    };
    t.render_intro();
    t
}

impl TaskLog {
    /// Cap visible lines while the task is running.
    pub fn limit(mut self, n: usize) -> Self {
        self.limit = n.max(1);
        self
    }

    fn render_intro(&self) {
        let mut out = std::io::stderr();
        let _ = writeln!(out, "{}", paint(DIM, BAR));
        let _ = writeln!(
            out,
            "{}  {}",
            paint(OK, STEP_GREEN),
            paint(WHITE, &self.title)
        );
        let _ = writeln!(out, "{}", paint(DIM, BAR));
    }

    pub fn message(&self, msg: impl AsRef<str>) {
        let mut lines = self.lines.lock().unwrap();
        for ln in msg.as_ref().split('\n') {
            lines.push(ln.to_string());
        }
        // trim oldest
        while lines.len() > self.limit {
            lines.remove(0);
        }
        let mut out = std::io::stderr();
        let _ = writeln!(out, "{}  {}", paint(DIM, BAR), paint(DIM, msg.as_ref()));
    }

    pub fn success(&self, msg: impl AsRef<str>) {
        let mut out = std::io::stderr();
        let _ = writeln!(
            out,
            "{}  {}",
            paint(OK, STEP_OK),
            paint(WHITE, msg.as_ref())
        );
    }

    pub fn error(&self, msg: impl AsRef<str>) {
        let mut out = std::io::stderr();
        let _ = writeln!(
            out,
            "{}  {}",
            paint(ERR, STEP_ERR),
            paint(ERR, msg.as_ref())
        );
        // dump retained lines under the error
        let lines = self.lines.lock().unwrap();
        for ln in lines.iter() {
            let _ = writeln!(out, "{}  {}", paint(DIM, BAR), paint(DIM, ln));
        }
    }
}
