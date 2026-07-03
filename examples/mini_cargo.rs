//! Example: mini_cargo — a cargo-like CLI emulator.
//!
//! Demonstrates subcommands, global flags, smart completions, and
//! all prompt types. Does NOT perform any real operations.
//!
//! Run:
//!   cargo run --example mini_cargo -- --help
//!   cargo run --example mini_cargo -- build --help
//!   cargo run --example mini_cargo -- run --example mini_cargo
//!   cargo run --example mini_cargo -- add serde --features derive
//!   cargo run --example mini_cargo -- init my-project
//!
//! Completions:
//!   cargo run --example mini_cargo -- --completions bash > /tmp/_cargo_mini
//!   source /tmp/_cargo_mini
//!   cargo-mini <Tab>

#![allow(dead_code)]

use cli_ui::styles::{paint, CYAN, DIM, OK};
use cli_ui::{bail, header, ok, phase, step, summary, CliCommand, CliOptions};
use std::path::PathBuf;

// ── Completion providers ──────────────────────────────────────────────────────

fn workspace_examples() -> Vec<String> {
    let Ok(rd) = std::fs::read_dir("examples") else {
        return Vec::new();
    };
    let mut v: Vec<String> = rd
        .flatten()
        .filter_map(|e| {
            let p = e.path();
            if p.is_file() && p.extension()?.to_str() == Some("rs") {
                Some(p.file_stem()?.to_string_lossy().to_string())
            } else {
                None
            }
        })
        .collect();
    v.sort();
    v
}

fn workspace_bins() -> Vec<String> {
    let Ok(rd) = std::fs::read_dir("src/bin") else {
        return Vec::new();
    };
    let mut v: Vec<String> = rd
        .flatten()
        .filter_map(|e| {
            let p = e.path();
            if p.is_file() && p.extension()?.to_str() == Some("rs") {
                Some(p.file_stem()?.to_string_lossy().to_string())
            } else {
                None
            }
        })
        .collect();
    v.sort();
    v
}

fn workspace_members() -> Vec<String> {
    // Read [workspace.members] from Cargo.toml if present
    let Ok(src) = std::fs::read_to_string("Cargo.toml") else {
        return Vec::new();
    };
    let mut members = Vec::new();
    let mut in_members = false;
    for line in src.lines() {
        let t = line.trim();
        if t == "members = [" || t.starts_with("members = [") {
            in_members = true;
        }
        if in_members {
            if t == "]" {
                break;
            }
            if let Some(s) = t
                .trim_matches(|c| c == '"' || c == ',' || c == '[' || c == ']')
                .split('"')
                .next()
            {
                let clean = s.trim().trim_matches('"').trim_matches(',').trim();
                if !clean.is_empty() && !clean.starts_with('[') {
                    members.push(clean.to_string());
                }
            }
        }
    }
    members
}

fn available_features() -> Vec<String> {
    // Parse [features] section from Cargo.toml
    let Ok(src) = std::fs::read_to_string("Cargo.toml") else {
        return Vec::new();
    };
    let mut features = Vec::new();
    let mut in_features = false;
    for line in src.lines() {
        let t = line.trim();
        if t == "[features]" {
            in_features = true;
            continue;
        }
        if in_features {
            if t.starts_with('[') {
                break;
            }
            if let Some(name) = t.split('=').next() {
                let n = name.trim();
                if !n.is_empty() && !n.starts_with('#') {
                    features.push(n.to_string());
                }
            }
        }
    }
    features
}

fn known_profiles() -> Vec<String> {
    vec![
        "dev".into(),
        "release".into(),
        "test".into(),
        "bench".into(),
    ]
}

fn known_targets() -> Vec<String> {
    vec![
        "x86_64-unknown-linux-gnu".into(),
        "x86_64-unknown-linux-musl".into(),
        "x86_64-apple-darwin".into(),
        "aarch64-apple-darwin".into(),
        "x86_64-pc-windows-msvc".into(),
        "wasm32-unknown-unknown".into(),
        "wasm32-wasi".into(),
        "aarch64-unknown-linux-gnu".into(),
    ]
}

