//! Example: mytool — general-purpose batch file processor
//!
//! Demonstrates all cli-ui features:
//! - #[derive(CliOptions)] with themes, multiple examples
//! - positionals, short/long flags, negatable bools, Vec<T> multi flags
//! - conflicts_with + resolved_*() helpers
//! - header!, phase!, step!, substep!, ok!, bail!, summary!
//! - Progress with ok/fail, format_bytes

use cli_ui::progress::format_bytes;
use cli_ui::styles::{paint, BOLD, CYAN, DIM, ERR, OK, YELLOW};
use cli_ui::{bail, header, ok, phase, step, substep, summary, CliOptions, Progress};
use std::path::PathBuf;

#[derive(CliOptions)]
#[cli(
    name = "mytool",
    about = "batch file processor",
    tagline = "fast and parallel",
    theme = "cyan",
    example = "mytool input/ output/",
    example = "mytool input/ output/ -j 8 --format json",
    example = "mytool -i input/ -o output/ --include '*.csv' --dry-run",
    hint = "all local files are copied automatically",
    url = "https://github.com/you/mytool"
)]
struct Opt {
    /// Input directory
    #[arg(positional)]
    input: PathBuf,

    /// Output directory
    #[arg(positional)]
    output: PathBuf,

    // ── Input / Output ────────────────────────────────────────────────
    /// Input directory (alternative to positional)
    #[arg(
        section = "Input / Output",
        short = 'i',
        long = "input",
        conflicts_with = "input"
    )]
    alt_input: Option<PathBuf>,

    /// Output directory (alternative to positional)
    #[arg(
        section = "Input / Output",
        short = 'o',
        long = "output",
        conflicts_with = "output"
    )]
    alt_output: Option<PathBuf>,

    // ── Processing ────────────────────────────────────────────────────
    /// Parallel workers
    #[arg(section = "Processing", short = 'j', long = "jobs", default = 4)]
    jobs: usize,

    /// Output format
    #[arg(section = "Processing", long = "format", default = "html")]
    format: String,

    // ── Filters ───────────────────────────────────────────────────────
    /// Include glob patterns — repeatable
    #[arg(section = "Filters", long = "include", multi)]
    include: Vec<String>,

    /// Exclude glob patterns — repeatable
    #[arg(section = "Filters", long = "exclude", multi)]
    exclude: Vec<String>,

    /// Preview without writing anything
    #[arg(section = "Filters", long = "dry-run", negatable)]
    dry_run: bool,

    /// Keep intermediate files
    #[arg(section = "Filters", long = "keep-tmp", negatable)]
    _keep_tmp: bool,
}

fn main() {
    // parse() handles --help, --version, --completions,
    // unknown flags, and missing positionals automatically
    let opt = Opt::parse();

    // resolved_alt_input() generated from conflicts_with = "input"
    let input = opt.resolved_alt_input();
    let output = opt.resolved_alt_output();
    let start = std::time::Instant::now();

    // bail! prints error and exits 1
    if !input.exists() {
        bail!("input path does not exist: {}", input.display());
    }

    header!(
        "mytool",
        env!("CARGO_PKG_VERSION"),
        "batch file processor",
        "fast and parallel"
    );

    phase!("init", "scanning {}", input.display());
    phase!(
        "scan",
        "found {} remote · {} local   jobs: {}",
        8,
        3,
        opt.jobs
    );

    if opt.dry_run {
        phase!("mode", "dry run — nothing will be written");
    }
    if !opt.include.is_empty() {
        phase!("filter", "include: {}", opt.include.join(", "));
    }
    if !opt.exclude.is_empty() {
        phase!("filter", "exclude: {}", opt.exclude.join(", "));
    }

    // ── remote files ──────────────────────────────────────────────────
    let remote: &[(&str, &str, usize)] = &[
        ("https://example.com/data.csv", "./data/data.csv", 43_100),
        ("https://example.com/ref.json", "./data/ref.json", 8_500),
        (
            "https://example.com/schema.json",
            "./data/schema.json",
            2_300,
        ),
        ("https://cdn.example.com/lib.js", "./js/lib.js", 128_000),
        (
            "https://cdn.example.com/style.css",
            "./css/style.css",
            12_400,
        ),
    ];

    step!("downloading remote files");
    let pb = Progress::new(remote.len());
    let mut errors = 0usize;
    for (url, local, bytes) in remote {
        // simulate one failure
        if url.contains("lib.js") {
            pb.fail("file", url, "connection timeout");
            errors += 1;
        } else {
            pb.ok("file", "remote", url, local, *bytes);
        }
    }
    pb.finish();

    // ── local files ───────────────────────────────────────────────────
    let local: &[(&str, &str, usize)] = &[
        ("config.toml", "./cfg/config.toml", 1_200),
        ("rules.yaml", "./cfg/rules.yaml", 3_400),
        ("README.md", "./README.md", 8_800),
    ];

    step!("copying local files");
    let pb2 = Progress::new(local.len());
    for (src, dst, bytes) in local {
        pb2.ok("file", "local", src, dst, *bytes);
    }
    pb2.finish();

    // ── nested operations ─────────────────────────────────────────────
    step!("resolving references in style.css");
    substep!(
        "https://fonts.example.com/Inter.woff2",
        "./fonts/Inter.woff2"
    );
    substep!(
        "https://fonts.example.com/JetBrains.woff2",
        "./fonts/JetBrains.woff2"
    );

    // ── write ─────────────────────────────────────────────────────────
    if !opt.dry_run {
        step!("writing output");
        ok!(output.join(format!("result.{}", opt.format)).display());
    }

    // ── summary ───────────────────────────────────────────────────────
    let remote_ok = remote.len() - errors;
    let total_files = remote_ok + local.len();
    let total_bytes: usize = remote
        .iter()
        .filter(|(u, _, _)| !u.contains("lib.js"))
        .map(|(_, _, b)| b)
        .chain(local.iter().map(|(_, _, b)| b))
        .sum();

    let input_val = paint(CYAN, &input.display().to_string());
    let output_val = paint(
        BOLD,
        &output
            .join(format!("result.{}", opt.format))
            .display()
            .to_string(),
    );
    let files_val = format!(
        "{} total · {} errors",
        paint(OK, &total_files.to_string()),
        paint(if errors > 0 { ERR } else { DIM }, &errors.to_string())
    );
    let size_val = paint(YELLOW, &format_bytes(total_bytes));
    let time_val = paint(DIM, &format!("{}ms", start.elapsed().as_millis()));

    if errors == 0 {
        summary! {
            done:    "All files processed",
            "input"  => input_val,
            "output" => output_val,
            section,
            "files"  => files_val,
            "size"   => size_val,
            "time"   => time_val,
        }
    } else {
        summary! {
            warn:    "Completed with errors",
            "input"  => input_val,
            "output" => output_val,
            section,
            "files"  => files_val,
            "size"   => size_val,
            "time"   => time_val,
        }
    }
}
