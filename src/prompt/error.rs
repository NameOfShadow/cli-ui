//! Error type for prompt operations.

use std::fmt;

/// Error returned by prompt `.run()` methods.
#[derive(Debug)]
pub enum PromptError {
    /// User pressed Ctrl+C or Ctrl+D.
    Interrupted,
    /// I/O error reading from stdin or writing to stderr.
    Io(std::io::Error),
}

impl fmt::Display for PromptError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Interrupted => write!(f, "prompt interrupted by user"),
            Self::Io(e) => write!(f, "prompt I/O error: {e}"),
        }
    }
}

impl std::error::Error for PromptError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(e) => Some(e),
            _ => None,
        }
    }
}

impl From<std::io::Error> for PromptError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

/// Result alias for prompt operations.
pub type Result<T> = std::result::Result<T, PromptError>;

/// `true` when the error came from Ctrl+C / Ctrl+D / Escape.
pub fn is_cancel(err: &PromptError) -> bool {
    matches!(err, PromptError::Interrupted)
}

/// Extension trait: gracefully end the process when the user cancels a prompt.
///
/// `or_cancel(msg)` prints `msg` (or the global [`settings::cancel`](super::settings::Settings::cancel)
/// when omitted) using the framed `■  …` style, then `exit(0)`. Any other
/// error is unwrapped — matching how clack's `isCancel` flow is usually used.
///
/// # Example
/// ```no_run
/// use cli_ui::prompt::{text, OnCancel};
///
/// let name = text("Your name").run().or_cancel("See you next time.");
/// println!("hello {name}");
/// ```
pub trait OnCancel<T> {
    /// Exit cleanly on cancellation with the given message.
    fn or_cancel(self, message: &str) -> T;

    /// Exit cleanly on cancellation using the global default message
    /// from [`super::settings`].
    fn or_cancel_default(self) -> T;
}

impl<T> OnCancel<T> for Result<T> {
    fn or_cancel(self, message: &str) -> T {
        match self {
            Ok(v) => v,
            Err(PromptError::Interrupted) => {
                super::cancel(message);
                std::process::exit(0);
            }
            Err(e) => panic!("{e}"),
        }
    }
    fn or_cancel_default(self) -> T {
        let msg = super::settings::get().cancel;
        self.or_cancel(&msg)
    }
}