fn known_editions() -> Vec<String> {
    vec!["2015".into(), "2018".into(), "2021".into(), "2024".into()]
}

fn known_vcs() -> Vec<String> {
    vec![
        "git".into(),
        "hg".into(),
        "pijul".into(),
        "fossil".into(),
        "none".into(),
    ]
}

// ── Global options ────────────────────────────────────────────────────────────

#[derive(CliOptions)]
#[cli(about = "global flags for mini-cargo")]
struct Global {
    /// Use verbose output
    #[arg(short = 'v', long = "verbose", negatable)]
    verbose: bool,

    /// Use quiet output (suppress most output)
    #[arg(short = 'q', long = "quiet", negatable)]
    quiet: bool,

    /// Coloring: auto, always, never
    #[arg(
        long = "color",
        default = "auto",
        validate(one_of("auto", "always", "never"))
    )]
    color: String,

    /// Path to Cargo.toml
    #[arg(long = "manifest-path", validate(is_file, ext("toml")))]
    manifest_path: Option<PathBuf>,

    /// Directory for all generated artifacts
    #[arg(long = "target-dir", validate(is_dir))]
    target_dir: Option<PathBuf>,
}

// ── Subcommand options ────────────────────────────────────────────────────────

/// Build the local package and all of its dependencies
#[derive(CliOptions)]
#[cli(
    name = "build",
    about = "Compile the current package",
    tagline = "all targets by default",
    example = "cargo-mini build --release",
    example = "cargo-mini build --target wasm32-unknown-unknown"
)]
struct BuildOpt {
    /// Build artifacts in release mode
    #[arg(section = "Build", short = 'r', long = "release")]
    release: bool,

    /// Build the specified example
    #[arg(section = "Build", long = "example", complete = workspace_examples)]
    example: Option<String>,

    /// Build the specified binary
    #[arg(section = "Build", long = "bin", complete = workspace_bins)]
    bin: Option<String>,

    /// Space or comma separated list of features to activate
    #[arg(section = "Features", short = 'F', long = "features",
          complete = available_features)]
    features: Option<String>,

    /// Activate all available features
    #[arg(section = "Features", long = "all-features")]
    all_features: bool,

    /// Do not activate the `default` feature
    #[arg(section = "Features", long = "no-default-features")]
    no_default_features: bool,

    /// Build for the target triple
    #[arg(section = "Target", long = "target", complete = known_targets)]
    target: Option<String>,

    /// Build profile to use
    #[arg(section = "Target", long = "profile", complete = known_profiles)]
    profile: Option<String>,

    /// Number of parallel jobs
    #[arg(
        section = "Performance",
        short = 'j',
        long = "jobs",
        default = "4",
        validate(range(1, 256))
    )]
    jobs: usize,

    /// Do not print cargo log messages
    #[arg(section = "Output", long = "quiet", negatable)]
    quiet: bool,

    /// Use verbose output
    #[arg(section = "Output", long = "verbose", negatable)]
    verbose: bool,
}

/// Run a binary or example of the local package
#[derive(CliOptions)]
#[cli(
    name = "run",
    about = "Run a binary or example of the local package",
    example = "cargo-mini run --example mini_cargo",
    example = "cargo-mini run --release -- --help"
)]
struct RunOpt {
    /// Name of the example to run
    #[arg(section = "Target", long = "example", complete = workspace_examples)]
    example: Option<String>,

    /// Name of the binary to run
    #[arg(section = "Target", long = "bin", complete = workspace_bins)]
    bin: Option<String>,

    /// Build in release mode
    #[arg(section = "Build", short = 'r', long = "release")]
    release: bool,

    /// Features to activate
    #[arg(section = "Features", short = 'F', long = "features",
          complete = available_features)]
    features: Option<String>,

    /// Build profile
    #[arg(section = "Build", long = "profile", complete = known_profiles)]
    profile: Option<String>,

    /// Target triple
    #[arg(section = "Build", long = "target", complete = known_targets)]
    target: Option<String>,

    /// Arguments passed to the binary (after --)
    #[arg(skip)]
    _extra_args: Vec<String>,
}

