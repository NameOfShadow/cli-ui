//! # cli-ui
//!
//! Styled CLI framework for Rust — typed argument parsing, consistent help,
//! progress output, and summary blocks with zero boilerplate.
//!
//! ## Quick start — single command
//!
//! ```rust,no_run
//! use std::path::PathBuf;
//! use cli_ui::{CliOptions, header, phase, step, ok, summary};
//! use cli_ui::styles::{paint, CYAN, YELLOW, DIM, OK};
//! use cli_ui::progress::format_bytes;
//!
//! #[derive(CliOptions)]
//! #[cli(about = "compress images", theme = "green")]
//! struct Opt {
//!     #[arg(positional, validate(exists, is_file, ext("png","jpg","webp")))]
//!     input: PathBuf,
//!
//!     #[arg(positional, validate(is_dir), action(create_dir_all))]
//!     output: PathBuf,
//!
//!     #[arg(section = "Quality", short = 'q', long = "quality",
//!           default = 85, validate(range(1, 100)))]
//!     quality: u32,
//!
//!     #[arg(section = "Quality", long = "format",
//!           default = "webp", validate(one_of("webp","jpeg","png")))]
//!     format: String,
//!
//!     #[arg(section = "Performance", short = 'j', long = "jobs",
//!           default = env("JOBS", 4), validate(range(1, 256)))]
//!     jobs: usize,
//! }
//!
//! fn main() {
//!     let opt = Opt::parse();
//!     header!("imgopt", env!("CARGO_PKG_VERSION"), "compress images", "lossless by default");
//!     phase!("init", "reading {}", opt.input.display());
//!     summary! {
//!         done: "Done",
//!         "output" => paint(CYAN, &opt.output.display().to_string()),
//!         section,
//!         "time" => paint(DIM, "120ms"),
//!     }
//! }
//! ```
//!
//! ## Quick start — subcommands
//!
//! ```rust,no_run
//! use cli_ui::{CliCommand, CliOptions, Result};
//!
//! #[derive(CliOptions)]
//! struct Global {
//!     #[arg(short = 'v', long = "verbose", negatable)]
//!     verbose: bool,
//! }
//!
//! #[derive(CliCommand)]
//! #[cli(name = "mytool", about = "...", theme = "cyan", global = Global)]
//! enum Cmd {
//!     /// Download a file
//!     #[cli(alias = "dl")]
//!     Download(DownloadOpt),
//!     /// Show status
//!     Status,
//! }
//!
//! fn main() -> Result<()> {
//!     match Cmd::parse()? {
//!         Cmd::Download(opt) => download(Cmd::global(), opt),
//!         Cmd::Status        => status(Cmd::global()),
//!     }
//!     Ok(())
//! }
//! # #[derive(cli_ui::CliOptions)] struct DownloadOpt {}
//! # fn download(_: &Global, _: DownloadOpt) {}
//! # fn status(_: &Global) {}
//! ```
//!
//! ## Interactive prompts
//!
//! `cli-ui` also ships clack-style interactive prompts behind the
//! `interactive` feature — text, select, multiselect, confirm, password,
//! autocomplete, date, path, spinner, framed log lines, sequential tasks.
//! Read the [`prompt`] module docs for the full mental model; here's the
//! whole API surface in five lines:
//!
//! ```no_run
//! use cli_ui::prompt::prelude::*;
//!
//! intro("Set up");
//! let name = text("Your name").run().or_cancel("Bye.");
//! let go   = confirm("Continue?").default(true).run().or_cancel("Bye.");
//! outro(format!("Hi {name}, {}", if go { "going" } else { "stopping" }));
//! ```

pub mod complete;
pub mod help;
pub mod macros;
pub mod progress;
pub mod prompt;
pub mod styles;
pub mod summary;
pub mod term;

pub use cli_ui_derive::{CliCommand, CliOptions};
pub use progress::{format_bytes, Progress};
pub use summary::Summary;

use styles::*;

// ─────────────────────────────────────────────────────────────────────────────
// Core error + result types
// ─────────────────────────────────────────────────────────────────────────────

/// Error returned by [`CliCommand::parse`] and [`CliOptions::parse_args`].
#[derive(Debug)]
pub struct CliError(pub String);

impl std::fmt::Display for CliError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for CliError {}

/// `Result<T>` alias used throughout cli-ui.
pub type Result<T> = std::result::Result<T, CliError>;

// ─────────────────────────────────────────────────────────────────────────────
// Traits
// ─────────────────────────────────────────────────────────────────────────────

/// Implemented by `#[derive(CliOptions)]` on structs.
///
/// Provides `parse_args(&[&str])` for use both as a standalone parser
/// and as a subcommand option parser inside `CliCommand`.
pub trait CliOptions: Sized {
    #[doc(hidden)]
    fn parse_args(args: &[&str]) -> std::result::Result<Self, String>;
}

