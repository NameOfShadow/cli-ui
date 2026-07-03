//! Interactive prompts — clack/prompts-style.
//!
//! # Hello, prompt
//!
//! ```no_run
//! use cli_ui::prompt::{intro, outro, text, confirm, OnCancel};
//!
//! fn main() {
//!     intro("Set up a new project");
//!
//!     let name = text("What's your name?")
//!         .placeholder("Anya")
//!         .run()
//!         .or_cancel("Cancelled.");
//!
//!     let public = confirm("Make it public?")
//!         .default(true)
//!         .run()
//!         .or_cancel("Cancelled.");
//!
//!     outro(format!("Hello {name}! profile is {}.",
//!         if public { "public" } else { "private" }));
//! }
//! ```
//!
//! # The mental model
//!
//! Every prompt is one of three things:
//!
//! 1. **A builder** — `text("…")`, `select("…")`, etc. Chain `.placeholder()`,
//!    `.default()`, `.validate()`, `.option()` etc. Always finish with `.run()`.
//! 2. **A frame helper** — [`intro`], [`outro`], [`note`], [`cancel`],
//!    [`boxed::boxed`]. They print a single framed line/block and return
//!    immediately. Use them to surround your prompt flow.
//! 3. **A long-running task display** — [`spinner`](fn@spinner), [`progress`],
//!    [`tasks`](fn@tasks), [`task_log`](fn@task_log). These run in the foreground
//!    while non-prompt work happens.
//!
//! Everything renders to **stderr** so your prompts don't pollute pipes.
//!
//! # The four categories at a glance
//!
//! ## Question prompts
//!
//! | Prompt            | Picks                                |
//! |-------------------|--------------------------------------|
//! | [`text`](fn@text)          | A line of text                       |
//! | [`secret`]        | A masked line (password, API key)    |
//! | [`multiline`](fn@multiline)     | Several lines                        |
//! | [`confirm`](fn@confirm)       | Yes / No                             |
//! | [`select`](fn@select)        | One option from a list               |
//! | [`multiselect`](fn@multiselect)   | Many options from a list             |
//! | [`groupmultiselect`](fn@groupmultiselect) | Many options from grouped lists |
//! | [`autocomplete`](fn@autocomplete)  | One option, filtered by typing       |
//! | [`select_key`](fn@select_key)    | One option by single keypress        |
//! | [`date::date`]    | A `yyyy-mm-dd` date                  |
//! | [`path::path`]    | A filesystem path with completion    |
//!
//! ## Framing
//!
//! [`intro`] / [`outro`] / [`note`] / [`cancel`] / [`boxed::boxed`] /
//! [`log`] / [`stream`].
//!
//! ## Live work
//!
//! [`spinner`](fn@spinner) / [`progress`] / [`tasks`](fn@tasks) / [`task_log`](fn@task_log).
//!
//! ## Composition
//!
//! [`group::group`] runs many prompts sequentially and collects their answers.
//!
//! # Validation
//!
//! Validators are promoted to the prompt root so they import like any
//! other constructor:
//!
//! ```no_run
//! use cli_ui::prompt::{text, min_chars, has_upper, has_digit};
//!
//! let pw = text("Password")
//!     .rule(min_chars(12).and(has_upper()).and(has_digit()))
//!     .run();
//! ```
//!
//! # Customisation
//!
//! Theme everything in one call via [`settings::update_colors`]:
//!
//! ```
//! cli_ui::prompt::settings::update_colors(|c| {
//!     c.accent = anstyle::Style::new()
//!         .fg_color(Some(anstyle::Color::Ansi(anstyle::AnsiColor::Magenta)))
//!         .bold();
//! });
//! ```
//!
//! # Cancellation
//!
//! When the user hits Ctrl-C or Esc, `.run()` returns
//! `Err(PromptError::Interrupted)`. The [`OnCancel`] extension trait turns
//! that into a clean exit:
//!
//! ```no_run
//! use cli_ui::prompt::{text, OnCancel};
//! let name = text("Your name").run().or_cancel("Aborted — see you next time.");
//! ```
//!
//! # Extending
//!
//! Implement [`core::Prompt`] to ship your own prompt — see the existing
//! prompts in this module for templates. The runner handles raw mode,
//! cleanup, validation transitions, and the answered redraw for you.
//!
//! # One-line import
//!
//! For quick scripts and examples, glob-import the prelude:
//!
//! ```
//! use cli_ui::prompt::prelude::*;
//! ```