/// Add a dependency to Cargo.toml
#[derive(CliOptions)]
#[cli(
    name = "add",
    about = "Add dependencies to a Cargo.toml manifest file",
    example = "cargo-mini add serde --features derive",
    example = "cargo-mini add tokio --features full --no-default-features"
)]
struct AddOpt {
    /// Crate name(s) to add
    #[arg(positional)]
    crate_name: String,

    /// Features to enable for the dependency
    #[arg(section = "Dependency", short = 'F', long = "features")]
    features: Option<String>,

    /// Add as a dev-dependency
    #[arg(section = "Dependency", long = "dev")]
    dev: bool,

    /// Add as a build-dependency
    #[arg(section = "Dependency", long = "build")]
    build: bool,

    /// Package to modify (in workspace)
    #[arg(section = "Workspace", short = 'p', long = "package",
          complete = workspace_members)]
    package: Option<String>,

    /// Don't actually write to Cargo.toml
    #[arg(section = "Misc", long = "dry-run")]
    dry_run: bool,

    /// Do not activate the `default` feature
    #[arg(section = "Dependency", long = "no-default-features")]
    no_default_features: bool,
}

/// Remove a dependency from Cargo.toml
#[derive(CliOptions)]
#[cli(
    name = "remove",
    about = "Remove dependencies from a Cargo.toml manifest file",
    example = "cargo-mini remove serde"
)]
struct RemoveOpt {
    /// Crate name to remove
    #[arg(positional)]
    crate_name: String,

    /// Remove a dev-dependency
    #[arg(section = "Dependency", long = "dev")]
    dev: bool,

    /// Remove a build-dependency
    #[arg(section = "Dependency", long = "build")]
    build: bool,

    /// Package to modify
    #[arg(section = "Workspace", short = 'p', long = "package",
          complete = workspace_members)]
    package: Option<String>,
}

/// Create a new Cargo package
#[derive(CliOptions)]
#[cli(
    name = "new",
    about = "Create a new Cargo package",
    example = "cargo-mini new my-project --lib",
    example = "cargo-mini new my-cli --edition 2021 --vcs git"
)]
struct NewOpt {
    /// Package name / directory path
    #[arg(positional)]
    path: PathBuf,

    /// Create as a library package
    #[arg(section = "Template", long = "lib")]
    lib: bool,

    /// Create as a binary package (default)
    #[arg(section = "Template", long = "bin")]
    bin: bool,

    /// Edition to use
    #[arg(
        section = "Manifest",
        long = "edition",
        default = "2021",
        validate(one_of("2015", "2018", "2021", "2024"))
    )]
    edition: String,

    /// VCS to use
    #[arg(
        section = "Manifest",
        long = "vcs",
        validate(one_of("git", "hg", "pijul", "fossil", "none"))
    )]
    vcs: Option<String>,

    /// Set the package name (defaults to directory name)
    #[arg(section = "Manifest", long = "name")]
    name: Option<String>,
}

/// Create a new Cargo package in an existing directory
#[derive(CliOptions)]
#[cli(
    name = "init",
    about = "Create a new Cargo package in an existing directory",
    example = "cargo-mini init",
    example = "cargo-mini init my-dir --lib"
)]
struct InitOpt {
    /// Directory to use (defaults to current)
    #[arg(positional)]
    path: Option<PathBuf>,

    /// Create as a library
    #[arg(section = "Template", long = "lib")]
    lib: bool,

    /// Create as a binary (default)
    #[arg(section = "Template", long = "bin")]
    bin: bool,

    /// Edition to use
    #[arg(
        section = "Manifest",
        long = "edition",
        default = "2021",
        validate(one_of("2015", "2018", "2021", "2024"))
    )]
    edition: String,

    /// VCS to use
    #[arg(
        section = "Manifest",
        long = "vcs",
        validate(one_of("git", "hg", "pijul", "fossil", "none"))
    )]
    vcs: Option<String>,
}

