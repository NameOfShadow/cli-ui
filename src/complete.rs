//! Runtime helpers for smart shell completions.
//!
//! These functions run **at completion time** inside the compiled binary when
//! the shell calls `app --complete --flag <word>`.  They are never invoked
//! during normal program execution.
//!
//! # Dynamic completion protocol
//!
//! The generated shell scripts use a two-phase strategy:
//!
//! **Static** (baked into the script at compile time):
//! - Flag names, bool flags, `one_of` value lists, range hints.
//!
//! **Dynamic** (called at completion time):
//! - Filesystem queries filtered by extension / directory.
//! - User-supplied `complete = fn_name` providers.
//!
//! The shell script calls:
//! ```text
//! app --complete --flag <current-word>
//! ```
//! The app detects `--complete`, dispatches to the right handler, prints
//! one candidate per line to **stdout**, then exits 0.  The shell captures
//! that output as the completion list.

use std::path::Path;

// ─────────────────────────────────────────────────────────────────────────────
// File completion
// ─────────────────────────────────────────────────────────────────────────────

/// List files under the directory implied by `prefix`, keeping only those
/// whose extension is in `exts` (case-insensitive, without the dot).
///
/// - `prefix`    — what the user has typed so far (may include a dir part).
/// - `exts`      — allowed extensions, e.g. `["csv", "json"]`. Empty = any.
/// - `strip_ext` — when `true`, the extension is removed from the suggestion
///   (cargo style: `--example foo` instead of `foo.rs`).
///
/// Directories are always suggested so the user can drill into them.
pub fn complete_files_with_ext(prefix: &str, exts: &[&str], strip_ext: bool) -> Vec<String> {
    let (dir, file_prefix) = split_prefix(prefix);
    let search = if dir.is_empty() {
        Path::new(".")
    } else {
        Path::new(&dir)
    };

    let Ok(rd) = std::fs::read_dir(search) else {
        return Vec::new();
    };

    let mut out = Vec::new();
    for entry in rd.flatten() {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if !name.starts_with(&*file_prefix) {
            continue;
        }

        let path = entry.path();
        if path.is_dir() {
            let full = join_prefix(&dir, &format!("{}/", name));
            out.push(full);
            continue;
        }

        if !exts.is_empty() {
            let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
            if !exts.iter().any(|&e| e.eq_ignore_ascii_case(ext)) {
                continue;
            }
        }

        let display = if strip_ext {
            path.file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or(&name)
                .to_string()
        } else {
            name.to_string()
        };

        out.push(join_prefix(&dir, &display));
    }
    out.sort();
    out
}

/// List only directories matching `prefix`.
pub fn complete_dirs(prefix: &str) -> Vec<String> {
    let (dir, file_prefix) = split_prefix(prefix);
    let search = if dir.is_empty() {
        Path::new(".")
    } else {
        Path::new(&dir)
    };

    let Ok(rd) = std::fs::read_dir(search) else {
        return Vec::new();
    };

    let mut out: Vec<String> = rd
        .flatten()
        .filter(|e| e.path().is_dir())
        .map(|e| e.file_name().to_string_lossy().to_string())
        .filter(|n| n.starts_with(&*file_prefix))
        .map(|n| join_prefix(&dir, &format!("{n}/")))
        .collect();
    out.sort();
    out
}

/// List all files (no extension filter).
pub fn complete_files(prefix: &str) -> Vec<String> {
    complete_files_with_ext(prefix, &[], false)
}

/// Infer allowed extensions from a glob pattern (`"*.csv"`, `"*.{rs,toml}"`)
/// and delegate to [`complete_files_with_ext`].
pub fn complete_files_from_glob(prefix: &str, pattern: &str) -> Vec<String> {
    let exts = extract_exts_from_glob(pattern);
    let refs: Vec<&str> = exts.iter().map(|s| s.as_str()).collect();
    complete_files_with_ext(prefix, &refs, false)
}

// ─────────────────────────────────────────────────────────────────────────────
// Internal helpers
// ─────────────────────────────────────────────────────────────────────────────

/// Split `"./src/ma"` → `("./src", "ma")`.  `"src/"` → `("src", "")`.
fn split_prefix(prefix: &str) -> (String, String) {
    if prefix.is_empty() {
        return (String::new(), String::new());
    }
    if prefix.ends_with('/') {
        return (prefix.trim_end_matches('/').to_string(), String::new());
    }
    let p = std::path::Path::new(prefix);
    let dir = p
        .parent()
        .map(|d| d.to_string_lossy().to_string())
        .unwrap_or_default();
    let file = p
        .file_name()
        .map(|f| f.to_string_lossy().to_string())
        .unwrap_or_default();
    (dir, file)
}

fn join_prefix(dir: &str, name: &str) -> String {
    if dir.is_empty() || dir == "." {
        name.to_string()
    } else {
        format!("{}/{}", dir.trim_end_matches('/'), name)
    }
}

fn extract_exts_from_glob(pattern: &str) -> Vec<String> {
    if let Some(dot) = pattern.rfind('.') {
        let after = &pattern[dot + 1..];
        if after.starts_with('{') && after.ends_with('}') {
            return after[1..after.len() - 1]
                .split(',')
                .map(|s| s.trim().to_string())
                .collect();
        }
        if !after.chars().any(|c| "*?[]{".contains(c)) {
            return vec![after.to_string()];
        }
    }
    Vec::new()
}

// ─────────────────────────────────────────────────────────────────────────────
// Output helpers
// ─────────────────────────────────────────────────────────────────────────────

/// Print one completion candidate per line to stdout.
pub fn print_values(values: &[String]) {
    for v in values {
        println!("{v}");
    }
}

/// Print one completion candidate per line, filtered by `word` prefix.
pub fn print_values_filtered(values: &[&str], word: &str) {
    for v in values {
        if v.starts_with(word) {
            println!("{v}");
        }
    }
}
