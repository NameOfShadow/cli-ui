#![allow(missing_docs)]
//! Progress bar — ports `@clack/prompts` `progress()`.
//!
//! Built on top of the spinner: the bar is rendered into the spinner's message
//! line, so it animates the bullet character while the user calls
//! [`Progress::advance`] from another thread or in a loop.

use super::spinner::{spinner, Spinner};
use crate::styles::{paint, DIM};

#[derive(Clone, Copy)]
pub enum Style {
    Light,
    Heavy,
    Block,
}

impl Style {
    fn ch(self) -> &'static str {
        match self {
            Style::Light => "─",
            Style::Heavy => "━",
            Style::Block => "█",
        }
    }
}

pub struct Progress {
    spin: Spinner,
    style: Style,
    size: usize,
    max: u64,
    value: std::sync::Mutex<u64>,
    msg: std::sync::Mutex<String>,
}

pub fn progress() -> Progress {
    Progress {
        spin: spinner(),
        style: Style::Heavy,
        size: 40,
        max: 100,
        value: std::sync::Mutex::new(0),
        msg: std::sync::Mutex::new(String::new()),
    }
}

impl Progress {
    pub fn style(mut self, s: Style) -> Self {
        self.style = s;
        self
    }
    pub fn size(mut self, n: usize) -> Self {
        self.size = n.max(1);
        self
    }
    pub fn max(mut self, n: u64) -> Self {
        self.max = n.max(1);
        self
    }

    pub fn start(&self, msg: impl Into<String>) {
        let m = msg.into();
        *self.msg.lock().unwrap() = m.clone();
        self.spin.start(self.render(&m));
    }

    pub fn advance(&self, step: u64, msg: Option<String>) {
        let mut v = self.value.lock().unwrap();
        *v = (*v + step).min(self.max);
        let value = *v;
        drop(v);
        if let Some(m) = msg {
            *self.msg.lock().unwrap() = m;
        }
        let m = self.msg.lock().unwrap().clone();
        self.spin.message(self.render_with(&m, value));
    }

    pub fn message(&self, msg: impl Into<String>) {
        self.advance(0, Some(msg.into()));
    }

    pub fn stop(&self, msg: impl Into<String>) {
        self.spin.stop(msg);
    }
    pub fn cancel(&self, msg: impl Into<String>) {
        self.spin.cancel(msg);
    }
    pub fn error(&self, msg: impl Into<String>) {
        self.spin.error(msg);
    }

    fn render(&self, msg: &str) -> String {
        self.render_with(msg, *self.value.lock().unwrap())
    }
    fn render_with(&self, msg: &str, value: u64) -> String {
        let filled = ((value as usize) * self.size) / (self.max as usize);
        let ch = self.style.ch();
        let on = ch.repeat(filled);
        let off = ch.repeat(self.size - filled);
        format!(
            "{}{} {}",
            paint(super::settings::colors().accent, &on),
            paint(DIM, &off),
            msg
        )
    }
}