/// Run the tests
#[derive(CliOptions)]
#[cli(
    name = "test",
    about = "Execute all unit and integration tests and build examples",
    example = "cargo-mini test --lib",
    example = "cargo-mini test integration_test -- --nocapture"
)]
struct TestOpt {
    /// Test name filter
    #[arg(positional)]
    filter: Option<String>,

    /// Test only this library's unit tests
    #[arg(section = "Target", long = "lib")]
    lib: bool,

    /// Test the specified example
    #[arg(section = "Target", long = "example", complete = workspace_examples)]
    example: Option<String>,

    /// Build in release mode
    #[arg(section = "Build", long = "release")]
    release: bool,

    /// Features to activate
    #[arg(section = "Features", short = 'F', long = "features",
          complete = available_features)]
    features: Option<String>,

    /// Number of parallel test threads
    #[arg(
        section = "Output",
        long = "test-threads",
        default = "4",
        validate(range(1, 256))
    )]
    test_threads: usize,

    /// Don't capture stdout/stderr of each test
    #[arg(section = "Output", long = "nocapture")]
    nocapture: bool,
}

/// Check that the current package compiles without producing binaries
#[derive(CliOptions)]
#[cli(
    name = "check",
    about = "Check a local package and all of its dependencies for errors",
    example = "cargo-mini check --all-targets"
)]
struct CheckOpt {
    /// Check all targets
    #[arg(section = "Target", long = "all-targets")]
    all_targets: bool,

    /// Features to activate
    #[arg(section = "Features", short = 'F', long = "features",
          complete = available_features)]
    features: Option<String>,

    /// Target triple
    #[arg(section = "Target", long = "target", complete = known_targets)]
    target: Option<String>,
}

/// Remove generated artifacts
#[derive(CliOptions)]
#[cli(
    name = "clean",
    about = "Remove artifacts that cargo has generated in the past",
    example = "cargo-mini clean --release"
)]
struct CleanOpt {
    /// Clean release artifacts
    #[arg(section = "Target", long = "release")]
    release: bool,

    /// Target triple
    #[arg(section = "Target", long = "target", complete = known_targets)]
    target: Option<String>,

    /// Package to clean
    #[arg(section = "Workspace", short = 'p', long = "package",
          complete = workspace_members)]
    package: Option<String>,
}

/// Display information about the current package
#[derive(CliOptions)]
#[cli(
    name = "metadata",
    about = "Output the resolved dependencies of the current package as JSON"
)]
struct MetadataOpt {
    /// Output format version
    #[arg(
        section = "Output",
        long = "format-version",
        default = "1",
        validate(one_of("1"))
    )]
    format_version: u32,

    /// Do not include dependencies of workspace members
    #[arg(section = "Output", long = "no-deps")]
    no_deps: bool,
}

/// Update dependencies as recorded in the lock file
#[derive(CliOptions)]
#[cli(
    name = "update",
    about = "Update dependencies as recorded in the lock file",
    example = "cargo-mini update serde"
)]
struct UpdateOpt {
    /// Specific crate to update
    #[arg(positional)]
    crate_name: Option<String>,

    /// Don't actually write the lock file
    #[arg(section = "Misc", long = "dry-run")]
    dry_run: bool,

    /// Update to pre-release versions
    #[arg(section = "Misc", long = "unstable-features")]
    unstable: bool,
}

/// Show information about a package
#[derive(CliOptions)]
#[cli(
    name = "info",
    about = "Display information about a package in the registry",
    example = "cargo-mini info serde"
)]
struct InfoOpt {
    /// Crate name to look up
    #[arg(positional)]
    crate_name: String,

    /// Show a specific version
    #[arg(section = "Filter", long = "version")]
    version: Option<String>,
}

