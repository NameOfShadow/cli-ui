//! Example: subcommands — demonstrates #[derive(CliCommand)]
//!
//! mytool download <url>
//! mytool upload <file> <remote>
//! mytool net download <url>
//! mytool net ping
//! mytool status

use cli_ui::progress::format_bytes;
use cli_ui::styles::{paint, CYAN, DIM, OK, YELLOW};
use cli_ui::{header, ok, phase, step, summary, CliCommand, CliOptions, Result};
use std::path::PathBuf;

// ── Global options ────────────────────────────────────────────────────────────

#[derive(CliOptions)]
struct Global {
    /// Enable verbose output
    #[arg(short = 'v', long = "verbose", negatable)]
    verbose: bool,

    /// Path to config file
    #[arg(short = 'c', long = "config")]
    config: Option<PathBuf>,
}

// ── Command options ───────────────────────────────────────────────────────────

#[derive(CliOptions)]
#[cli(
    about = "download a file from URL",
    example = "mytool download https://example.com/file.csv",
    example = "mytool dl https://example.com/file.csv -o ./data/ -j 8"
)]
struct DownloadOpt {
    /// URL to download
    #[arg(positional)]
    url: String,

    /// Output directory
    #[arg(short = 'o', long = "output", default = ".")]
    output: PathBuf,

    /// Parallel connections
    #[arg(section = "Performance", short = 'j', long = "jobs", default = 4)]
    jobs: usize,
}

#[derive(CliOptions)]
#[cli(about = "upload a file to remote storage")]
struct UploadOpt {
    /// Local file path
    #[arg(positional)]
    file: PathBuf,

    /// Remote destination
    #[arg(positional)]
    remote: String,

    /// Compress before upload
    #[arg(section = "Transfer", short = 'z', long = "compress", negatable)]
    compress: bool,
}

#[derive(CliOptions)]
#[cli(
    about = "download from a network endpoint",
    example = "mytool net download https://example.com/data --timeout 60"
)]
struct NetDownloadOpt {
    /// Target URL
    #[arg(positional)]
    url: String,

    /// Timeout in seconds
    #[arg(long = "timeout", default = 30)]
    timeout: u32,
}

// ── Nested subcommands ────────────────────────────────────────────────────────

#[derive(CliCommand)]
#[cli(about = "network diagnostics and operations")]
enum NetCmd {
    /// Download from a network endpoint
    #[cli(alias = "dl")]
    Download(NetDownloadOpt), // mytool net download <url>
    // mytool net dl <url>
    /// Check network connectivity
    Ping, // mytool net ping
}

// ── Root subcommands ──────────────────────────────────────────────────────────

#[derive(CliCommand)]
#[cli(
    name    = "mytool",
    about   = "batch file processor",
    tagline = "fast and parallel",
    theme   = "cyan",
    global  = Global,
    example = "mytool download https://example.com/file.csv",
    example = "mytool -v upload ./report.pdf s3://bucket/report.pdf --compress",
    url     = "https://github.com/you/mytool",
)]
enum Cmd {
    /// Download a file from URL
    #[cli(alias = "dl")]
    Download(DownloadOpt), // mytool download <url> | mytool dl <url>

    /// Upload a file to remote storage
    #[cli(alias = "up")]
    Upload(UploadOpt), // mytool upload <file> <remote> | mytool up ...

    /// Network diagnostics
    #[cli(alias = "net")]
    Network(NetCmd), // mytool network ... | mytool net ...

    /// Show current status
    Status, // mytool status — unit variant, no options
}

// ── Handlers ──────────────────────────────────────────────────────────────────

fn download(g: &Global, opt: DownloadOpt) {
    header!(
        "mytool",
        env!("CARGO_PKG_VERSION"),
        "batch file processor",
        "fast and parallel"
    );
    phase!("download", "{}", opt.url);
    if g.verbose {
        phase!("debug", "output={} jobs={}", opt.output.display(), opt.jobs);
    }
    step!("fetching");
    ok!(opt.output.join("file.csv").display());
    summary! {
        done:   "Download complete",
        "url"    => paint(CYAN, &opt.url),
        "output" => paint(CYAN, &opt.output.display().to_string()),
        section,
        "size"   => paint(YELLOW, &format_bytes(43_100)),
        "time"   => paint(DIM, "320ms"),
    }
}

fn upload(g: &Global, opt: UploadOpt) {
    header!(
        "mytool",
        env!("CARGO_PKG_VERSION"),
        "batch file processor",
        "fast and parallel"
    );
    phase!("upload", "{} → {}", opt.file.display(), opt.remote);
    if g.verbose {
        phase!("debug", "compress={}", opt.compress);
    }
    step!("uploading");
    ok!(opt.remote.as_str());
    summary! {
        done:   "Upload complete",
        "file"   => paint(CYAN, &opt.file.display().to_string()),
        "remote" => paint(CYAN, &opt.remote),
    }
}

fn net_download(g: &Global, opt: NetDownloadOpt) {
    header!(
        "mytool",
        env!("CARGO_PKG_VERSION"),
        "batch file processor",
        "fast and parallel"
    );
    phase!("net-download", "{} (timeout: {}s)", opt.url, opt.timeout);
    if g.verbose {
        phase!("debug", "using network stack");
    }
    step!("fetching");
    ok!("./data/file.bin");
    summary! {
        done:  "Download complete",
        "url"  => paint(CYAN, &opt.url),
        "time" => paint(DIM, "120ms"),
    }
}

fn net_ping(g: &Global) {
    header!(
        "mytool",
        env!("CARGO_PKG_VERSION"),
        "batch file processor",
        "fast and parallel"
    );
    phase!("ping", "checking connectivity...");
    if g.verbose {
        phase!("debug", "sending ICMP");
    }
    summary! {
        done:      "Reachable",
        "latency"  => paint(OK, "12ms"),
    }
}

fn status(g: &Global) {
    header!(
        "mytool",
        env!("CARGO_PKG_VERSION"),
        "batch file processor",
        "fast and parallel"
    );
    phase!("status", "checking...");
    if g.verbose {
        phase!("debug", "reading config");
    }
    summary! {
        done:      "All systems operational",
        "version"  => paint(OK, env!("CARGO_PKG_VERSION")),
        "config"   => paint(DIM, &g.config.as_deref()
                        .map(|p| p.display().to_string())
                        .unwrap_or_else(|| "default".into())),
    }
}

// ── main ─────────────────────────────────────────────────────────────────────

fn main() -> Result<()> {
    match Cmd::parse()? {
        Cmd::Download(opt) => download(Cmd::global(), opt),
        Cmd::Upload(opt) => upload(Cmd::global(), opt),
        Cmd::Status => status(Cmd::global()),

        Cmd::Network(sub) => match sub {
            NetCmd::Download(opt) => net_download(Cmd::global(), opt),
            NetCmd::Ping => net_ping(Cmd::global()),
        },
    }

    Ok(())
}
