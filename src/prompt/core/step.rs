//! Outcome of a single key press inside a [`Prompt`](super::Prompt).

/// The transition a prompt requests after handling one key.
#[derive(Debug)]
pub enum Step<T> {
    /// State changed but we're not finished — keep reading keys.
    Continue,
    /// Done; deliver `T` to the caller.
    Submit(T),
    /// User asked to abort (Esc, Ctrl-C, …). Runner converts this into
    /// [`PromptError::Interrupted`](super::super::error::PromptError::Interrupted).
    Cancel,
    /// Block submission and display `msg` under the input line in the
    /// runner's error style. The next non-Enter key clears the error.
    Reject(String),
}