/// Import a list of dependencies from a CSV manifest.
///
/// Demonstrates *validation-driven* smart completion: the `--file` flag
/// has no `complete = fn` provider — completion is derived entirely from
/// `validate(ext("csv"))`. When the user presses Tab after `--file`, the
/// shell only sees `.csv` files (plus directories, to drill into).
#[derive(CliOptions)]
#[cli(
    name = "import",
    about = "Import a list of dependencies from a CSV manifest",
    example = "cargo-mini import --file deps.csv"
)]
struct ImportOpt {
    /// CSV manifest with one `name,version` per line
    #[arg(
        section = "Input",
        long = "file",
        validate(exists, is_file, ext("csv"))
    )]
    file: PathBuf,

    /// Don't actually write to Cargo.toml
    #[arg(section = "Misc", long = "dry-run")]
    dry_run: bool,
}

// ── Top-level command enum ────────────────────────────────────────────────────

#[derive(CliCommand)]
#[cli(
    name    = "cargo-mini",
    about   = "The Rust package manager (emulator)",
    tagline = "no real side effects — safe to tab-complete",
    theme   = "yellow",
    global  = Global,
    example = "cargo-mini build --release",
    example = "cargo-mini run --example mini_cargo",
    example = "cargo-mini add tokio --features full",
)]
enum Cmd {
    /// Compile the current package
    Build(BuildOpt),
    /// Run a binary or example of the local package
    #[cli(alias = "r")]
    Run(RunOpt),
    /// Add a dependency to Cargo.toml
    Add(AddOpt),
    /// Remove a dependency from Cargo.toml
    #[cli(alias = "rm")]
    Remove(RemoveOpt),
    /// Create a new Cargo package
    New(NewOpt),
    /// Create a new Cargo package in an existing directory
    Init(InitOpt),
    /// Execute all unit and integration tests
    #[cli(alias = "t")]
    Test(TestOpt),
    /// Check a local package for errors without building
    Check(CheckOpt),
    /// Remove generated artifacts
    Clean(CleanOpt),
    /// Output resolved dependency information as JSON
    Metadata(MetadataOpt),
    /// Update dependencies as recorded in the lock file
    Update(UpdateOpt),
    /// Display information about a package in the registry
    Info(InfoOpt),
    /// Import dependencies from a CSV manifest
    Import(ImportOpt),
}

// ── main ──────────────────────────────────────────────────────────────────────

fn main() {
    let cmd = match Cmd::parse() {
        Ok(c) => c,
        Err(e) => bail!("{}", e),
    };
    let global = Cmd::global();

    header!(
        "cargo-mini",
        env!("CARGO_PKG_VERSION"),
        "The Rust package manager (emulator)",
        "no real side effects — safe to tab-complete",
        badge = cli_ui::styles::theme_styles("yellow").0
    );

    if global.verbose {
        phase!("verbose", "verbose output enabled");
    }

    match cmd {
        Cmd::Build(opt) => cmd_build(&opt, global),
        Cmd::Run(opt) => cmd_run(&opt, global),
        Cmd::Add(opt) => cmd_add(&opt, global),
        Cmd::Remove(opt) => cmd_remove(&opt, global),
        Cmd::New(opt) => cmd_new(&opt, global),
        Cmd::Init(opt) => cmd_init(&opt, global),
        Cmd::Test(opt) => cmd_test(&opt, global),
        Cmd::Check(opt) => cmd_check(&opt, global),
        Cmd::Clean(opt) => cmd_clean(&opt, global),
        Cmd::Metadata(opt) => cmd_metadata(&opt, global),
        Cmd::Update(opt) => cmd_update(&opt, global),
        Cmd::Info(opt) => cmd_info(&opt, global),
        Cmd::Import(opt) => cmd_import(&opt, global),
    }
}

// ── Subcommand handlers (all fake) ────────────────────────────────────────────

