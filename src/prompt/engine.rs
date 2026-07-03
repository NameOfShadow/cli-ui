#![allow(missing_docs)]
//! Input engine — interactive (crossterm) or fallback (numeric stdin).

use super::error::{PromptError, Result};

pub fn is_interactive() -> bool {
    use std::io::IsTerminal;
    std::io::stderr().is_terminal() && std::io::stdin().is_terminal()
}

// ── Key enum ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum Key {
    Up,
    Down,
    Left,
    Right,
    Home,
    End,
    PageUp,
    PageDown,
    Enter,
    Tab,
    /// Any printable character. Space arrives here as `Char(' ')`.
    Char(char),
    Backspace,
    Delete,
    Escape,
    /// Ctrl+C or Ctrl+D — triggers Interrupted error
    Interrupt,
    /// Ctrl+W — delete word backward
    DeleteWordBack,
}

// ── Interactive engine ────────────────────────────────────────────────────────

pub mod interactive {
    use super::{Key, PromptError, Result};
    use crossterm::event::{self, Event, KeyCode, KeyModifiers};

    pub fn read_key() -> Result<Key> {
        loop {
            match event::read().map_err(PromptError::Io)? {
                Event::Key(ke) => {
                    if ke.modifiers.contains(KeyModifiers::CONTROL) {
                        match ke.code {
                            KeyCode::Char('c') | KeyCode::Char('d') => {
                                return Ok(Key::Interrupt);
                            }
                            KeyCode::Char('w') => return Ok(Key::DeleteWordBack),
                            KeyCode::Char('a') => return Ok(Key::Home),
                            KeyCode::Char('e') => return Ok(Key::End),
                            _ => continue,
                        }
                    }
                    return Ok(match ke.code {
                        KeyCode::Up => Key::Up,
                        KeyCode::Down => Key::Down,
                        KeyCode::Left => Key::Left,
                        KeyCode::Right => Key::Right,
                        KeyCode::Home => Key::Home,
                        KeyCode::End => Key::End,
                        KeyCode::PageUp => Key::PageUp,
                        KeyCode::PageDown => Key::PageDown,
                        KeyCode::Enter => Key::Enter,
                        KeyCode::Tab => Key::Tab,
                        KeyCode::Char(c) => Key::Char(c),
                        KeyCode::Backspace => Key::Backspace,
                        KeyCode::Delete => Key::Delete,
                        KeyCode::Esc => Key::Escape,
                        _ => continue,
                    });
                }
                _ => continue,
            }
        }
    }

    pub fn enter_raw() -> Result<()> {
        crossterm::terminal::enable_raw_mode().map_err(PromptError::Io)
    }

    pub fn leave_raw() -> Result<()> {
        crossterm::terminal::disable_raw_mode().map_err(PromptError::Io)
    }
}

// ── Fallback engine ───────────────────────────────────────────────────────────

pub mod fallback {
    use super::{PromptError, Result};
    use std::io::BufRead;

    pub fn read_line_raw() -> Result<String> {
        let mut line = String::new();
        std::io::stdin()
            .lock()
            .read_line(&mut line)
            .map_err(PromptError::Io)?;
        if line.is_empty() {
            return Err(PromptError::Interrupted);
        }
        Ok(line.trim().to_string())
    }
}
