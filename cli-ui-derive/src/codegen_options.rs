//! Code generation for #[derive(CliOptions)].

use heck::ToKebabCase;
use proc_macro2::TokenStream as TS2;
use quote::{format_ident, quote};
use syn::{Data, DeriveInput, Field, Fields};

use crate::attrs::{
    is_option, is_vec, type_hint, unwrap_generic, Action, ArgAttrs, CliAttrs, DefaultVal, Validator,
};

pub fn impl_cli_options(input: &DeriveInput) -> syn::Result<TS2> {
    let sname = &input.ident;
    let cli = CliAttrs::parse(&input.attrs);
    let app_name = cli
        .name
        .clone()
        .unwrap_or_else(|| sname.to_string().to_kebab_case());
    let about = cli.about.unwrap_or_default();
    let tagline = cli.tagline.unwrap_or_default();
    let url = cli.url.unwrap_or_default();
    let examples = cli.examples;
    let hint = cli.hint.unwrap_or_default();
    let theme = cli.theme.unwrap_or_else(|| "cyan".to_string());

    let fields: Vec<&Field> = match &input.data {
        Data::Struct(ds) => match &ds.fields {
            Fields::Named(f) => f.named.iter().collect(),
            _ => {
                return Err(syn::Error::new_spanned(
                    input,
                    "only named fields supported",
                ))
            }
        },
        _ => return Err(syn::Error::new_spanned(input, "only structs supported")),
    };

    let parse_body = gen_parse(sname, &fields, &app_name, &about, &tagline, &theme)?;
    let help_body = gen_help(
        &fields, &app_name, &about, &tagline, &url, &examples, &hint, &theme,
    );
    let completions_b = gen_completions(&fields, &app_name);
    let resolvers = gen_resolvers(&fields);
    let parse_inner_b = gen_parse_inner(sname, &fields, &app_name, &about, &tagline);

    Ok(quote! {
        impl #sname {
            /// Parse CLI arguments from `std::env::args()`.
            pub fn parse() -> Self {
                #parse_body
            }

            /// Print styled `--help` output to stderr.
            pub fn help() {
                #help_body
            }

            /// Print shell completion script to stdout.
            pub fn completions(shell: &str) {
                #completions_b
            }

            #resolvers
        }

        impl ::cli_ui::CliOptions for #sname {
            fn parse_args(args: &[&str]) -> ::std::result::Result<Self, String> {
                #parse_inner_b
            }
        }

        impl ::cli_ui::ParseInner for #sname {
            fn parse_inner(args: &[&str]) -> ::cli_ui::Result<Self> {
                <Self as ::cli_ui::CliOptions>::parse_args(args)
                    .map_err(|e| ::cli_ui::CliError(e))
            }
        }

        impl ::cli_ui::PrintHelp for #sname {
            fn print_help() {
                #sname::help();
            }
        }

        impl ::cli_ui::NestedCompletions for #sname {
            fn print_completions(shell: &str) {
                #sname::completions(shell);
            }
        }
    })
}

// ─────────────────────────────────────────────────────────────────────────────
// parse() — from env::args, with --complete protocol intercept
// ─────────────────────────────────────────────────────────────────────────────