fn cmd_build(opt: &BuildOpt, global: &Global) {
    let mode = if opt.release { "release" } else { "dev" };
    phase!("compile", "Compiling packages in {} mode", mode);
    if let Some(ref e) = opt.example {
        phase!("target", "example = {e}");
    }
    if let Some(ref b) = opt.bin {
        phase!("target", "bin = {b}");
    }
    if let Some(ref t) = opt.target {
        phase!("target", "target = {t}");
    }
    if let Some(ref f) = opt.features {
        phase!("features", "{f}");
    }
    fake_compile(opt.jobs);
    ok!("target/debug/mini_cargo");
    summary! {
        done: "Build finished",
        "mode"    => paint(OK,     mode),
        "jobs"    => paint(DIM,    &opt.jobs.to_string()),
        "verbose" => paint(DIM,    &global.verbose.to_string()),
    }
}

fn cmd_run(opt: &RunOpt, global: &Global) {
    let mode = if opt.release { "release" } else { "dev" };
    phase!("compile", "Compiling in {} mode", mode);
    if let Some(ref e) = opt.example {
        phase!("run", "example = {e}");
    }
    if let Some(ref b) = opt.bin {
        phase!("run", "bin = {b}");
    }
    if let Some(ref f) = opt.features {
        phase!("features", "{f}");
    }
    fake_compile(4);
    phase!("running", "Running `target/{}/...`", mode);
    summary! {
        done: "Run complete",
        "mode"   => paint(OK, mode),
        "quiet"  => paint(DIM, &global.quiet.to_string()),
    }
}

fn cmd_add(opt: &AddOpt, _global: &Global) {
    let kind = if opt.dev {
        "dev-dependency"
    } else if opt.build {
        "build-dependency"
    } else {
        "dependency"
    };
    phase!("resolve", "Resolving {} `{}`", kind, opt.crate_name);
    if let Some(ref f) = opt.features {
        phase!("features", "enabling: {f}");
    }
    if opt.dry_run {
        phase!("dry-run", "would write to Cargo.toml (dry run)");
    } else {
        phase!("update", "Updating Cargo.toml");
        phase!("lock", "Updating Cargo.lock");
    }
    summary! {
        done: "Dependency added",
        "crate" => paint(CYAN, &opt.crate_name),
        "kind"  => paint(DIM,  kind),
    }
}

fn cmd_remove(opt: &RemoveOpt, _global: &Global) {
    phase!("remove", "Removing `{}` from Cargo.toml", opt.crate_name);
    summary! {
        done: "Dependency removed",
        "crate" => paint(CYAN, &opt.crate_name),
    }
}

fn cmd_new(opt: &NewOpt, _global: &Global) {
    let kind = if opt.lib { "library" } else { "binary" };
    let name = opt
        .name
        .as_deref()
        .or_else(|| opt.path.file_name()?.to_str())
        .unwrap_or("my-package");
    phase!("create", "Creating {} package `{}`", kind, name);
    phase!("edition", "edition = {}", opt.edition);
    if let Some(ref v) = opt.vcs {
        phase!("vcs", "initializing {} repo", v);
    }
    ok!(opt.path.display());
    summary! {
        done: "Package created",
        "name"    => paint(CYAN, name),
        "kind"    => paint(DIM,  kind),
        "edition" => paint(DIM,  &opt.edition),
    }
}

fn cmd_init(opt: &InitOpt, _global: &Global) {
    let dir = opt
        .path
        .as_deref()
        .unwrap_or_else(|| std::path::Path::new("."));
    let kind = if opt.lib { "library" } else { "binary" };
    phase!(
        "init",
        "Initializing {} package in `{}`",
        kind,
        dir.display()
    );
    if let Some(ref v) = opt.vcs {
        phase!("vcs", "initializing {} repo", v);
    }
    summary! {
        done: "Package initialized",
        "dir"     => paint(CYAN, &dir.display().to_string()),
        "edition" => paint(DIM,  &opt.edition),
    }
}

