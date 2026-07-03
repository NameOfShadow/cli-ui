//! Core abstractions every prompt implements.
//!
//! A prompt is three small things:
//!
//! ```text
//! impl Prompt for MyPrompt {
//!     type Output = String;
//!     fn handle(&mut self, key: Key) -> Step<String> { ... }
//!     fn render(&self, ctx: RenderCtx) -> Frame      { ... }
//!     fn render_answered(&self, v: &String) -> Frame { ... }
//! }
//! ```
//!
//! The generic [`run`] function in `core::runner` owns everything
//! else: raw mode, the key loop, line-count bookkeeping, the validate
//! → success transition, the answered redraw, and Ctrl+C cleanup. Prompts
//! never touch crossterm or stderr directly.

mod frame;
mod runner;
mod step;

pub use frame::{Frame, RenderCtx};
pub use runner::run;
pub use step::Step;

pub use super::engine::Key;
use super::error::Result;

/// The contract every interactive prompt implements.
pub trait Prompt {
    /// The value returned to the caller on successful submission.
    type Output;

    /// Apply a key press to the prompt's internal state. Return:
    ///
    /// * [`Step::Continue`] — state changed, but we're not done.
    /// * `Step::Submit(v)` — done, hand `v` back to the caller.
    /// * `Step::Reject(msg)` — block submission, show `msg` as an error
    ///   below the input. The runner clears the error when the next key
    ///   arrives.
    /// * [`Step::Cancel`] — user asked to abort (Esc / Ctrl-C).
    fn handle(&mut self, key: Key) -> Step<Self::Output>;

    /// Render the prompt's current state as one frame (a `Vec<String>`,
    /// one entry per terminal row). The runner takes care of writing the
    /// frame and clearing it on the next iteration — prompts never call
    /// `eprintln!` themselves.
    fn render(&self, ctx: RenderCtx) -> Frame;

    /// Render the prompt's post-submission display. Typically the
    /// `◇  question / │  value / │` triple from `theme::answered`.
    fn render_answered(&self, value: &Self::Output) -> Frame;

    /// Optional non-interactive fallback for piped/CI environments. The
    /// default is to refuse — most prompts override this with a numeric
    /// or line-buffered alternative.
    fn run_fallback(self) -> Result<Self::Output>
    where
        Self: Sized,
    {
        Err(super::error::PromptError::Interrupted)
    }
}