impl CliOptions for () {
    fn parse_args(_: &[&str]) -> std::result::Result<Self, String> {
        Ok(())
    }
}

/// Implemented by `#[derive(CliCommand)]` on enums.
///
/// # Note on `global()`
///
/// Global options are stored in a process-wide [`std::sync::OnceLock`]
/// set during `parse()`. Safe for CLI utilities. For libraries or tests
/// that call `parse()` multiple times in the same process, pass the global
/// options explicitly instead of relying on `global()`.
pub trait CliCommand: Sized {
    /// Type of global options. `()` if `#[cli(global = ...)]` is not used.
    type Global: 'static;

    /// Parse subcommand + global options from `std::env::args()`.
    fn parse() -> Result<Self>;

    /// Return a reference to the parsed global options.
    ///
    /// # Panics
    /// Panics if called before [`parse()`](Self::parse).
    fn global() -> &'static Self::Global
    where
        Self::Global: Sized;

    /// Print styled help to stderr.
    fn help();
}

/// Internal trait — uniform dispatch for both `CliOptions` leaves
/// and nested `CliCommand` enums.
#[doc(hidden)]
pub trait ParseInner: Sized {
    fn parse_inner(args: &[&str]) -> Result<Self>;
}

/// Internal trait — print help for a specific subcommand type.
/// Implemented by generated code for both CliOptions and CliCommand types.
#[doc(hidden)]
pub trait PrintHelp {
    fn print_help();
}

/// Internal trait — nested completion support.
/// Implemented by generated code for both CliOptions and CliCommand types.
#[doc(hidden)]
pub trait NestedCompletions {
    fn print_completions(shell: &str);
}

// ─────────────────────────────────────────────────────────────────────────────
// Global flag parsing (used by generated code)
// ─────────────────────────────────────────────────────────────────────────────

/// Split `args` into global flags (before the subcommand name) and the rest.
///
/// Called by generated `parse()` when `#[cli(global = T)]` is present.
#[doc(hidden)]
pub fn parse_global_flags<G: CliOptions>(args: &[String]) -> (G, Vec<String>) {
    let split = args
        .iter()
        .position(|a| !a.starts_with('-'))
        .unwrap_or(args.len());
    let global_refs: Vec<&str> = args[..split].iter().map(|s| s.as_str()).collect();
    let rest: Vec<String> = args[split..].to_vec();
    let global = G::parse_args(&global_refs).unwrap_or_else(|e| {
        print_error(&format!("invalid global flag: {e}"));
        std::process::exit(1);
    });
    (global, rest)
}

// ─────────────────────────────────────────────────────────────────────────────
// Print helpers (called from generated code)
// ─────────────────────────────────────────────────────────────────────────────

/// Print a themed version badge to stderr.
pub fn print_version(name: &str, version: &str, badge: anstyle::Style) {
    eprintln!();
    eprintln!(
        "  {}  {}",
        paint(badge, &format!(" {name} ")),
        paint(DIM, &format!("v{version}"))
    );
    eprintln!();
}

/// Print a styled error line to stderr.
///
/// ```text
///  ✘  something went wrong
/// ```
pub fn print_error(msg: &str) {
    eprintln!();
    eprintln!("{}  {}", paint(ERR, CROSS), paint(BOLD, msg));
    eprintln!();
}

/// Print a yellow warning line to stderr (non-fatal).
///
/// ```text
///  ⚠  file already exists — may be overwritten
/// ```
pub fn print_warning(msg: &str) {
    eprintln!();
    eprintln!("{}  {}", paint(YELLOW, "⚠"), paint(YELLOW, msg));
    eprintln!();
}

/// Print "missing required arguments" error.
pub fn print_missing_args(name: &str, _about: &str, _tagline: &str) {
    eprintln!();
    eprintln!(
        "{}  {}",
        paint(ERR, CROSS),
        paint(BOLD, "missing required arguments")
    );
    eprintln!(
        "{}  {}",
        paint(DIM, BAR),
        paint(WHITE, &format!("{name} <INPUT> <OUTPUT>"))
    );
    eprintln!(
        "{}  {}",
        paint(DIM, ARROW),
        paint(WHITE, &format!("run `{name} --help` for usage"))
    );
    eprintln!();
}

/// Print "unknown subcommand" error with did-you-mean suggestion.
pub fn print_unknown_subcommand(app: &str, cmd: &str, known: &[&str]) {
    eprintln!();
    eprintln!(
        "{}  {}",
        paint(ERR, CROSS),
        paint(BOLD, &format!("unknown command: `{cmd}`"))
    );
    if let Some(s) = did_you_mean(cmd, known) {
        eprintln!(
            "{}  {}",
            paint(DIM, BAR),
            paint(WHITE, &format!("did you mean `{s}`?"))
        );
    }
    eprintln!(
        "{}  {}",
        paint(DIM, ARROW),
        paint(
            WHITE,
            &format!("run `{app} --help` to see available commands")
        )
    );
    eprintln!();
}

