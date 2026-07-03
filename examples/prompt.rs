//! Example: prompt — demonstrates all prompt types.
//!
//! ```bash
//! cargo run --example prompt
//! ```
//!
//! When stdout is piped or not a TTY, the prompts fall back to a numeric
//! line-buffered interface automatically.

use cli_ui::prompt::{
    self, autocomplete, confirm, groupmultiselect, intro, log, multiselect, note, outro, progress,
    secret, select, select_key, spinner, tasks, text, ProgressStyle, Task,
};
use cli_ui::styles::{paint, CYAN, DIM, OK};
use cli_ui::{header, summary};

fn main() -> cli_ui::prompt::Result<()> {
    header!(
        "prompt",
        env!("CARGO_PKG_VERSION"),
        "interactive prompt showcase",
        "all prompt types"
    );

    intro("Set up a new project");

    // ── 1. Single select with per-option hints ────────────────────────────────
    let template = select("How would you like to start your new project?")
        .option("basic", "A basic, helpful starter project")
        .option("blog", "Use blog template")
        .hint("Markdown-based blog with RSS feed support")
        .option("docs", "Use docs (Starlight) template")
        .hint("Documentation site with search and sidebar")
        .option("minimal", "Use minimal (empty) template")
        .hint("Just a Cargo.toml and src/lib.rs — nothing else")
        .run()?;

    // ── 2. Text input with validation ─────────────────────────────────────────
    let dir = text("Where should we create your new project?")
        .default("./my-app")
        .placeholder("e.g. ./my-project")
        .hint("Use a relative or absolute path")
        .validate(|s| {
            if s.starts_with('.') || s.starts_with('/') {
                Ok(())
            } else {
                Err("path must start with . or /".into())
            }
        })
        .run()?;

    // ── 3. Multi select with per-option hints ─────────────────────────────────
    let features = multiselect("Which features would you like to enable?")
        .option("ts", "TypeScript")
        .hint("JavaScript with syntax for types")
        .option("tailwind", "Tailwind CSS")
        .hint("A utility-first CSS framework")
        .option("react", "React")
        .hint("A JavaScript library for building user interfaces")
        .option("vue", "Vue")
        .hint("The Progressive JavaScript Framework")
        .option("eslint", "ESLint")
        .hint("Find and fix problems in your JavaScript code")
        .run()?;

    // ── 4. Grouped multiselect ────────────────────────────────────────────────
    let tools = groupmultiselect("Select development tools:")
        .group("Frontend")
        .item("TypeScript")
        .hint("JavaScript with syntax for types")
        .item("ESLint")
        .hint("Find and fix problems in your JS code")
        .item("Prettier")
        .hint("An opinionated code formatter")
        .group("Backend")
        .item("Node.js")
        .hint("JavaScript runtime built on V8")
        .item("Express")
        .hint("Fast, unopinionated web framework")
        .item("Prisma")
        .hint("Next-generation ORM for Node.js")
        .group("Testing")
        .item("Jest")
        .hint("Delightful JavaScript testing framework")
        .item("Cypress")
        .hint("End-to-end testing for the modern web")
        .item("Vitest")
        .hint("Vite-native unit testing framework")
        .run()?;

    // ── 5. Autocomplete ───────────────────────────────────────────────────────
    let pkg = autocomplete("Search for a UI package:")
        .option("astro", "Astro")
        .hint("The web framework for content-based sites")
        .option("react", "React")
        .hint("A JavaScript library for building user interfaces")
        .option("vue", "Vue")
        .hint("The Progressive JavaScript Framework")
        .option("svelte", "Svelte")
        .hint("Cybernetically enhanced web apps")
        .option("angular", "Angular")
        .hint("Platform for building mobile & desktop web apps")
        .option("solid", "SolidJS")
        .hint("Simple and performant reactivity for building UIs")
        .option("qwik", "Qwik")
        .hint("HTML-first framework with instant interactivity")
        .placeholder("Type to search...")
        .max_items(5)
        .run()?;

    // ── 6. Confirm ────────────────────────────────────────────────────────────
    let git = confirm("Initialize a new git repository?")
        .default(true)
        .run()?;

    // ── 7. Secret ─────────────────────────────────────────────────────────────
    let _token = secret("Enter your API token (optional)")
        .allow_empty(true)
        .hint("Leave empty to skip — you can set it later in .env")
        .run()?;

    // ── 8. Single-keypress select ─────────────────────────────────────────────
    let action = select_key("Apply changes?")
        .option('y', "Yes, apply")
        .hint("write to disk")
        .option('n', "No, abort")
        .hint("discard everything")
        .option('d', "Diff first")
        .hint("preview the changes")
        .run()?;
    log::info(format!("you chose: {} ({})", action.key, action.label));

    note(
        "What happens next",
        "We'll scaffold your project, install dependencies,\nand initialise git if you opted in.",
    );

    // ── 9. Spinner + tasks runner ─────────────────────────────────────────────
    let s = spinner();
    s.start("Resolving dependencies");
    std::thread::sleep(std::time::Duration::from_millis(700));
    s.message("Downloading 42 packages");
    std::thread::sleep(std::time::Duration::from_millis(700));
    s.stop("Resolved 42 packages");

    // ── 10. Progress bar ──────────────────────────────────────────────────────
    // Filled/empty bar rendered inside a spinner line. Advance in a loop
    // (or from a worker thread) with `.advance(step, Some(message))`.
    let pb = progress().style(ProgressStyle::Heavy).size(30).max(50);
    pb.start("Downloading assets");
    for i in 1..=50 {
        std::thread::sleep(std::time::Duration::from_millis(30));
        let msg = format!("asset {i}/50");
        pb.advance(1, Some(msg));
    }
    pb.stop("Downloaded 50 assets");

    tasks(vec![
        Task::new("Compiling", |s| {
            std::thread::sleep(std::time::Duration::from_millis(400));
            s.message("crate cli-ui");
            std::thread::sleep(std::time::Duration::from_millis(400));
            Some("Compiled in 0.8s".into())
        }),
        Task::new("Running tests", |_| {
            std::thread::sleep(std::time::Duration::from_millis(500));
            Some("All 12 tests passed".into())
        }),
    ]);

    log::success("Project ready");
    outro("Happy hacking!");

    let _ = prompt::Result::<()>::Ok(());

    // ── Summary ───────────────────────────────────────────────────────────────
    let feat_str = if features.is_empty() {
        paint(DIM, "none")
    } else {
        paint(OK, &features.values().join(", "))
    };

    let tool_str = if tools.is_empty() {
        paint(DIM, "none")
    } else {
        paint(OK, &tools.values().join(", "))
    };

    summary! {
        done:        "Project configured",
        "template"   => paint(CYAN,   template.value()),
        "directory"  => paint(CYAN,   &dir),
        section,
        "features"   => feat_str,
        "tools"      => tool_str,
        "ui package" => paint(CYAN,   pkg.value()),
        section,
        "git"        => paint(if git { OK } else { DIM }, if git { "yes" } else { "no" }),
    }

    Ok(())
}
