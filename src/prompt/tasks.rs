#![allow(missing_docs)]
//! Sequential task runner — ports `@clack/prompts` `tasks()`.
//!
//! Each task gets its own spinner and runs in order; a task may return a final
//! message that replaces its title on success.
//!
//! # Example
//! ```rust,no_run
//! use cli_ui::prompt::{tasks, Task};
//!
//! tasks(vec![
//!     Task::new("Linting", |s| {
//!         std::thread::sleep(std::time::Duration::from_millis(400));
//!         s.message("Checking imports");
//!         std::thread::sleep(std::time::Duration::from_millis(400));
//!         Some("Lint clean".into())
//!     }),
//!     Task::new("Testing", |_| {
//!         std::thread::sleep(std::time::Duration::from_millis(500));
//!         Some("42 passing".into())
//!     }),
//! ]);
//! ```

use super::spinner::{spinner, Spinner};

type TaskFn = Box<dyn FnOnce(&Spinner) -> Option<String> + Send + 'static>;

pub struct Task {
    title: String,
    body: TaskFn,
    enabled: bool,
}

impl Task {
    pub fn new<F>(title: impl Into<String>, body: F) -> Self
    where
        F: FnOnce(&Spinner) -> Option<String> + Send + 'static,
    {
        Self {
            title: title.into(),
            body: Box::new(body),
            enabled: true,
        }
    }

    /// Skip this task when `false`.
    pub fn enabled(mut self, v: bool) -> Self {
        self.enabled = v;
        self
    }
}

/// Run a list of tasks in order, each with its own spinner.
pub fn tasks(items: Vec<Task>) {
    for t in items {
        if !t.enabled {
            continue;
        }
        let s = spinner();
        s.start(&t.title);
        let result = (t.body)(&s);
        s.stop(result.unwrap_or(t.title));
    }
}