fn cmd_test(opt: &TestOpt, _global: &Global) {
    let mode = if opt.release { "release" } else { "dev" };
    phase!("compile", "Compiling test targets in {} mode", mode);
    if let Some(ref f) = opt.filter {
        phase!("filter", "running tests matching `{f}`");
    }
    if let Some(ref f) = opt.features {
        phase!("features", "{f}");
    }
    fake_compile(4);
    step!("running {} test thread(s)", opt.test_threads);
    eprintln!();
    eprintln!(
        "   {}  test result: {}. {} passed; 0 failed",
        paint(DIM, "│"),
        paint(OK, "ok"),
        paint(OK, "42"),
    );
    eprintln!();
    summary! {
        done: "Tests passed",
        "mode"    => paint(OK,     mode),
        "threads" => paint(DIM,    &opt.test_threads.to_string()),
        "passed"  => paint(OK,     "42"),
        "failed"  => paint(DIM,    "0"),
    }
}

fn cmd_check(opt: &CheckOpt, _global: &Global) {
    phase!("check", "Checking package");
    if opt.all_targets {
        phase!("targets", "checking all targets");
    }
    if let Some(ref f) = opt.features {
        phase!("features", "{f}");
    }
    if let Some(ref t) = opt.target {
        phase!("target", "{t}");
    }
    fake_compile(4);
    summary! { done: "Check passed" }
}

fn cmd_clean(opt: &CleanOpt, _global: &Global) {
    let mode = if opt.release { "release" } else { "debug" };
    phase!("clean", "Removing target/{}", mode);
    if let Some(ref p) = opt.package {
        phase!("package", "{p}");
    }
    summary! {
        done: "Clean complete",
        "mode" => paint(DIM, mode),
    }
}

fn cmd_metadata(opt: &MetadataOpt, _global: &Global) {
    phase!("metadata", "Resolving dependency graph");
    if opt.no_deps {
        phase!("filter", "excluding transitive dependencies");
    }
    eprintln!();
    eprintln!(
        "   {}  {{\"packages\":[...],\"workspace_root\":\"{}\"}}",
        paint(DIM, "│"),
        paint(DIM, "."),
    );
    eprintln!();
    summary! {
        done:    "Metadata output",
        "version" => paint(DIM, &opt.format_version.to_string()),
    }
}

fn cmd_update(opt: &UpdateOpt, _global: &Global) {
    if let Some(ref c) = opt.crate_name {
        phase!("update", "Updating `{}`", c);
    } else {
        phase!("update", "Updating all dependencies");
    }
    if opt.dry_run {
        phase!("dry-run", "would update Cargo.lock (dry run)");
    }
    summary! { done: "Lock file updated" }
}

fn cmd_info(opt: &InfoOpt, _global: &Global) {
    phase!("fetch", "Fetching info for `{}`", opt.crate_name);
    if let Some(ref v) = opt.version {
        phase!("version", "showing v{v}");
    }
    eprintln!();
    eprintln!(
        "   {}  {} v1.0.0",
        paint(DIM, "│"),
        paint(CYAN, &opt.crate_name)
    );
    eprintln!(
        "   {}  {}",
        paint(DIM, "│"),
        paint(DIM, "A well-known Rust crate (emulated)")
    );
    eprintln!(
        "   {}  https://crates.io/crates/{}",
        paint(DIM, "│"),
        opt.crate_name
    );
    eprintln!();
    summary! {
        done: "Info retrieved",
        "crate" => paint(CYAN, &opt.crate_name),
    }
}

fn cmd_import(opt: &ImportOpt, _global: &Global) {
    phase!("read", "Reading CSV manifest `{}`", opt.file.display());
    if opt.dry_run {
        phase!("dry-run", "would update Cargo.toml (dry run)");
    } else {
        phase!("update", "Updating Cargo.toml");
    }
    summary! {
        done: "Import complete",
        "file" => paint(CYAN, &opt.file.display().to_string()),
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn fake_compile(jobs: usize) {
    let pkgs = ["proc-macro2", "quote", "syn", "serde", "cli-ui"];
    step!("compiling {} packages ({} jobs)", pkgs.len(), jobs);
    eprintln!();
    for pkg in &pkgs {
        eprintln!(
            "   {}  {} {}",
            paint(DIM, "│"),
            paint(DIM, "Compiling"),
            paint(CYAN, pkg),
        );
    }
    eprintln!();
}
