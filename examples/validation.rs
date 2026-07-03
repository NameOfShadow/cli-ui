//! Example: validation — demonstrates all validate(), action(), and
//! inter-field constraint attributes.
//!
//! Run:
//!   cargo run --example validation -- --help
//!   cargo run --example validation -- input.csv output/ --port 80
//!   cargo run --example validation -- input.csv output/ --json --csv   # conflict error
//!   cargo run --example validation -- input.csv output/ --sign         # requires --key

use cli_ui::styles::{paint, CYAN, DIM, OK, YELLOW};
use cli_ui::{header, ok, phase, summary, CliOptions};
use std::path::PathBuf;

// ── Validator functions ───────────────────────────────────────────────────────

fn validate_worker_name(s: &str) -> Result<(), String> {
    if s.chars().all(|c| c.is_alphanumeric() || c == '-') {
        Ok(())
    } else {
        Err(format!(
            "`{s}` contains invalid characters — only a-z, 0-9, and `-` allowed"
        ))
    }
}

fn validate_slug(s: &str) -> Result<(), String> {
    if s.chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
    {
        Ok(())
    } else {
        Err(format!(
            "`{s}` is not a valid slug — only a-z, 0-9, and `-` allowed"
        ))
    }
}

// ── Completion providers ──────────────────────────────────────────────────────

fn available_profiles() -> Vec<String> {
    vec![
        "default".into(),
        "fast".into(),
        "quality".into(),
        "debug".into(),
    ]
}

/// Scans examples/ and returns filenames without .rs extension.
/// Returns empty Vec if the directory doesn't exist — identical to cargo behaviour.
fn available_examples() -> Vec<String> {
    let Ok(rd) = std::fs::read_dir("examples") else {
        return Vec::new();
    };
    let mut names: Vec<String> = rd
        .flatten()
        .filter_map(|e| {
            let path = e.path();
            if path.is_file() && path.extension()?.to_str() == Some("rs") {
                Some(path.file_stem()?.to_string_lossy().to_string())
            } else {
                None
            }
        })
        .collect();
    names.sort();
    names
}

/// Scans src/bin/ and returns filenames without .rs extension.
fn available_bins() -> Vec<String> {
    let Ok(rd) = std::fs::read_dir("src/bin") else {
        return Vec::new();
    };
    let mut names: Vec<String> = rd
        .flatten()
        .filter_map(|e| {
            let path = e.path();
            if path.is_file() && path.extension()?.to_str() == Some("rs") {
                Some(path.file_stem()?.to_string_lossy().to_string())
            } else {
                None
            }
        })
        .collect();
    names.sort();
    names
}

#[allow(dead_code)]
#[derive(CliOptions)]
#[cli(
    name = "processor",
    about = "process and transform data files",
    tagline = "fast, validated, reliable",
    theme = "blue",
    example = "processor input.csv output/ --format json --jobs 8",
    example = "processor input.csv output/ --sign --key signing.pem",
    hint = "run with --dry-run to preview without writing",
    url = "https://github.com/you/processor"
)]
struct Opt {
    // ── positionals ───────────────────────────────────────────────────
    /// Input CSV or JSON file (must exist)
    #[arg(positional, validate(exists, is_file, ext("csv", "json")))]
    input: PathBuf,

    /// Output directory (created automatically if missing)
    #[arg(positional, validate(is_dir), action(create_dir_all))]
    output: PathBuf,