fn gen_parse(
    sname: &syn::Ident,
    fields: &[&Field],
    app_name: &str,
    _about: &str,
    _tagline: &str,
    theme: &str,
) -> syn::Result<TS2> {
    let complete_arms = gen_complete_dispatch(fields);

    Ok(quote! {
        let __raw: Vec<String> = ::std::env::args().skip(1).collect();

        if __raw.iter().any(|a| a == "--help" || a == "-h") {
            #sname::help();
            ::std::process::exit(0);
        }
        if __raw.iter().any(|a| a == "--version" || a == "-V") {
            let (__badge, _) = ::cli_ui::styles::theme_styles(#theme);
            ::cli_ui::print_version(#app_name, env!("CARGO_PKG_VERSION"), __badge);
            ::std::process::exit(0);
        }
        if let Some(__pos) = __raw.iter().position(|a| a == "--completions") {
            let __shell = __raw.get(__pos + 1).map(|s| s.as_str()).unwrap_or("bash");
            #sname::completions(__shell);
            ::std::process::exit(0);
        }

        // ── dynamic completion protocol ───────────────────────────────
        // Shell script calls: app --complete --flag <word>
        // We print one candidate per line to stdout, then exit 0.
        if let Some(__ci) = __raw.iter().position(|a| a == "--complete") {
            let __flag = __raw.get(__ci + 1).map(|s| s.as_str()).unwrap_or("");
            let __word = __raw.get(__ci + 2).map(|s| s.as_str()).unwrap_or("");
            match __flag {
                #(#complete_arms)*
                _ => {}
            }
            ::std::process::exit(0);
        }

        let __refs: Vec<&str> = __raw.iter().map(|s| s.as_str()).collect();
        <#sname as ::cli_ui::CliOptions>::parse_args(&__refs).unwrap_or_else(|e| {
            ::cli_ui::print_error(&e);
            eprintln!("  {}  {}",
                ::cli_ui::styles::paint(::cli_ui::styles::DIM, ::cli_ui::styles::ARROW),
                ::cli_ui::styles::paint(::cli_ui::styles::WHITE,
                    &format!("run `{} --help` for usage", #app_name)));
            eprintln!();
            ::std::process::exit(1);
        })
    })
}

// ─────────────────────────────────────────────────────────────────────────────
// Completion action — derived from validators + type at codegen time
// ─────────────────────────────────────────────────────────────────────────────

/// What kind of runtime completion a field needs.
#[derive(Clone)]
enum CompletionKind {
    /// Bool flag — no value to complete.
    None,
    /// Any file.
    Files,
    /// Files filtered by extension(s).  `strip` = cargo-style (remove ext).
    FilesExt { exts: Vec<String>, strip: bool },
    /// Directories only.
    Dirs,
    /// Fixed set of values known at compile time.
    Static(Vec<String>),
    /// User-supplied function called at completion time.
    Dynamic(proc_macro2::TokenStream),
    /// Files matching a glob pattern.
    Glob(String),
    /// Generic word — let the shell handle it.
    Word,
}

/// Derive the completion kind from `#[arg(validate(...), complete = fn)]`
/// and the Rust type of the field.
fn infer_completion(
    validators: &[Validator],
    complete_fn: &Option<String>,
    ty: &syn::Type,
) -> CompletionKind {
    // Explicit dynamic provider wins everything.
    if let Some(ref f) = complete_fn {
        let ts: proc_macro2::TokenStream = f.parse().unwrap_or_default();
        return CompletionKind::Dynamic(ts);
    }

    let mut exts: Option<Vec<String>> = None;
    let mut is_file = false;
    let mut is_dir = false;
    let mut one_of: Option<Vec<String>> = None;
    let mut glob_pat: Option<String> = None;

    for v in validators {
        match v {
            Validator::Ext(e) => {
                exts = Some(e.clone());
            }
            Validator::IsFile => {
                is_file = true;
            }
            Validator::IsDir => {
                is_dir = true;
            }
            Validator::OneOf(vs) => {
                one_of = Some(vs.clone());
            }
            Validator::Glob(p) => {
                glob_pat = Some(p.clone());
            }
            _ => {}
        }
    }

    // Priority order: one_of > dir > ext > is_file > glob > type inference
    if let Some(vals) = one_of {
        return CompletionKind::Static(vals);
    }
    if is_dir {
        return CompletionKind::Dirs;
    }
    if let Some(e) = exts {
        // Cargo-style strip only for single-ext String fields (e.g. --example NAME).
        // For PathBuf, keep the extension so the completed value passes validation.
        let ty_str = quote!(#ty).to_string().replace(' ', "");
        let is_pathbuf = ty_str.contains("PathBuf");
        let strip = e.len() == 1 && !is_pathbuf;
        return CompletionKind::FilesExt { exts: e, strip };
    }
    if is_file {
        return CompletionKind::Files;
    }
    if let Some(p) = glob_pat {
        return CompletionKind::Glob(p);
    }

    // Fall back to type
    let ty_str = quote!(#ty).to_string().replace(' ', "");
    // unwrap Option<T> / Vec<T>
    let inner =
        if (ty_str.starts_with("Option<") || ty_str.starts_with("Vec<")) && ty_str.ends_with('>') {
            &ty_str[ty_str.find('<').unwrap() + 1..ty_str.len() - 1]
        } else {
            ty_str.as_str()
        };

    match inner {
        "PathBuf" | "std::path::PathBuf" => CompletionKind::Files,
        "bool" => CompletionKind::None,
        _ => CompletionKind::Word,
    }
}

/// Emit the runtime code that prints candidates to stdout.
/// Returns `None` for kinds where the shell handles it natively (None/Word).
fn completion_runtime(kind: &CompletionKind) -> Option<TS2> {
    match kind {
        CompletionKind::None | CompletionKind::Word => None,

        CompletionKind::Files => Some(quote! {
            ::cli_ui::complete::print_values(
                &::cli_ui::complete::complete_files(__word)
            );
        }),

        CompletionKind::FilesExt { exts, strip } => {
            let ext_lits: Vec<&str> = exts.iter().map(|s| s.as_str()).collect();
            let s = *strip;
            Some(quote! {
                ::cli_ui::complete::print_values(
                    &::cli_ui::complete::complete_files_with_ext(
                        __word, &[#(#ext_lits),*], #s
                    )
                );
            })
        }

        CompletionKind::Dirs => Some(quote! {
            ::cli_ui::complete::print_values(
                &::cli_ui::complete::complete_dirs(__word)
            );
        }),

        CompletionKind::Static(vals) => Some(quote! {
            ::cli_ui::complete::print_values_filtered(&[#(#vals),*], __word);
        }),

        CompletionKind::Dynamic(fn_path) => Some(quote! {
            let __candidates: Vec<String> = #fn_path();
            ::cli_ui::complete::print_values_filtered(
                &__candidates.iter().map(|s| s.as_str()).collect::<Vec<_>>(),
                __word,
            );
        }),

        CompletionKind::Glob(pat) => Some(quote! {
            ::cli_ui::complete::print_values(
                &::cli_ui::complete::complete_files_from_glob(__word, #pat)
            );
        }),
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// --complete dispatch arms (embedded in parse())
// ─────────────────────────────────────────────────────────────────────────────

fn gen_complete_dispatch(fields: &[&Field]) -> Vec<TS2> {
    let mut arms: Vec<TS2> = Vec::new();

    // Build a map: field_name → validators for positional fields.
    // Used so that alt-flags (conflicts_with = "input") inherit ext() from
    // the positional they replace — e.g. alt_input gets csv/json from input.
    let positional_validators: std::collections::HashMap<String, Vec<Validator>> = fields
        .iter()
        .filter_map(|f| {
            let a = ArgAttrs::parse(f);
            if a.positional && !a.validators.is_empty() {
                let name = f.ident.as_ref()?.to_string();
                Some((name, a.validators.clone()))
            } else {
                None
            }
        })
        .collect();

    for field in fields {
        let arg = ArgAttrs::parse(field);
        if arg.skip || arg.positional {
            continue;
        }
        let Some(ref long) = arg.long else { continue };

        // Merge validators: own + inherited from conflicting positional.
        // This means --input gets ext("csv","json") from positional `input`.
        let mut effective_validators = arg.validators.clone();
        for conflict in &arg.conflicts_with {
            if let Some(pos_vals) = positional_validators.get(conflict) {
                // Only inherit Ext and IsDir — not Exists/IsFile (already covered)
                for v in pos_vals {
                    match v {
                        Validator::Ext(_) | Validator::IsDir | Validator::Glob(_)
                            if !effective_validators.iter().any(|ev| {
                                std::mem::discriminant(ev) == std::mem::discriminant(v)
                            }) =>
                        {
                            effective_validators.push(v.clone());
                        }
                        _ => {}
                    }
                }
            }
        }

        let kind = infer_completion(&effective_validators, &arg.complete, &field.ty);
        let body = match completion_runtime(&kind) {
            Some(b) => b,
            None => continue, // bool / word — nothing to emit
        };

        let flag = format!("--{long}");
        arms.push(quote! { #flag => { #body } });

        if let Some(s) = arg.short {
            let sf = format!("-{s}");
            let body2 = completion_runtime(&kind).unwrap();
            arms.push(quote! { #sf => { #body2 } });
        }
    }

    arms
}

// ─────────────────────────────────────────────────────────────────────────────
// completions() — smart shell scripts
// ─────────────────────────────────────────────────────────────────────────────
//
// Design:
//  • bool flags and `one_of` lists → static, baked into the script
//  • file/dir/glob/dynamic → the script calls `app --complete --flag <word>`
//    which hits the dispatch above and prints candidates
//
// Each shell gets a proper per-flag description so the user sees contextual
// hints in the completion menu.

fn gen_completions(fields: &[&Field], app_name: &str) -> TS2 {
    // ── collect per-field metadata ────────────────────────────────────
    struct FlagSpec {
        long: String,
        short: Option<char>,
        desc: String,
        is_bool: bool,
        kind: CompletionKind,
    }

    // Same positional validator inheritance as gen_complete_dispatch
    let positional_validators: std::collections::HashMap<String, Vec<Validator>> = fields
        .iter()
        .filter_map(|f| {
            let a = ArgAttrs::parse(f);
            if a.positional && !a.validators.is_empty() {
                let name = f.ident.as_ref()?.to_string();
                Some((name, a.validators.clone()))
            } else {
                None
            }
        })
        .collect();

    let mut specs: Vec<FlagSpec> = Vec::new();
    let mut all_flags: Vec<String> =
        vec!["--help".into(), "--version".into(), "--completions".into()];

    for field in fields {
        let arg = ArgAttrs::parse(field);
        if arg.skip || arg.positional {
            continue;
        }
        let Some(ref long) = arg.long else { continue };

        let is_bool = is_bool_type(&field.ty) || arg.negatable;

        // Inherit ext/is_dir/glob from conflicting positional
        let mut effective_validators = arg.validators.clone();
        for conflict in &arg.conflicts_with {
            if let Some(pos_vals) = positional_validators.get(conflict) {
                for v in pos_vals {
                    match v {
                        Validator::Ext(_) | Validator::IsDir | Validator::Glob(_)
                            if !effective_validators.iter().any(|ev| {
                                std::mem::discriminant(ev) == std::mem::discriminant(v)
                            }) =>
                        {
                            effective_validators.push(v.clone());
                        }
                        _ => {}
                    }
                }
            }
        }

        let kind = infer_completion(&effective_validators, &arg.complete, &field.ty);

        all_flags.push(format!("--{long}"));
        if arg.negatable {
            all_flags.push(format!("--no-{long}"));
        }

        specs.push(FlagSpec {
            long: long.clone(),
            short: arg.short,
            desc: arg.doc.clone(),
            is_bool,
            kind,
        });
    }

    let all_flags_str = all_flags.join(" ");

    // ── bash ─────────────────────────────────────────────────────────
    //
    // Generates a case statement so each flag gets its own completion:
    //
    //   case "$prev" in
    //     --input)   COMPREPLY=($( app --complete --input "$cur" )) ;;
    //     --format)  COMPREPLY=($(compgen -W "json csv toml" -- "$cur")) ;;
    //     *)         COMPREPLY=($(compgen -W "$opts" -- "$cur")) ;;
    //   esac

    // Each element is a (pattern, rhs) pair we'll format into the script.
    let bash_case_lines: Vec<TS2> = specs
        .iter()
        .map(|sp| {
            let flag = format!("--{}", sp.long);
            let rhs: TS2 = if sp.is_bool {
                quote! { "COMPREPLY=()" }
            } else {
                match &sp.kind {
                    CompletionKind::Static(vals) => {
                        let joined = vals.join(" ");
                        quote! {
                            &format!("COMPREPLY=($(compgen -W {:?} -- \"$cur\"))", #joined)
                        }
                    }
                    CompletionKind::None | CompletionKind::Word => {
                        quote! { "COMPREPLY=($(compgen -f -- \"$cur\"))" }
                    }
                    _ => {
                        // dynamic: delegate to --complete protocol
                        quote! {
                            &format!("COMPREPLY=($({} --complete {} \"$cur\" 2>/dev/null))",
                                #app_name, #flag)
                        }
                    }
                }
            };
            quote! {
                script.push_str(&format!("        {})\n            {}\n            ;;\n",
                    #flag, #rhs));
            }
        })
        .collect();

    // ── zsh ──────────────────────────────────────────────────────────
    //
    // Uses _arguments with per-flag action specs:
    //   '--input[Input file]:file:_files -g "*.csv"'
    //   '--format[Format]: :(json csv toml)'
    //   '--jobs[Workers]: :{app --complete --jobs $words[$CURRENT]}'

    // zsh single-quoted strings can't contain a literal `'`. Escape by
    // closing-quote-backslash-quote-reopening: `Don't` → `Don'\''t`.
    fn zsh_esc(s: &str) -> String {
        s.replace('\'', "'\\''")
    }

    let zsh_spec_stmts: Vec<TS2> = specs
        .iter()
        .map(|sp| {
            let flag = format!("--{}", sp.long);
            let desc = zsh_esc(&sp.desc);
            let spec_expr: TS2 = if sp.is_bool {
                quote! { format!("'{}[{}]'", #flag, #desc) }
            } else {
                match &sp.kind {
                    CompletionKind::Static(vals) => {
                        let joined = vals.join(" ");
                        quote! { format!("'{}[{}]: :({})'", #flag, #desc, #joined) }
                    }
                    CompletionKind::FilesExt { exts, .. } => {
                        // zsh glob pattern: "*.{csv,json}" or "*.csv"
                        let pat = if exts.len() == 1 {
                            format!("*.{}", exts[0])
                        } else {
                            format!("*.{{{}}}", exts.join(","))
                        };
                        quote! { format!("'{}[{}]:file:_files -g \"{}\"'", #flag, #desc, #pat) }
                    }
                    CompletionKind::Files => {
                        quote! { format!("'{}[{}]:file:_files'", #flag, #desc) }
                    }
                    CompletionKind::Dirs => {
                        quote! { format!("'{}[{}]:dir:_files -/'", #flag, #desc) }
                    }
                    CompletionKind::Glob(pat) => {
                        quote! { format!("'{}[{}]:file:_files -g \"{}\"'", #flag, #desc, #pat) }
                    }
                    _ => {
                        // Dynamic: inline command substitution. `__zsh_cmd` is a
                        // Rust string set just above this expansion — either
                        // `<app> --complete` (standalone zsh path) or
                        // `<parent> --complete <subcmd>` (nested via zsh-inner tag).
                        quote! {
                            format!("'{}[{}]: :{{{} {} $words[$CURRENT]}}'",
                                #flag, #desc, __zsh_cmd, #flag)
                        }
                    }
                }
            };
            quote! { __zsh_specs.push(#spec_expr); }
        })
        .collect();

    // ── fish ─────────────────────────────────────────────────────────
    //
    // One `complete` line per flag.  fish supports -F (force file) and
    // -a for static/dynamic argument lists.

    let fish_line_stmts: Vec<TS2> = specs.iter().map(|sp| {
        let long  = &sp.long;
        let desc  = &sp.desc;
        let short_part: TS2 = match sp.short {
            Some(s) => {
                let sf = s.to_string();
                quote! { format!("-s {} ", #sf) }
            }
            None => quote! { String::new() }
        };
        let args_part: TS2 = if sp.is_bool {
            quote! { String::new() }
        } else {
            match &sp.kind {
                CompletionKind::Static(vals) => {
                    let joined = vals.join(" ");
                    quote! { format!("-r -a '{}'", #joined) }
                }
                CompletionKind::FilesExt { exts, .. } => {
                    // fish: require value + force file completion;
                    // extension filtering via condition
                    let exts_re = exts.join("|");
                    quote! {
                        format!("-r -F -a '(string match -r \"\\.({})$\" (commandline -ct) > /dev/null; and __fish_complete_path (commandline -ct))'",
                            #exts_re)
                    }
                }
                CompletionKind::Files => {
                    quote! { "-r -F".to_string() }
                }
                CompletionKind::Dirs => {
                    quote! { "-r -a '(__fish_complete_directories (commandline -ct))'".to_string() }
                }
                _ => {
                    // dynamic: call --complete at completion time
                    let flag = format!("--{}", sp.long);
                    quote! {
                        format!("-r -a '({} --complete {} (commandline -ct) 2>/dev/null)'",
                            #app_name, #flag)
                    }
                }
            }
        };
        quote! {
            __fish_lines.push(format!(
                "complete -c {} -l {} {} -d '{}' {}",
                #app_name, #long, #short_part, #desc, #args_part
            ));
        }
    }).collect();

    // fish-subcmd lines conditioned on __fish_seen_subcommand_from
    let fish_subcmd_lines: Vec<TS2> = specs.iter().map(|sp| {
        let long       = &sp.long;
        let desc       = &sp.desc;
        let short_part: TS2 = match sp.short {
            Some(s) => { let sf = s.to_string(); quote! { format!("-s {} ", #sf) } }
            None    => quote! { String::new() }
        };
        let args_part: TS2 = if sp.is_bool {
            quote! { String::new() }
        } else {
            match &sp.kind {
                CompletionKind::Static(vals) => {
                    let joined = vals.join(" ");
                    quote! { format!("-r -a '{}'", #joined) }
                }
                CompletionKind::FilesExt { .. } | CompletionKind::Files => {
                    quote! { "-r -F".to_string() }
                }
                CompletionKind::Dirs => {
                    quote! { "-r -a '(__fish_complete_directories (commandline -ct))'".to_string() }
                }
                _ => {
                    let flag = format!("--{}", sp.long);
                    quote! {
                        format!("-r -a '({} --complete {} (commandline -ct) 2>/dev/null)'",
                            #app_name, #flag)
                    }
                }
            }
        };
        quote! {
            println!("complete -c {} -l {} {} -d '{}' {} --condition '__fish_seen_subcommand_from {}'",
                #app_name, #long, #short_part, #desc, #args_part, __subcmd_name);
        }
    }).collect();

    // complete dispatch arms reused here for the "complete:<flag> <word>" tag
    let complete_arms2 = gen_complete_dispatch(fields);

    quote! {
        match shell {
            // ── complete:<flag> <word> — called from CliCommand --complete dispatch ──
            // Protocol: shell = "complete:--flag word" or "complete: word" (root)
            s if s.starts_with("complete:") => {
                let __payload = &s["complete:".len()..];
                let __parts: Vec<&str> = __payload.splitn(2, ' ').collect();
                let __flag = __parts.first().copied().unwrap_or("");
                let __word = __parts.get(1).copied().unwrap_or("");
                match __flag {
                    #(#complete_arms2)*
                    _ => {}
                }
            }

            // ── fish-subcmd:<name> — called from codegen_command for fish inner flags ──
            s if s.starts_with("fish-subcmd:") => {
                let __subcmd_name = &s["fish-subcmd:".len()..];
                #(#fish_subcmd_lines)*
            }

            // ── zsh-inner:<parent> <sub> — called from codegen_command for nested ─────
            // Tag carries the parent app name + this subcommand name so that
            // dynamic dispatch can call `parent --complete sub --flag <word>`.
            s if s.starts_with("zsh-inner:") => {
                let __payload = &s["zsh-inner:".len()..];
                let __parts: Vec<&str> = __payload.splitn(2, ' ').collect();
                let __parent = __parts.first().copied().unwrap_or(#app_name);
                let __sub    = __parts.get(1).copied().unwrap_or("");
                let __zsh_cmd: String = if __sub.is_empty() {
                    format!("{} --complete", __parent)
                } else {
                    format!("{} --complete {}", __parent, __sub)
                };
                let __zsh_cmd = __zsh_cmd.as_str();
                let mut __zsh_specs: Vec<String> = Vec::new();
                __zsh_specs.push("'--help[Print help]'".to_string());
                #(#zsh_spec_stmts)*
                let args_block = __zsh_specs.join(" \\\n        ");
                println!("    _arguments \\");
                println!("        {}", args_block);
            }

            // ── bash ─────────────────────────────────────────────────
            "bash" => {
                let mut script = String::new();
                script.push_str(&format!(
                    "_{n}() {{\n    local cur prev opts\n    cur=\"${{COMP_WORDS[COMP_CWORD]}}\"\n    prev=\"${{COMP_WORDS[COMP_CWORD-1]}}\"\n    opts=\"{f}\"\n\n    case \"$prev\" in\n",
                    n = #app_name, f = #all_flags_str
                ));
                // per-flag case arms
                #(#bash_case_lines)*
                // default arm
                script.push_str(&format!(
                    "        *)\n            COMPREPLY=($(compgen -W \"$opts\" -- \"$cur\"))\n            ;;\n    esac\n}}\ncomplete -F _{n} {n}\n",
                    n = #app_name
                ));
                print!("{script}");
            }

            // ── zsh ──────────────────────────────────────────────────
            "zsh" => {
                let __zsh_cmd: String = format!("{} --complete", #app_name);
                let __zsh_cmd = __zsh_cmd.as_str();
                let mut __zsh_specs: Vec<String> = Vec::new();
                // meta flags
                __zsh_specs.push("'--help[Print help]'".to_string());
                __zsh_specs.push("'--version[Print version]'".to_string());
                __zsh_specs.push(
                    "'--completions[Generate completions]:shell:(bash zsh fish)'".to_string()
                );
                // per-field specs
                #(#zsh_spec_stmts)*
                let args_block = __zsh_specs.join(" \\\n    ");
                println!(
                    "#compdef {n}\n_{n}() {{\n    _arguments \\\n    {a}\n}}\ncompdef _{n} {n}\n",
                    n = #app_name, a = args_block
                );
            }

            // ── fish ─────────────────────────────────────────────────
            "fish" => {
                let mut __fish_lines: Vec<String> = Vec::new();
                // meta
                __fish_lines.push(format!(
                    "complete -c {n} -l help    -d 'Print help'", n = #app_name));
                __fish_lines.push(format!(
                    "complete -c {n} -l version -d 'Print version'", n = #app_name));
                __fish_lines.push(format!(
                    "complete -c {n} -l completions -r -a 'bash zsh fish' -d 'Generate shell completions'",
                    n = #app_name));
                // per-field lines
                #(#fish_line_stmts)*
                for __l in &__fish_lines { println!("{__l}"); }
            }

            other => {
                ::cli_ui::print_error(
                    &format!("unknown shell: `{other}`. Supported: bash, zsh, fish"));
                ::std::process::exit(1);
            }
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// parse_args(&[&str]) — unchanged from original
// ─────────────────────────────────────────────────────────────────────────────

fn gen_parse_inner(
    sname: &syn::Ident,
    fields: &[&Field],
    _app_name: &str,
    _about: &str,
    _tagline: &str,
) -> TS2 {
    let mut known_flags: Vec<TS2> = Vec::new();
    let mut field_inits: Vec<TS2> = Vec::new();
    let mut inter_checks: Vec<TS2> = Vec::new();
    let mut group_collect: Vec<TS2> = Vec::new();
    let mut group_check: Vec<TS2> = Vec::new();

    let all_idents: Vec<&syn::Ident> = fields.iter().map(|f| f.ident.as_ref().unwrap()).collect();

    let mut groups: std::collections::HashMap<String, Vec<String>> = Default::default();

    for field in fields {
        let arg = ArgAttrs::parse(field);
        if let Some(ref g) = arg.group {
            groups
                .entry(g.clone())
                .or_default()
                .push(field.ident.as_ref().unwrap().to_string());
        }
        if let Some(ref l) = arg.long {
            known_flags.push(quote! { #(format!("--{}", #l)) });
            if arg.negatable {
                known_flags.push(quote! { #(format!("--no-{}", #l)) });
            }
        }
        if let Some(s) = arg.short {
            let sf = format!("-{s}");
            known_flags.push(quote! { #sf });
        }
    }

    for field in fields {
        let arg = ArgAttrs::parse(field);
        let fname = field.ident.as_ref().unwrap();
        let ty = &field.ty;
        let fname_str = fname.to_string();
        let is_opt = is_option(ty);
        let is_v = is_vec(ty);

        if arg.skip {
            field_inits.push(quote! { let #fname: #ty = Default::default(); });
            continue;
        }

        let default_expr: TS2 = match &arg.default_val {
            Some(DefaultVal::Lit(s)) => {
                if is_opt {
                    quote! { None }
                } else {
                    quote! { #s.parse::<#ty>().unwrap_or_default() }
                }
            }
            Some(DefaultVal::Env { var, fallback }) => {
                let fb = fallback.as_deref().unwrap_or("");
                if is_opt {
                    let inner = unwrap_generic(ty);
                    quote! {
                        ::std::env::var(#var).ok()
                            .or_else(|| if #fb.is_empty() { None } else { Some(#fb.to_string()) })
                            .and_then(|v| v.parse::<#inner>().ok())
                    }
                } else {
                    quote! {
                        ::std::env::var(#var)
                            .unwrap_or_else(|_| #fb.to_string())
                            .parse::<#ty>()
                            .unwrap_or_default()
                    }
                }
            }
            Some(DefaultVal::Fn(path_str)) => {
                let path: proc_macro2::TokenStream = path_str.parse().unwrap();
                quote! { #path() }
            }
            None => {
                if is_opt {
                    quote! { None }
                } else if is_v {
                    quote! { Vec::new() }
                } else {
                    quote! { <#ty as ::std::default::Default>::default() }
                }
            }
        };

        let validators_code = gen_validators(&fname_str, &arg.validators, is_opt || is_v);
        let actions_code = gen_actions(&arg.actions);
        let warn_code = gen_warn_validators(&fname_str, &arg.validators);

        if arg.positional {
            let required_unless = &arg.required_unless;
            if is_opt {
                // Optional positional: parse inner type into Some, None when absent.
                let inner = unwrap_generic(ty);
                field_inits.push(quote! {
                    let #fname: #ty = match __pos_iter.next() {
                        Some(v) => {
                            let parsed: #inner = v.parse().map_err(|_|
                                format!("invalid value for <{}>: {:?}", #fname_str, v))?;
                            #actions_code
                            #validators_code
                            #warn_code
                            Some(parsed)
                        }
                        None => None,
                    };
                });
                continue;
            }
            field_inits.push(quote! {
                let #fname: #ty = match __pos_iter.next() {
                    Some(v) => {
                        let parsed: #ty = v.parse().map_err(|_|
                            format!("invalid value for <{}>: {:?}", #fname_str, v))?;
                        #actions_code
                        #validators_code
                        #warn_code
                        parsed
                    }
                    None => {
                        let mut __skip = false;
                        #( if __flags.contains(concat!("--", #required_unless)) ||
                              __flags.contains(concat!("-",  #required_unless)) {
                            __skip = true;
                        })*
                        if __skip { #default_expr }
                        else { return Err(format!("missing required argument <{}>", #fname_str)); }
                    }
                };
            });
            continue;
        }

        if arg.negatable {
            let long = arg
                .long
                .clone()
                .unwrap_or_else(|| fname_str.replace('_', "-"));
            let flag = format!("--{long}");
            let no_flag = format!("--no-{long}");
            let default = arg
                .default_val
                .as_ref()
                .map(|d| match d {
                    DefaultVal::Lit(s) => s != "false",
                    _ => true,
                })
                .unwrap_or(false);
            field_inits.push(quote! {
                let #fname: #ty =
                    if __flags.contains(#no_flag)   { false }
                    else if __flags.contains(#flag) { true  }
                    else                            { #default };
            });
            continue;
        }

        if is_v {
            let flag_key = arg
                .long
                .as_deref()
                .map(|l| format!("--{l}"))
                .or_else(|| arg.short.map(|s| format!("-{s}")));
            if let Some(flag) = flag_key {
                let inner = unwrap_generic(ty);
                field_inits.push(quote! {
                    let #fname: #ty = {
                        let raw_vals = __multi.get(#flag).cloned().unwrap_or_default();
                        let mut __collected: #ty = Vec::new();
                        for v in &raw_vals {
                            let item: #inner = v.parse().map_err(|_|
                                format!("invalid value for `{}`: {:?}", #flag, v))?;
                            __collected.push(item);
                        }
                        for item in &__collected {
                            let _ = item;
                            #validators_code
                        }
                        __collected
                    };
                });
            } else {
                field_inits.push(quote! { let #fname: #ty = Vec::new(); });
            }
            continue;
        }

        let flag_key = arg
            .long
            .as_deref()
            .map(|l| format!("--{l}"))
            .or_else(|| arg.short.map(|s| format!("-{s}")));

        if let Some(flag) = flag_key {
            if is_opt {
                let inner = unwrap_generic(ty);
                field_inits.push(quote! {
                    let #fname: #ty = match __kv.get(#flag) {
                        Some(v) => {
                            let parsed: #inner = v.parse().map_err(|_|
                                format!("invalid value for `{}`: {:?}", #flag, v))?;
                            #validators_code
                            #warn_code
                            #actions_code
                            Some(parsed)
                        }
                        None => None,
                    };
                });
            } else {
                field_inits.push(quote! {
                    let #fname: #ty = match __kv.get(#flag) {
                        Some(v) => {
                            let parsed: #ty = v.parse().map_err(|_|
                                format!("invalid value for `{}`: {:?}", #flag, v))?;
                            #validators_code
                            #warn_code
                            #actions_code
                            parsed
                        }
                        None => { let __d: #ty = { #default_expr }; __d }
                    };
                });
            }
            continue;
        }

        field_inits.push(quote! { let #fname: #ty = #default_expr; });
    }

    // ── inter-field constraints ───────────────────────────────────────
    for field in fields {
        let arg = ArgAttrs::parse(field);
        let fname = field.ident.as_ref().unwrap();
        let fname_str = fname.to_string();
        let flag_str = arg
            .long
            .as_deref()
            .map(|l| format!("--{l}"))
            .or_else(|| arg.short.map(|s| format!("-{s}")))
            .unwrap_or_else(|| format!("<{fname_str}>"));

        for other in &arg.conflicts_with {
            let other_flag = format!("--{}", other.replace('_', "-"));
            inter_checks.push(quote! {
                if __kv.contains_key(#flag_str) || __flags.contains(#flag_str) {
                    if __kv.contains_key(#other_flag) || __flags.contains(#other_flag) {
                        return Err(format!(
                            "conflicting flags: {} and {}\n\
                             │  {} cannot be used together with {}",
                            #flag_str, #other_flag, #flag_str, #other_flag
                        ));
                    }
                }
            });
        }

        for req in &arg.requires {
            let req_flag = format!("--{}", req.replace('_', "-"));
            inter_checks.push(quote! {
                if __kv.contains_key(#flag_str) || __flags.contains(#flag_str) {
                    if !__kv.contains_key(#req_flag) && !__flags.contains(#req_flag) {
                        return Err(format!(
                            "missing required flag: {}\n│  {} requires {} to be specified",
                            #req_flag, #flag_str, #req_flag
                        ));
                    }
                }
            });
        }

        if !arg.requires_any.is_empty() {
            let req_flags: Vec<String> = arg
                .requires_any
                .iter()
                .map(|r| format!("--{}", r.replace('_', "-")))
                .collect();
            inter_checks.push(quote! {
                if __kv.contains_key(#flag_str) || __flags.contains(#flag_str) {
                    let __any = false #(|| __kv.contains_key(#req_flags)
                                        || __flags.contains(#req_flags))*;
                    if !__any {
                        return Err(format!(
                            "missing required flag for {}: at least one of {:?} must be specified",
                            #flag_str, &[#(#req_flags),*]
                        ));
                    }
                }
            });
        }

        for req in &arg.required_unless {
            let req_flag = format!("--{}", req.replace('_', "-"));
            inter_checks.push(quote! {
                {
                    let __fp = __kv.contains_key(#flag_str) || __flags.contains(#flag_str);
                    let __up = __kv.contains_key(#req_flag) || __flags.contains(#req_flag);
                    if !__fp && !__up {
                        return Err(format!(
                            "missing required flag: {}\n│  {} is required unless {} is specified",
                            #flag_str, #flag_str, #req_flag
                        ));
                    }
                }
            });
        }

        if !arg.required_unless_any.is_empty() {
            let req_flags: Vec<String> = arg
                .required_unless_any
                .iter()
                .map(|r| format!("--{}", r.replace('_', "-")))
                .collect();
            inter_checks.push(quote! {
                {
                    let __fp  = __kv.contains_key(#flag_str) || __flags.contains(#flag_str);
                    let __any = false #(|| __kv.contains_key(#req_flags)
                                         || __flags.contains(#req_flags))*;
                    if !__fp && !__any {
                        return Err(format!(
                            "missing required flag: {}\n│  required unless one of {:?} is given",
                            #flag_str, &[#(#req_flags),*]
                        ));
                    }
                }
            });
        }

        if let Some(ref g) = arg.group {
            let gn = g.clone();
            let fns = fname.to_string();
            group_collect.push(quote! {
                if __kv.contains_key(#flag_str) || __flags.contains(#flag_str) {
                    __group_counts.entry(#gn.to_string()).or_default().push(#fns.to_string());
                }
            });
        }
    }

    for g in groups.keys() {
        group_check.push(quote! {
            if let Some(active) = __group_counts.get(#g) {
                if active.len() > 1 {
                    return Err(format!(
                        "conflicting flags in group `{}`: {} cannot be used together",
                        #g, active.join(", ")
                    ));
                }
            }
        });
    }

    let struct_fields: Vec<&syn::Ident> = all_idents.clone();

    quote! {
        let mut __kv:     ::std::collections::HashMap<&str, &str>           = Default::default();
        let mut __flags:  ::std::collections::HashSet<&str>                 = Default::default();
        let mut __multi:  ::std::collections::HashMap<&str, Vec<&str>>      = Default::default();
        let mut __pos:    Vec<&str>                                          = Vec::new();
        let mut __group_counts: ::std::collections::HashMap<String, Vec<String>> = Default::default();
        {
            let mut __i = 0usize;
            while __i < args.len() {
                let tok = args[__i];
                if let Some(eq) = tok.find('=').filter(|_| tok.starts_with("--")) {
                    let k = &tok[..eq]; let v = &tok[eq+1..];
                    __kv.insert(k, v);
                    __multi.entry(k).or_default().push(v);
                } else if tok.starts_with("--") || (tok.starts_with('-') && tok.len() == 2) {
                    if let Some(nx) = args.get(__i + 1) {
                        if !nx.starts_with('-') {
                            __kv.insert(tok, nx);
                            __multi.entry(tok).or_default().push(nx);
                            __i += 2;
                            continue;
                        }
                    }
                    __flags.insert(tok);
                } else if !tok.starts_with('-') {
                    __pos.push(tok);
                } else {
                    __flags.insert(tok);
                }
                __i += 1;
            }
        }
        let mut __pos_iter = __pos.into_iter();
        #(#field_inits)*
        #(#inter_checks)*
        #(#group_collect)*
        #(#group_check)*
        Ok(#sname { #(#struct_fields),* })
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Validator codegen — unchanged from original
// ─────────────────────────────────────────────────────────────────────────────

fn gen_validators(fname: &str, validators: &[Validator], _is_opt: bool) -> TS2 {
    let checks: Vec<TS2> = validators
        .iter()
        .filter(|v| !matches!(v, Validator::WarnIf(_)))
        .map(|v| gen_validator_check(fname, v))
        .collect();
    quote! { #(#checks)* }
}

fn gen_validator_check(fname: &str, v: &Validator) -> TS2 {
    match v {
        Validator::Exists => quote! {{
            use ::std::path::Path;
            let __p: &dyn ::std::convert::AsRef<Path> = &parsed;
            if !__p.as_ref().exists() {
                return Err(format!("path does not exist: {:?}", __p.as_ref()));
            }
        }},
        Validator::IsFile => quote! {{
            use ::std::path::Path;
            let __p: &dyn ::std::convert::AsRef<Path> = &parsed;
            if !__p.as_ref().is_file() {
                return Err(format!("not a file: {:?}", __p.as_ref()));
            }
        }},
        Validator::IsDir => quote! {{
            use ::std::path::Path;
            let __p: &dyn ::std::convert::AsRef<Path> = &parsed;
            if !__p.as_ref().is_dir() {
                return Err(format!("not a directory: {:?}", __p.as_ref()));
            }
        }},
        Validator::Ext(exts) => {
            let es: Vec<&str> = exts.iter().map(|s| s.as_str()).collect();
            quote! {{
                use ::std::path::Path;
                let __p: &dyn ::std::convert::AsRef<Path> = &parsed;
                let __ext = __p.as_ref().extension().and_then(|e| e.to_str()).unwrap_or("");
                let __ok  = &[#(#es),*];
                if !__ok.iter().any(|e| e.eq_ignore_ascii_case(__ext)) {
                    return Err(format!(
                        "invalid file extension for `{}`: {:?} — expected one of: {}",
                        #fname, __p.as_ref(), __ok.join(", ")
                    ));
                }
            }}
        }
        Validator::Range { min, max } => {
            let mn = min.as_ref().map(|m| {
                let t: proc_macro2::TokenStream = m.parse().unwrap();
                quote! { if parsed < (#t as _) {
                    return Err(format!("invalid value for `{}`: {} is below minimum {}", #fname, parsed, #t));
                }}
            });
            let mx = max.as_ref().map(|m| {
                let t: proc_macro2::TokenStream = m.parse().unwrap();
                quote! { if parsed > (#t as _) {
                    return Err(format!("invalid value for `{}`: {} exceeds maximum {}", #fname, parsed, #t));
                }}
            });
            quote! { #mn #mx }
        }
        Validator::OneOf(vals) => quote! {{
            let __ok = &[#(#vals),*];
            let __s  = format!("{}", parsed);
            if !__ok.contains(&__s.as_str()) {
                return Err(format!(
                    "invalid value for `{}`: {:?}\n│  expected one of: {}",
                    #fname, __s, __ok.join(", ")
                ));
            }
        }},
        Validator::MinLen(n) => quote! {{
            let __s = format!("{}", parsed);
            if __s.len() < #n {
                return Err(format!(
                    "invalid value for `{}`: too short ({} chars, minimum {})",
                    #fname, __s.len(), #n
                ));
            }
        }},
        Validator::MaxLen(n) => quote! {{
            let __s = format!("{}", parsed);
            if __s.len() > #n {
                return Err(format!(
                    "invalid value for `{}`: too long ({} chars, maximum {})",
                    #fname, __s.len(), #n
                ));
            }
        }},
        Validator::Glob(pattern) => quote! {{
            let __s = format!("{}", parsed);
            if !::cli_ui::glob_match(#pattern, &__s) {
                return Err(format!(
                    "invalid value for `{}`: {:?} does not match pattern `{}`",
                    #fname, __s, #pattern
                ));
            }
        }},
        Validator::Custom(path_str) => {
            let path: proc_macro2::TokenStream = path_str.parse().unwrap();
            quote! {{
                if let Err(__e) = #path(&parsed) {
                    return Err(format!("invalid value for `{}`: {}", #fname, __e));
                }
            }}
        }
        Validator::Env { var, fallback } => {
            let fb = fallback.as_deref().unwrap_or("");
            quote! {{
                let __ev = ::std::env::var(#var).unwrap_or_else(|_| #fb.to_string());
                let __s  = format!("{}", parsed);
                if !__ev.is_empty() && __s != __ev {
                    return Err(format!(
                        "invalid value for `{}`: {:?} does not match ${} ({:?})",
                        #fname, __s, #var, __ev
                    ));
                }
            }}
        }
        Validator::WarnIf(_) => quote! {},
    }
}

fn gen_warn_validators(fname: &str, validators: &[Validator]) -> TS2 {
    let warns: Vec<TS2> = validators
        .iter()
        .filter_map(|v| {
            let Validator::WarnIf(inner) = v else {
                return None;
            };
            let msg = match inner.as_ref() {
                Validator::Exists => "path already exists — may be overwritten",
                Validator::IsFile => "path is a file",
                Validator::IsDir => "path is a directory",
                _ => "condition met",
            };
            let cond = match inner.as_ref() {
                Validator::Exists => quote! {{
                    use ::std::path::Path;
                    let __p: &dyn ::std::convert::AsRef<Path> = &parsed;
                    __p.as_ref().exists()
                }},
                Validator::IsFile => quote! {{
                    use ::std::path::Path;
                    let __p: &dyn ::std::convert::AsRef<Path> = &parsed;
                    __p.as_ref().is_file()
                }},
                Validator::IsDir => quote! {{
                    use ::std::path::Path;
                    let __p: &dyn ::std::convert::AsRef<Path> = &parsed;
                    __p.as_ref().is_dir()
                }},
                _ => quote! { false },
            };
            Some(quote! {
                if #cond {
                    ::cli_ui::print_warning(&format!("warning for `{}`: {}", #fname, #msg));
                }
            })
        })
        .collect();
    quote! { #(#warns)* }
}

fn gen_actions(actions: &[Action]) -> TS2 {
    let stmts: Vec<TS2> = actions
        .iter()
        .map(|a| match a {
            Action::CreateDir => quote! {{
                use ::std::path::Path;
                let __p: &dyn ::std::convert::AsRef<Path> = &parsed;
                if !__p.as_ref().exists() {
                    ::std::fs::create_dir(__p.as_ref()).map_err(|e|
                        format!("failed to create directory {:?}: {}", __p.as_ref(), e))?;
                }
            }},
            Action::CreateDirAll => quote! {{
                use ::std::path::Path;
                let __p: &dyn ::std::convert::AsRef<Path> = &parsed;
                if !__p.as_ref().exists() {
                    ::std::fs::create_dir_all(__p.as_ref()).map_err(|e|
                        format!("failed to create directories {:?}: {}", __p.as_ref(), e))?;
                }
            }},
        })
        .collect();
    quote! { #(#stmts)* }
}

fn is_bool_type(ty: &syn::Type) -> bool {
    quote!(#ty).to_string().replace(' ', "") == "bool"
}

// ─────────────────────────────────────────────────────────────────────────────
// resolved_*() helpers — unchanged from original
// ─────────────────────────────────────────────────────────────────────────────

fn gen_resolvers(fields: &[&Field]) -> TS2 {
    let mut methods: Vec<TS2> = Vec::new();
    let field_types: std::collections::HashMap<String, &syn::Type> = fields
        .iter()
        .map(|f| (f.ident.as_ref().unwrap().to_string(), &f.ty))
        .collect();

    for field in fields {
        let arg = ArgAttrs::parse(field);
        if arg.conflicts_with.is_empty() || !is_option(&field.ty) {
            continue;
        }

        let opt_field = field.ident.as_ref().unwrap();
        let inner_ty = unwrap_generic(&field.ty);
        let target_name = &arg.conflicts_with[0];
        let target = format_ident!("{}", target_name);

        let Some(target_ty) = field_types.get(target_name.as_str()) else {
            continue;
        };
        if quote!(#inner_ty).to_string().replace(' ', "")
            != quote!(#target_ty).to_string().replace(' ', "")
        {
            continue;
        }

        let method = format_ident!("resolved_{}", opt_field);
        methods.push(quote! {
            /// Returns the flag value if provided, otherwise falls back to the positional.
            pub fn #method(&self) -> &#inner_ty {
                self.#opt_field.as_ref().unwrap_or(&self.#target)
            }
        });
    }
    quote! { #(#methods)* }
}

// ─────────────────────────────────────────────────────────────────────────────
// help() — unchanged from original
// ─────────────────────────────────────────────────────────────────────────────

fn gen_help(
    fields: &[&Field],
    name: &str,
    about: &str,
    tagline: &str,
    url: &str,
    examples: &[String],
    hint: &str,
    theme: &str,
) -> TS2 {
    let mut pos_entries: Vec<TS2> = Vec::new();
    let mut section_stmts: Vec<TS2> = Vec::new();
    let mut seen: Vec<String> = Vec::new();

    for field in fields {
        let arg = ArgAttrs::parse(field);
        if arg.skip {
            continue;
        }

        if arg.positional {
            let label = format!(
                "<{}>",
                field.ident.as_ref().unwrap().to_string().to_uppercase()
            );
            let doc = &arg.doc;
            pos_entries.push(quote! {
                ::cli_ui::help::HelpEntry::Pair { key: #label.to_string(), desc: #doc.to_string() }
            });
            continue;
        }

        let Some(ref sec) = arg.section else { continue };
        if !seen.contains(sec) {
            seen.push(sec.clone());
            section_stmts.push(quote! {
                __secs.push(::cli_ui::help::HelpSection {
                    title:   Box::leak(#sec.to_string().into_boxed_str()),
                    entries: Vec::new(),
                });
            });
        }

        let flag_label = match (&arg.short, &arg.long, arg.negatable, arg.multi) {
            (Some(s), Some(l), true, _) => format!("-{s}, --{l} / --no-{l}"),
            (Some(s), Some(l), false, true) => format!("-{s}, --{l} <N>..."),
            (Some(s), Some(l), false, _) => format!("-{s}, --{l}"),
            (None, Some(l), true, _) => format!("    --{l} / --no-{l}"),
            (None, Some(l), false, true) => format!("    --{l} <N>..."),
            (None, Some(l), false, _) => {
                let th = type_hint(&field.ty);
                if th.is_empty() {
                    format!("    --{l}")
                } else {
                    format!("    --{l} <{th}>")
                }
            }
            (Some(s), None, _, _) => format!("-{s}"),
            _ => continue,
        };

        let mut desc_parts = vec![arg.doc.clone()];
        if let Some(ref d) = arg.default_val {
            desc_parts.push(match d {
                DefaultVal::Lit(s) => format!("[default: {s}]"),
                DefaultVal::Env { var, fallback } => fallback
                    .as_ref()
                    .map(|fb| format!("[default: ${var} or {fb}]"))
                    .unwrap_or_else(|| format!("[default: ${var}]")),
                DefaultVal::Fn(_) => "[default: auto]".to_string(),
            });
        }
        let desc = desc_parts
            .into_iter()
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join("  ");

        section_stmts.push(quote! {
            __secs.last_mut().unwrap().entries.push(::cli_ui::help::HelpEntry::Pair {
                key:  #flag_label.to_string(),
                desc: #desc.to_string(),
            });
        });
    }

    let pos_names: Vec<String> = fields
        .iter()
        .filter_map(|f| {
            let a = ArgAttrs::parse(f);
            if a.positional {
                Some(format!(
                    "<{}>",
                    f.ident.as_ref().unwrap().to_string().to_uppercase()
                ))
            } else {
                None
            }
        })
        .collect();
    let usage = format!("{} {} [OPTIONS]", name, pos_names.join(" "));
    let example_lits: Vec<TS2> = examples.iter().map(|e| quote! { #e }).collect();

    quote! {
        let __pos_sec = ::cli_ui::help::HelpSection {
            title:   "Arguments",
            entries: vec![ #(#pos_entries),* ],
        };
        let mut __secs: Vec<::cli_ui::help::HelpSection> = Vec::new();
        #(#section_stmts)*
        __secs.push(::cli_ui::help::HelpSection {
            title: "Meta",
            entries: vec![
                ::cli_ui::help::HelpEntry::Pair { key: "-h, --help".into(),    desc: "Print this help".into() },
                ::cli_ui::help::HelpEntry::Pair { key: "-V, --version".into(), desc: "Print version".into() },
                ::cli_ui::help::HelpEntry::Pair {
                    key:  "    --completions <SHELL>".into(),
                    desc: "Generate completions (bash, zsh, fish)".into(),
                },
            ],
        });
        let mut __all = vec![__pos_sec];
        __all.extend(__secs);
        let (__badge, __accent) = ::cli_ui::styles::theme_styles(#theme);
        ::cli_ui::help::render(
            #name, env!("CARGO_PKG_VERSION"), #about, #tagline, #url,
            &#usage, &__all, &[#(#example_lits),*], #hint, __badge, __accent,
        );
    }
}
