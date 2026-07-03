//! User-facing macros for runtime output.
//!
//! All macros write to **stderr** via [`anstream`], which automatically
//! strips ANSI codes when output is piped or redirected.
//!
//! # Quick reference
//!
//! | Macro | Output |
//! |-------|--------|
//! | [`header!`](crate::header) | Themed badge + version + tagline |
//! | [`phase!`](crate::phase) | `▶ init  reading file.html` |
//! | [`step!`](crate::step) | `▸ downloading remote assets` |
//! | [`substep!`](crate::substep) | `└─ https://… → ./fonts/…` |
//! | [`ok!`](crate::ok) | `✓  dist/index.html` |
//! | [`bail!`](crate::bail) | Error + exit(1) |
//! | [`summary!`](macro@crate::summary) | Aligned summary block |

/// Print the app header with a themed badge.
///
/// Reads `badge` from the app theme set in `#[cli(theme = "...")]`.
/// Use the 4-argument form for the default cyan badge.
///
/// # Example
/// ```rust,no_run
/// cli_ui::header!("sitefetch", "0.1.0", "offline asset localizer", "all assets offline");
/// ```
///
/// Output:
/// ```text
///   sitefetch   v0.1.0
///  offline asset localizer — all assets offline
/// ```
#[macro_export]
macro_rules! header {
    // with explicit badge style (generated code passes this form)
    ($name:expr, $version:expr, $about:expr, $tagline:expr, badge=$badge:expr) => {{
        use $crate::styles::*;
        eprintln!();
        eprintln!(
            "  {}  {}",
            paint($badge, &format!(" {} ", $name)),
            paint(DIM, &format!("v{}", $version)),
        );
        eprintln!(" {}", paint(DIM, &format!("{} — {}", $about, $tagline)));
        eprintln!();
    }};
    // default cyan badge
    ($name:expr, $version:expr, $about:expr, $tagline:expr) => {{
        use $crate::styles::*;
        eprintln!();
        eprintln!(
            "  {}  {}",
            paint(BADGE_CYAN, &format!(" {} ", $name)),
            paint(DIM, &format!("v{}", $version)),
        );
        eprintln!(" {}", paint(DIM, &format!("{} — {}", $about, $tagline)));
        eprintln!();
    }};
}

/// Print a phase line: `▶ tag  message`
///
/// # Example
/// ```rust,no_run
/// cli_ui::phase!("init", "reading {}", "index.html");
/// // ▶  init  reading index.html
/// ```
#[macro_export]
macro_rules! phase {
    ($tag:expr, $($arg:tt)*) => {{
        use $crate::styles::*;
        let msg = format!($($arg)*);
        eprintln!(
            " {}  {:<6}  {}",
            paint(ACCENT, PHASE),
            paint(ACCENT, $tag),
            paint(DIM, &msg),
        );
    }};
}

/// Print a step header: `▸ message`
///
/// # Example
/// ```rust,no_run
/// cli_ui::step!("downloading remote assets");
/// // ▸  downloading remote assets
/// ```
#[macro_export]
macro_rules! step {
    ($($arg:tt)*) => {{
        use $crate::styles::*;
        let msg = format!($($arg)*);
        eprintln!();
        eprintln!(" {}  {}", paint(WHITE, BULLET), paint(WHITE, &msg));
    }};
}

/// Print a CSS/asset sub-step: `└─ url → local`
///
/// # Example
/// ```rust,no_run
/// cli_ui::substep!("https://fonts.gstatic.com/inter.ttf", "./fonts/inter.ttf");
/// ```
#[macro_export]
macro_rules! substep {
    ($url:expr, $local:expr) => {
        $crate::progress::substep($url, $local);
    };
}

/// Print a success line outside of a [`Progress`](crate::Progress) block.
///
/// # Example
/// ```rust,no_run
/// cli_ui::ok!("dist/index.html");
/// //   ✓  dist/index.html
/// ```
#[macro_export]
macro_rules! ok {
    ($path:expr) => {{
        use $crate::styles::*;
        eprintln!(
            "   {}  {}",
            paint(OK, CHECK),
            paint(DIM, &$path.to_string())
        );
    }};
}

/// Print an error message and exit with code 1.
///
/// # Example
/// ```rust,no_run
/// # let path = std::path::PathBuf::from("/etc/hosts");
/// cli_ui::bail!("cannot read file: {}", path.display());
/// ```
#[macro_export]
macro_rules! bail {
    ($($arg:tt)*) => {{
        $crate::print_error(&format!($($arg)*));
        ::std::process::exit(1);
    }};
}

/// Build and print a styled summary block at the end of a run.
///
/// # Syntax
/// ```rust,no_run
/// use cli_ui::{summary, styles::{paint, CYAN, YELLOW, DIM, OK}};
///
/// summary! {
///     done: "All assets localized",       // or warn: "..." for yellow badge
///     "input"  => paint(CYAN, "in.html"),
///     "output" => paint(CYAN, "dist/"),
///     section,                            // starts new alignment group
///     "assets" => format!("{} remote", paint(OK, "19")),
///     "size"   => paint(YELLOW, "1.64 MB"),
///     "time"   => paint(DIM, "13034ms"),
/// }
/// ```
///
/// The `section,` separator resets column alignment — keys in each section
/// are right-aligned independently.
#[macro_export]
macro_rules! summary {
    (done: $msg:expr, $($rest:tt)*) => {{
        let s = $crate::summary::Summary::new().done($msg);
        let s = $crate::__summary_inner!(s, $($rest)*);
        s.print();
    }};
    (warn: $msg:expr, $($rest:tt)*) => {{
        let s = $crate::summary::Summary::new().warn($msg);
        let s = $crate::__summary_inner!(s, $($rest)*);
        s.print();
    }};
    ($($rest:tt)*) => {{
        let s = $crate::summary::Summary::new();
        let s = $crate::__summary_inner!(s, $($rest)*);
        s.print();
    }};
}

#[doc(hidden)]
#[macro_export]
macro_rules! __summary_inner {
    ($s:expr,) => { $s };
    ($s:expr)  => { $s };

    ($s:expr, done: $msg:expr, $($rest:tt)*) => {{
        let s = $s.done($msg);
        $crate::__summary_inner!(s, $($rest)*)
    }};
    ($s:expr, done: $msg:expr) => { $s.done($msg) };

    ($s:expr, warn: $msg:expr, $($rest:tt)*) => {{
        let s = $s.warn($msg);
        $crate::__summary_inner!(s, $($rest)*)
    }};
    ($s:expr, warn: $msg:expr) => { $s.warn($msg) };

    ($s:expr, section, $($rest:tt)*) => {{
        let s = $s.blank().section();
        $crate::__summary_inner!(s, $($rest)*)
    }};

    ($s:expr, $key:literal => $val:expr, $($rest:tt)*) => {{
        let s = $s.stat($key, &format!("{}", $val));
        $crate::__summary_inner!(s, $($rest)*)
    }};
    ($s:expr, $key:literal => $val:expr) => {
        $s.stat($key, &format!("{}", $val))
    };
}