/// Print "missing subcommand" error.
pub fn print_missing_subcommand(app: &str, known: &[&str]) {
    eprintln!();
    eprintln!("{}  {}", paint(ERR, CROSS), paint(BOLD, "missing command"));
    eprintln!(
        "{}  {}",
        paint(DIM, BAR),
        paint(WHITE, &format!("available: {}", known.join(", ")))
    );
    eprintln!(
        "{}  {}",
        paint(DIM, ARROW),
        paint(WHITE, &format!("run `{app} --help` for usage"))
    );
    eprintln!();
}

/// Print "usage: app subcommand" header for subcommand help.
pub fn print_sub_usage(app: &str, cmd: &str) {
    let (_, accent) = styles::theme_styles("cyan");
    eprintln!();
    eprintln!(
        "{} {} {}",
        paint(accent, DIAMOND),
        paint(BOLD, &format!("{app} {cmd}")),
        paint(DIM, "— subcommand options"),
    );
}

/// Print help for a unit variant (no options).
pub fn print_unit_help(app: &str, cmd: &str, _opts: &[(&str, &str)]) {
    let (_, accent) = styles::theme_styles("cyan");
    eprintln!();
    eprintln!(
        "{} {} {}",
        paint(accent, DIAMOND),
        paint(BOLD, &format!("{app} {cmd}")),
        paint(DIM, "— no options")
    );
    eprintln!(
        "{}  {}",
        paint(DIM, BAR),
        paint(DIM, &format!("usage: {app} {cmd}"))
    );
    eprintln!();
}

// ─────────────────────────────────────────────────────────────────────────────
// Glob matching (no regex dependency)
// ─────────────────────────────────────────────────────────────────────────────

/// Simple glob matching — supports `*`, `?`, `[abc]`.
///
/// Used by `validate(glob("*.csv"))`. Does not use the `regex` crate.
///
/// # Examples
/// ```
/// assert!(cli_ui::glob_match("*.csv", "data.csv"));
/// assert!(cli_ui::glob_match("file_?.txt", "file_1.txt"));
/// assert!(!cli_ui::glob_match("*.json", "data.csv"));
/// ```
pub fn glob_match(pattern: &str, input: &str) -> bool {
    glob_match_inner(pattern.as_bytes(), input.as_bytes())
}

fn glob_match_inner(pat: &[u8], s: &[u8]) -> bool {
    match (pat.first(), s.first()) {
        (None, None) => true,
        (Some(b'*'), _) => {
            // * matches zero or more chars
            glob_match_inner(&pat[1..], s) || (!s.is_empty() && glob_match_inner(pat, &s[1..]))
        }
        (Some(b'?'), Some(_)) => glob_match_inner(&pat[1..], &s[1..]),
        (Some(b'['), _) => {
            // find closing ]
            let end = pat.iter().position(|&b| b == b']').unwrap_or(pat.len());
            let class = &pat[1..end];
            if s.is_empty() {
                return false;
            }
            let matched = class.contains(&s[0]);
            if matched {
                glob_match_inner(&pat[end + 1..], &s[1..])
            } else {
                false
            }
        }
        (Some(a), Some(b)) if a == b => glob_match_inner(&pat[1..], &s[1..]),
        _ => false,
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Did-you-mean (Levenshtein distance ≤ 2)
// ─────────────────────────────────────────────────────────────────────────────

fn did_you_mean<'a>(input: &str, candidates: &[&'a str]) -> Option<&'a str> {
    candidates
        .iter()
        .filter_map(|c| {
            let d = edit_distance(input, c);
            if d <= 2 {
                Some((d, *c))
            } else {
                None
            }
        })
        .min_by_key(|(d, _)| *d)
        .map(|(_, c)| c)
}

#[allow(clippy::needless_range_loop)]
fn edit_distance(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let (m, n) = (a.len(), b.len());
    let mut dp = vec![vec![0usize; n + 1]; m + 1];
    for i in 0..=m {
        dp[i][0] = i;
    }
    for j in 0..=n {
        dp[0][j] = j;
    }
    for i in 1..=m {
        for j in 1..=n {
            dp[i][j] = if a[i - 1] == b[j - 1] {
                dp[i - 1][j - 1]
            } else {
                1 + dp[i - 1][j].min(dp[i][j - 1]).min(dp[i - 1][j - 1])
            };
        }
    }
    dp[m][n]
}
