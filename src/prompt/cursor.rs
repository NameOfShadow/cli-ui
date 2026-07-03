//! In-buffer text cursor — ports `@clack/core`'s `findCursor` / `findTextCursor`.
//!
//! `LineBuf` keeps a single-line string plus a byte-offset cursor, and provides
//! the editing operations that every text-style prompt needs (text, secret,
//! autocomplete query, path, multiline lines).

use crate::styles::{paint, DIM};

#[derive(Default)]
pub struct LineBuf {
    buf: String,
    /// Byte offset into `buf`. Always falls on a char boundary.
    cur: usize,
}

impl LineBuf {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn value(&self) -> &str {
        &self.buf
    }
    pub fn is_empty(&self) -> bool {
        self.buf.is_empty()
    }

    pub fn insert(&mut self, c: char) {
        self.buf.insert(self.cur, c);
        self.cur += c.len_utf8();
    }

    pub fn backspace(&mut self) {
        if self.cur == 0 {
            return;
        }
        let prev = self.buf[..self.cur].chars().next_back().unwrap();
        self.cur -= prev.len_utf8();
        self.buf.remove(self.cur);
    }

    pub fn delete(&mut self) {
        if self.cur >= self.buf.len() {
            return;
        }
        self.buf.remove(self.cur);
    }

    pub fn delete_word_back(&mut self) {
        let start = self.cur;
        // skip trailing whitespace
        while self.cur > 0 {
            let prev = self.buf[..self.cur].chars().next_back().unwrap();
            if !prev.is_whitespace() {
                break;
            }
            self.cur -= prev.len_utf8();
        }
        // delete word chars
        while self.cur > 0 {
            let prev = self.buf[..self.cur].chars().next_back().unwrap();
            if prev.is_whitespace() {
                break;
            }
            self.cur -= prev.len_utf8();
        }
        self.buf.drain(self.cur..start);
    }

    pub fn left(&mut self) {
        if self.cur == 0 {
            return;
        }
        let prev = self.buf[..self.cur].chars().next_back().unwrap();
        self.cur -= prev.len_utf8();
    }
    pub fn right(&mut self) {
        if self.cur >= self.buf.len() {
            return;
        }
        let next = self.buf[self.cur..].chars().next().unwrap();
        self.cur += next.len_utf8();
    }
    pub fn home(&mut self) {
        self.cur = 0;
    }
    pub fn end(&mut self) {
        self.cur = self.buf.len();
    }

    /// Render the buffer with an inverse-video block where the cursor is, so
    /// the user can see exactly where the next character will go. Pulls its
    /// foreground from [`super::settings::colors().input`] so the live text
    /// matches the global palette.
    pub fn with_cursor(&self) -> String {
        let input = super::settings::colors().input;
        if self.buf.is_empty() {
            return paint(input, "\x1b[7m \x1b[27m");
        }
        if self.cur >= self.buf.len() {
            return format!(
                "{}{}",
                paint(input, &self.buf),
                paint(input, "\x1b[7m \x1b[27m")
            );
        }
        let (a, b) = self.buf.split_at(self.cur);
        let next_char_len = b.chars().next().unwrap().len_utf8();
        let (under, rest) = b.split_at(next_char_len);
        format!(
            "{}{}{}",
            paint(input, a),
            paint(input, &format!("\x1b[7m{under}\x1b[27m")),
            paint(input, rest)
        )
    }

    /// Render with a placeholder when the buffer is empty.
    pub fn with_placeholder(&self, placeholder: &str) -> String {
        if self.buf.is_empty() && !placeholder.is_empty() {
            paint(DIM, placeholder)
        } else {
            self.with_cursor()
        }
    }
}