pub mod core;
mod cursor;
mod engine;
mod error;
mod limit_options;
pub mod settings;
mod theme;
pub mod validate;

mod render;

pub mod autocomplete;
pub mod boxed;
pub mod confirm;
pub mod date;
pub mod group;
pub mod groupmultiselect;
pub mod log;
pub mod multiline;
pub mod multiselect;
pub mod path;
pub mod progress_bar;
pub mod select;
pub mod select_key;
pub mod spinner;
pub mod stream;
pub mod task_log;
pub mod tasks;
pub mod text;

pub use autocomplete::{autocomplete, AutocompleteSelected};
pub use boxed::{boxed, Align as BoxAlign, BoxOptions};
pub use confirm::confirm;
pub use date::{date, Date, DatePrompt};
pub use group::group;
pub use groupmultiselect::{groupmultiselect, GroupItem, GroupSelected};
pub use multiline::multiline;
pub use multiselect::{multiselect, MultiSelected};
pub use path::path;
pub use progress_bar::{progress, Progress, Style as ProgressStyle};
pub use select::{select, Selected};
pub use select_key::{select_key, KeySelected};
pub use spinner::{spinner, Spinner};
pub use task_log::{task_log, TaskLog};
pub use tasks::{tasks, Task};
pub use text::{secret, text};

pub use error::{is_cancel, OnCancel, PromptError, Result};

// ── Promoted from `validate::*` ───────────────────────────────────────────────
//
// Every common rule and the `Validator` type sit at the prompt root so users
// can compose without the `validate::` prefix:
//
//     use cli_ui::prompt::{text, min_chars, has_upper};
//     text("pw").rule(min_chars(8).and(has_upper())).run()?;
//
// The full `validate` module is still re-exported for users who want every
// helper at once.
pub use validate::{
    alpha_only, alphanumeric, email, ends_with, exact_chars, float_between, forbid_chars,
    has_digit, has_lower, has_special, has_upper, int_between, max_chars, min_chars, one_of,
    only_chars, required, starts_with, word_count, words_between, Validate, Validator,
};

// ── Promoted from `settings::*` ──────────────────────────────────────────────
//
// The most common settings ops — colour overrides — at the prompt root so
// users can theme without an extra module path:
//
//     use cli_ui::prompt::update_colors;
//     update_colors(|c| c.accent = magenta_bold);
pub use settings::{colors, set_colors, update_colors, Colors};

/// Glob-importable shortcut for the most common entry points.
///
/// ```
/// use cli_ui::prompt::prelude::*;
/// ```
///
/// Re-exports every prompt constructor, the frame helpers (`intro`, `outro`,
/// `note`, `cancel`), `OnCancel`, the validator rules library, and the
/// colour theme ops. Designed for quick scripts; production code can stick
/// with the explicit imports.
pub mod prelude {
    pub use super::{
        autocomplete, boxed, cancel, confirm, date, group, groupmultiselect, intro, log, multiline,
        multiselect, note, outro, path, progress, secret, select, select_key, spinner, stream,
        task_log, tasks, text,
    };
    pub use super::{OnCancel, PromptError, Result};
    // Rules library — composable predicates and the Validator type.
    pub use super::{
        alpha_only, alphanumeric, email, ends_with, exact_chars, float_between, forbid_chars,
        has_digit, has_lower, has_special, has_upper, int_between, max_chars, min_chars, one_of,
        only_chars, required, starts_with, word_count, words_between, Validate, Validator,
    };
    // Colour theme.
    pub use super::{colors, set_colors, update_colors, Colors};
    // Full validate module for users who want everything at once.
    pub use super::validate;
}

// ── Frame helpers (clack-style) ───────────────────────────────────────────────

/// Print `┌  message` — opens a connected prompt session.
pub fn intro(message: impl AsRef<str>) {
    eprintln!("{}", theme::intro(message.as_ref()));
}

/// Print `└  message` — closes a prompt session.
pub fn outro(message: impl AsRef<str>) {
    eprintln!("{}", theme::outro(message.as_ref()));
}

/// Print a framed informational note between prompts.
pub fn note(title: impl AsRef<str>, body: impl AsRef<str>) {
    eprintln!("{}", theme::note(title.as_ref(), body.as_ref()));
}

/// Print `■  message` — used after a prompt was cancelled / interrupted.
pub fn cancel(message: impl AsRef<str>) {
    eprintln!("{}", theme::cancel(message.as_ref()));
}
