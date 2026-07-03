#![allow(missing_docs)]
//! Bordered text block — ports `@clack/prompts` `box()`.
//!
//! Renders a configurable rounded or square box with a title and aligned body.

use crate::styles::{paint, DIM, WHITE};

#[derive(Clone, Copy)]
pub enum Align {
    Left,
    Center,
    Right,
}

pub struct BoxOptions {
    pub title: String,
    pub body: String,
    pub title_align: Align,
    pub content_align: Align,
    pub title_padding: usize,
    pub content_padding: usize,
    pub width: Option<usize>,
    pub rounded: bool,
}

pub fn boxed(body: impl Into<String>, title: impl Into<String>) -> BoxOptions {
    BoxOptions {
        title: title.into(),
        body: body.into(),
        title_align: Align::Left,
        content_align: Align::Left,
        title_padding: 1,
        content_padding: 2,
        width: None,
        rounded: true,
    }
}

impl BoxOptions {
    pub fn title_align(mut self, a: Align) -> Self {
        self.title_align = a;
        self
    }
    pub fn content_align(mut self, a: Align) -> Self {
        self.content_align = a;
        self
    }
    pub fn title_padding(mut self, n: usize) -> Self {
        self.title_padding = n;
        self
    }
    pub fn content_padding(mut self, n: usize) -> Self {
        self.content_padding = n;
        self
    }
    pub fn width(mut self, n: usize) -> Self {
        self.width = Some(n);
        self
    }
    pub fn rounded(mut self, v: bool) -> Self {
        self.rounded = v;
        self
    }

    /// Print the box to stderr.
    pub fn print(&self) {
        eprintln!("{}", self.render());
    }

    pub fn render(&self) -> String {
        let (tl, tr, bl, br) = if self.rounded {
            ("╭", "╮", "╰", "╯")
        } else {
            ("┌", "┐", "└", "┘")
        };
        let h = "─";
        let v = "│";

        let body_lines: Vec<&str> = self.body.lines().collect();
        let longest = body_lines
            .iter()
            .map(|l| l.chars().count())
            .max()
            .unwrap_or(0);
        let title_len = self.title.chars().count();
        let inner = self.width.unwrap_or_else(|| {
            longest.max(title_len + self.title_padding * 2) + self.content_padding * 2
        });

        let mut out = String::new();
        // top border with title
        let (tl_pad, tr_pad) = pad_for(title_len, inner, self.title_padding, self.title_align);
        out.push_str(&paint(DIM, tl));
        out.push_str(&paint(DIM, &h.repeat(tl_pad)));
        out.push_str(&paint(WHITE, &self.title));
        out.push_str(&paint(DIM, &h.repeat(tr_pad)));
        out.push_str(&paint(DIM, tr));
        out.push('\n');

        // body
        for line in &body_lines {
            let len = line.chars().count();
            let (lp, rp) = pad_for(len, inner, self.content_padding, self.content_align);
            out.push_str(&paint(DIM, v));
            out.push_str(&" ".repeat(lp));
            out.push_str(line);
            out.push_str(&" ".repeat(rp));
            out.push_str(&paint(DIM, v));
            out.push('\n');
        }
        if body_lines.is_empty() {
            out.push_str(&paint(DIM, v));
            out.push_str(&" ".repeat(inner));
            out.push_str(&paint(DIM, v));
            out.push('\n');
        }

        // bottom
        out.push_str(&paint(DIM, bl));
        out.push_str(&paint(DIM, &h.repeat(inner)));
        out.push_str(&paint(DIM, br));
        out
    }
}

fn pad_for(len: usize, inner: usize, default_pad: usize, align: Align) -> (usize, usize) {
    let left = match align {
        Align::Left => default_pad,
        Align::Center => inner.saturating_sub(len) / 2,
        Align::Right => inner.saturating_sub(len + default_pad),
    };
    let right = inner.saturating_sub(left + len);
    (left, right)
}