    // ── input / output alternatives ───────────────────────────────────
    /// Input file (alternative to positional) — only .csv and .json
    #[arg(
        section = "Input / Output",
        short = 'i',
        long = "input",
        validate(exists, is_file, ext("csv", "json")),
        conflicts_with("input")
    )]
    alt_input: Option<PathBuf>,

    /// Output directory (alternative to positional)
    #[arg(
        section = "Input / Output",
        short = 'o',
        long = "output",
        validate(is_dir),
        action(create_dir_all),
        conflicts_with("output")
    )]
    alt_output: Option<PathBuf>,

    // ── output format — mutually exclusive group ───────────────────────
    /// Output as JSON
    #[arg(
        section = "Format",
        long = "json",
        group = "format",
        conflicts_with("csv", "toml")
    )]
    json: bool,

    /// Output as CSV
    #[arg(
        section = "Format",
        long = "csv",
        group = "format",
        conflicts_with("json", "toml")
    )]
    csv: bool,

    /// Output as TOML
    #[arg(
        section = "Format",
        long = "toml",
        group = "format",
        conflicts_with("json", "csv")
    )]
    toml: bool,

    // ── network ───────────────────────────────────────────────────────
    /// Remote host — required unless --local is set
    #[arg(
        section = "Network",
        long = "host",
        required_unless("local"),
        conflicts_with("local")
    )]
    host: Option<String>,

    /// Run in local mode (no host required)
    #[arg(section = "Network", long = "local", conflicts_with("host"))]
    local: bool,

    /// Port number (1–65535)
    #[arg(
        section = "Network",
        short = 'p',
        long = "port",
        default = 8080,
        validate(range(1, 65535))
    )]
    port: u32,

    // ── auth — requires / required_unless ─────────────────────────────
    /// Sign the output — requires --key
    #[arg(section = "Auth", long = "sign", requires("key"))]
    sign: bool,

    /// Signing key file — .pem or .der
    #[arg(
        section = "Auth",
        long = "key",
        validate(exists, is_file, ext("pem", "der"))
    )]
    key: Option<PathBuf>,

    /// Disable authentication — conflicts with --sign
    #[arg(section = "Auth", long = "no-auth", conflicts_with("sign"))]
    no_auth: bool,

    // ── processing ────────────────────────────────────────────────────
    /// Parallel workers (from $JOBS env or 4)
    #[arg(section = "Processing", short = 'j', long = "jobs",
          default = env("JOBS", "4"), validate(range(1, 256)))]
    jobs: usize,

    /// Worker name — 3–32 chars, alphanumeric + hyphen
    #[arg(section = "Processing", long = "worker-name",
          validate(min_len(3), max_len(32), custom = validate_worker_name))]
    worker_name: Option<String>,

    /// Slug — only a-z, 0-9, hyphen allowed
    #[arg(section = "Processing", long = "slug",
          validate(custom = validate_slug))]
    slug: Option<String>,

    /// File pattern filter (glob)
    #[arg(section = "Processing", long = "pattern", validate(glob("*.*")))]
    pattern: Option<String>,

    // ── env validator ─────────────────────────────────────────────────
    /// API token — validated against $API_TOKEN env var
    #[arg(
        section = "Auth",
        long = "token",
        validate(env("API_TOKEN", "")),
        required_unless("no_auth")
    )]
    token: Option<String>,

    // ── warn_if ───────────────────────────────────────────────────────
    /// Output file that will warn if it already exists
    #[arg(
        section = "Input / Output",
        long = "out-file",
        validate(warn_if(exists))
    )]
    out_file: Option<PathBuf>,

    // ── misc ──────────────────────────────────────────────────────────
    /// Preview without writing anything
    #[arg(section = "Misc", long = "dry-run", negatable)]
    dry_run: bool,

    /// Verbose output
    #[arg(section = "Misc", short = 'v', long = "verbose", negatable)]
    verbose: bool,

    /// Profile — dynamic completions via available_profiles()
    #[arg(section = "Misc", long = "profile", complete = available_profiles)]
    profile: Option<String>,

    /// Example name — completes from examples/ without .rs (like cargo run --example)
    #[arg(section = "Misc", long = "example", complete = available_examples)]
    example: Option<String>,

    /// Binary name — completes from src/bin/ without .rs (like cargo run --bin)
    #[arg(section = "Misc", long = "bin", complete = available_bins)]
    bin: Option<String>,

    /// Skip — not parsed, always default
    #[arg(skip)]
    _runtime_state: Vec<String>,
}

fn main() {
    let opt = Opt::parse();
    let input = opt.resolved_alt_input();
    let output = opt.resolved_alt_output();
    let start = std::time::Instant::now();

    header!(
        "processor",
        env!("CARGO_PKG_VERSION"),
        "process and transform data files",
        "fast, validated, reliable"
    );

    let fmt = if opt.json {
        "json"
    } else if opt.csv {
        "csv"
    } else if opt.toml {
        "toml"
    } else {
        "default"
    };

    phase!("init", "reading {}", input.display());
    phase!("cfg", "format={} jobs={} port={}", fmt, opt.jobs, opt.port);

    if opt.dry_run {
        phase!("mode", "dry run — nothing will be written");
    }
    if opt.verbose {
        phase!("debug", "verbose output enabled");
    }
    if opt.sign {
        phase!("auth", "signing enabled, key={:?}", opt.key);
    }
    if opt.no_auth {
        phase!("auth", "authentication disabled");
    }
    if let Some(ref p) = opt.pattern {
        phase!("filter", "pattern: {p}");
    }
    if let Some(ref n) = opt.worker_name {
        phase!("worker", "name: {n}");
    }
    if let Some(ref p) = opt.profile {
        phase!("profile", "{p}");
    }
    if let Some(ref e) = opt.example {
        phase!("example", "{e}");
    }
    if let Some(ref b) = opt.bin {
        phase!("bin", "{b}");
    }

    if !opt.dry_run {
        let out = output.join(format!("result.{fmt}"));
        ok!(out.display());
    }

    let elapsed = start.elapsed().as_millis();
    summary! {
        done:    "Processing complete",
        "input"  => paint(CYAN, &input.display().to_string()),
        "output" => paint(CYAN, &output.display().to_string()),
        section,
        "format" => paint(OK,     fmt),
        "jobs"   => paint(OK,     &opt.jobs.to_string()),
        "port"   => paint(YELLOW, &opt.port.to_string()),
        "time"   => paint(DIM,    &format!("{elapsed}ms")),
    }
}
