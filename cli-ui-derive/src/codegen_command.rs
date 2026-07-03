//! Code generation for #[derive(CliCommand)].

use heck::ToKebabCase;
use proc_macro2::TokenStream as TS2;
use quote::{format_ident, quote};
use syn::{Data, DeriveInput, Fields, Type};

use crate::attrs::{collect_doc, ident_str, lit_str, CliAttrs};

// ─────────────────────────────────────────────────────────────────────────────
// Variant-level attrs
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Default)]
struct CmdVariantAttrs {
    about: Option<String>,
    aliases: Vec<String>,
}

impl CmdVariantAttrs {
    fn parse(attrs: &[syn::Attribute]) -> Self {
        let mut out = Self::default();
        for attr in attrs {
            if !attr.path().is_ident("cli") {
                continue;
            }
            let _ = attr.parse_nested_meta(|m| {
                let key = ident_str(&m.path);
                match key.as_str() {
                    "about" => {
                        out.about = Some(lit_str(&m.value()?.parse::<syn::Lit>()?));
                    }
                    "alias" => {
                        out.aliases.push(lit_str(&m.value()?.parse::<syn::Lit>()?));
                    }
                    _ => {}
                }
                Ok(())
            });
        }
        out
    }
}

struct VariantMeta {
    ident: syn::Ident,
    cmd_name: String,
    aliases: Vec<String>,
    about: String,
    inner_type: Option<Type>,
}

// ─────────────────────────────────────────────────────────────────────────────
// Entry point
// ─────────────────────────────────────────────────────────────────────────────

pub fn impl_cli_command(input: &DeriveInput) -> syn::Result<TS2> {
    let enum_name = &input.ident;
    let attrs = CliAttrs::parse(&input.attrs);

    let app_name = attrs
        .name
        .clone()
        .unwrap_or_else(|| enum_name.to_string().to_kebab_case());
    let about = attrs
        .about
        .clone()
        .unwrap_or_else(|| collect_doc(&input.attrs));
    let tagline = attrs.tagline.clone().unwrap_or_default();
    let url = attrs.url.clone().unwrap_or_default();
    let hint = attrs.hint.clone().unwrap_or_default();
    let theme = attrs.theme.clone().unwrap_or_else(|| "cyan".to_string());
    let examples = attrs.examples.clone();

    let variants = match &input.data {
        Data::Enum(de) => de.variants.iter().cloned().collect::<Vec<_>>(),
        _ => {
            return Err(syn::Error::new_spanned(
                input,
                "#[derive(CliCommand)] only works on enums",
            ))
        }
    };

    // ── collect + validate variants ───────────────────────────────────
    let mut metas: Vec<VariantMeta> = Vec::new();
    let mut all_names: Vec<(String, syn::Ident)> = Vec::new();

    for v in &variants {
        let va = CmdVariantAttrs::parse(&v.attrs);
        let cmd_name = v.ident.to_string().to_kebab_case();
        let about = va.about.clone().unwrap_or_else(|| collect_doc(&v.attrs));
        let inner_type = match &v.fields {
            Fields::Unnamed(f) if f.unnamed.len() == 1 => {
                Some(f.unnamed.first().unwrap().ty.clone())
            }
            Fields::Unit => None,
            _ => {
                return Err(syn::Error::new_spanned(
                    &v.ident,
                    "CliCommand variants must be unit or single-tuple: Cmd or Cmd(Opt)",
                ))
            }
        };

        if let Some((_, prev)) = all_names.iter().find(|(n, _)| n == &cmd_name) {
            return Err(syn::Error::new_spanned(
                &v.ident,
                format!("command name `{cmd_name}` conflicts with variant `{prev}`"),
            ));
        }
        all_names.push((cmd_name.clone(), v.ident.clone()));

        for alias in &va.aliases {
            if let Some((_, prev)) = all_names.iter().find(|(n, _)| n == alias) {
                return Err(syn::Error::new_spanned(
                    &v.ident,
                    format!("alias `{alias}` conflicts with `{prev}`"),
                ));
            }
            all_names.push((alias.clone(), v.ident.clone()));
        }

        metas.push(VariantMeta {
            ident: v.ident.clone(),
            cmd_name,
            aliases: va.aliases,
            about,
            inner_type,
        });
    }

    let has_global = attrs.global.is_some();
    let global_type = attrs
        .global
        .clone()
        .map(|p| quote! { #p })
        .unwrap_or_else(|| quote! { () });

    let static_name = format_ident!("__CLI_GLOBAL_{}", enum_name.to_string().to_uppercase());
    let once_lock = if has_global {
        quote! {
            static #static_name: ::std::sync::OnceLock<#global_type> =
                ::std::sync::OnceLock::new();
        }
    } else {
        quote! {}
    };

    let parse_impl = gen_parse(
        enum_name,
        &metas,
        &app_name,
        &about,
        &tagline,
        &theme,
        &examples,
        &hint,
        &url,
        has_global,
        &global_type,
        &static_name,
    )?;
    let help_impl = gen_help(
        &metas, &app_name, &about, &tagline, &url, &examples, &hint, &theme, has_global,
    );
    let completions_impl = gen_completions(&metas, &app_name);

    let global_method = if has_global {
        quote! {
            fn global() -> &'static Self::Global {
                #static_name.get()
                    .expect("cli_ui: global() called before parse()")
            }
        }
    } else {
        quote! {
            fn global() -> &'static Self::Global { &() }
        }
    };

    let dispatch_arms: Vec<TS2> = metas
        .iter()
        .map(|m| {
            let vident = &m.ident;
            let name = &m.cmd_name;
            let aliases = &m.aliases;
            let pattern = if aliases.is_empty() {
                quote! { #name }
            } else {
                quote! { #name #(| #aliases)* }
            };
            if let Some(ref inner_ty) = m.inner_type {
                quote! {
                    #pattern => {
                        <#inner_ty as ::cli_ui::ParseInner>::parse_inner(&rest)
                            .map(|inner| Self::#vident(inner))
                    }
                }
            } else {
                quote! {
                    #pattern => Ok(Self::#vident),
                }
            }
        })
        .collect();

    Ok(quote! {
        #once_lock

        impl ::cli_ui::CliCommand for #enum_name {
            type Global = #global_type;

            fn parse() -> ::cli_ui::Result<Self> {
                #parse_impl
            }

            #global_method

            fn help() {
                #help_impl
            }
        }

        impl #enum_name {
            /// Generate shell completions for the full command tree.
            pub fn completions(shell: &str) {
                #completions_impl
            }
        }

        impl ::cli_ui::ParseInner for #enum_name {
            fn parse_inner(args: &[&str]) -> ::cli_ui::Result<Self> {
                let cmd_pos = args.iter().position(|a| !a.starts_with('-'));
                let (cmd, rest): (&str, Vec<&str>) = match cmd_pos {
                    Some(i) => (
                        args[i],
                        args[..i].iter().chain(args[i+1..].iter()).copied().collect(),
                    ),
                    None => {
                        return Err(::cli_ui::CliError("missing subcommand".to_string()));
                    }
                };
                match cmd {
                    #(#dispatch_arms)*
                    other => Err(::cli_ui::CliError(
                        format!("unknown subcommand: `{other}`")
                    )),
                }
            }
        }

        impl ::cli_ui::PrintHelp for #enum_name {
            fn print_help() {
                #enum_name::help();
            }
        }

        impl ::cli_ui::NestedCompletions for #enum_name {
            fn print_completions(shell: &str) {
                #enum_name::completions(shell);
            }
        }
    })
}

// ─────────────────────────────────────────────────────────────────────────────
// parse() — with --complete protocol intercept for subcommand-level completion
// ─────────────────────────────────────────────────────────────────────────────

fn gen_parse(
    enum_name: &syn::Ident,
    metas: &[VariantMeta],
    app_name: &str,
    _about: &str,
    _tagline: &str,
    theme: &str,
    _examples: &[String],
    _hint: &str,
    _url: &str,
    has_global: bool,
    global_type: &TS2,
    static_name: &syn::Ident,
) -> syn::Result<TS2> {
    let mut arms: Vec<TS2> = Vec::new();
    for m in metas {
        let vident = &m.ident;
        let name = &m.cmd_name;
        let aliases = &m.aliases;
        let pattern = if aliases.is_empty() {
            quote! { #name }
        } else {
            quote! { #name #(| #aliases)* }
        };

        let arm = if let Some(ref inner_ty) = m.inner_type {
            quote! {
                #pattern => {
                    if rest.iter().any(|a| *a == "--help" || *a == "-h") {
                        eprintln!();
                        ::cli_ui::print_sub_usage(#app_name, cmd);
                        <#inner_ty as ::cli_ui::PrintHelp>::print_help();
                        ::std::process::exit(0);
                    }
                    let inner = <#inner_ty as ::cli_ui::ParseInner>::parse_inner(&rest)
                        .map_err(|e| {
                            ::cli_ui::print_error(&e.0);
                            eprintln!("  {}  {}",
                                ::cli_ui::styles::paint(::cli_ui::styles::DIM,
                                    ::cli_ui::styles::ARROW),
                                ::cli_ui::styles::paint(::cli_ui::styles::WHITE,
                                    &format!("run `{} {} --help` for usage",
                                        #app_name, cmd)));
                            eprintln!();
                            e
                        })?;
                    Ok(#enum_name::#vident(inner))
                }
            }
        } else {
            quote! {
                #pattern => {
                    if rest.iter().any(|a| *a == "--help" || *a == "-h") {
                        ::cli_ui::print_unit_help(#app_name, cmd, &[]);
                        ::std::process::exit(0);
                    }
                    if let Some(unknown) = rest.iter().find(|a| a.starts_with('-')) {
                        return Err(::cli_ui::CliError(
                            format!("unknown flag: `{unknown}`")
                        ));
                    }
                    Ok(#enum_name::#vident)
                }
            }
        };
        arms.push(arm);
    }

    let all_names: Vec<String> = metas
        .iter()
        .flat_map(|m| std::iter::once(m.cmd_name.clone()).chain(m.aliases.clone()))
        .collect();

    let global_parse = if has_global {
        quote! {
            let (__global, __raw) = ::cli_ui::parse_global_flags::<#global_type>(&__raw);
            #static_name.set(__global).ok();
        }
    } else {
        quote! {}
    };

    // --complete protocol for subcommand-level completion:
    //
    //   app --complete ""              → list all subcommand names
    //   app --complete download ""     → delegate to DownloadOpt's --complete handler
    //   app --complete download --flag word → same
    //
    // The generated script calls these forms depending on cursor position.
    let complete_subcmd_delegate: Vec<TS2> = metas
        .iter()
        .filter_map(|m| {
            let name = &m.cmd_name;
            let aliases = &m.aliases;
            let inner_ty = m.inner_type.as_ref()?;
            let pattern = if aliases.is_empty() {
                quote! { #name }
            } else {
                quote! { #name #(| #aliases)* }
            };
            Some(quote! {
                // app --complete <subcmd> --flag <word>
                // → rebuild argv as [--complete, --flag, word] and let the inner
                //   type's parse() intercept the --complete flag itself.
                #pattern => {
                    // Synthesise a fake argv: ["--complete", rest...]
                    // then re-exec via std::process so the inner parse() path runs.
                    // Since we are in the same binary we use the env::args override
                    // trick: temporarily set args via a thread-local, but that's
                    // complex. Simplest correct approach: call print_completions with
                    // a special tag that carries the remaining tokens.
                    //
                    // Protocol: "complete:<flag> <word>"
                    // The inner CliOptions completions() match arm handles this tag.
                    let __tag = if __complete_rest.is_empty() {
                        "complete: ".to_string()
                    } else {
                        format!("complete:{}", __complete_rest.join(" "))
                    };
                    <#inner_ty as ::cli_ui::NestedCompletions>::print_completions(&__tag);
                }
            })
        })
        .collect();

    Ok(quote! {
        let __raw: Vec<String> = ::std::env::args().skip(1).collect();

        // --help triggers root help only if no subcommand precedes it
        let __help_pos    = __raw.iter().position(|a| a == "--help" || a == "-h");
        let __subcmd_pos  = __raw.iter().position(|a| !a.starts_with('-'));
        let __root_help   = match (__help_pos, __subcmd_pos) {
            (Some(h), Some(s)) => h < s,
            (Some(_), None)    => true,
            _                  => false,
        };
        if __root_help {
            #enum_name::help();
            ::std::process::exit(0);
        }
        if __raw.iter().any(|a| a == "--version" || a == "-V") {
            let (__badge, _) = ::cli_ui::styles::theme_styles(#theme);
            ::cli_ui::print_version(#app_name, env!("CARGO_PKG_VERSION"), __badge);
            ::std::process::exit(0);
        }
        if let Some(__ci) = __raw.iter().position(|a| a == "--completions") {
            let __shell = __raw.get(__ci + 1).map(|s| s.as_str()).unwrap_or("bash");
            #enum_name::completions(__shell);
            ::std::process::exit(0);
        }

        // ── dynamic completion protocol ───────────────────────────────
        // Forms:
        //   app --complete ""              → print subcommand names
        //   app --complete <subcmd> ...    → delegate to subcommand's handler
        if let Some(__ci) = __raw.iter().position(|a| a == "--complete") {
            let __complete_args = &__raw[__ci + 1..];
            // First token after --complete: either empty/flag (root) or subcmd name
            let __first = __complete_args.first().map(|s| s.as_str()).unwrap_or("");

            if __first.is_empty() || __first.starts_with('-') {
                // Root level: suggest subcommand names filtered by typed word
                let __word = __first;
                let __cmds = &[#(#all_names),*];
                for __c in __cmds {
                    if __c.starts_with(__word) { println!("{__c}"); }
                }
            } else {
                // Subcommand level: delegate --complete to the right inner type
                let __subcmd = __first;
                let __complete_rest = &__complete_args[1..];
                match __subcmd {
                    #(#complete_subcmd_delegate)*
                    _ => {}
                }
            }
            ::std::process::exit(0);
        }

        #global_parse

        let __cmd_pos = __raw.iter().position(|a| !a.starts_with('-'));
        let (cmd, rest): (&str, Vec<&str>) = match __cmd_pos {
            Some(i) => (
                __raw[i].as_str(),
                __raw[..i].iter().chain(__raw[i+1..].iter())
                    .map(|s| s.as_str()).collect(),
            ),
            None => {
                #enum_name::help();
                ::cli_ui::print_missing_subcommand(#app_name, &[#(#all_names),*]);
                ::std::process::exit(1);
            }
        };

        let (cmd, rest): (&str, Vec<&str>) = if cmd == "help" {
            let sub = rest.first().copied().unwrap_or("");
            if sub.is_empty() {
                #enum_name::help();
                ::std::process::exit(0);
            }
            (sub, vec!["--help"])
        } else {
            (cmd, rest)
        };

        let result = match cmd {
            #(#arms)*
            other => {
                ::cli_ui::print_unknown_subcommand(#app_name, other,
                    &[#(#all_names),*]);
                ::std::process::exit(1);
            }
        };
        result
    })
}

// ─────────────────────────────────────────────────────────────────────────────
// help() — unchanged
// ─────────────────────────────────────────────────────────────────────────────

fn gen_help(
    metas: &[VariantMeta],
    name: &str,
    about: &str,
    tagline: &str,
    url: &str,
    examples: &[String],
    hint: &str,
    theme: &str,
    has_global: bool,
) -> TS2 {
    let entries: Vec<TS2> = metas
        .iter()
        .map(|m| {
            let cmd_name = &m.cmd_name;
            let about = &m.about;
            let aliases = m
                .aliases
                .iter()
                .map(|a| format!(", {a}"))
                .collect::<String>();
            let label = format!("{cmd_name}{aliases}");
            quote! {
                ::cli_ui::help::HelpEntry::Pair {
                    key:  #label.to_string(),
                    desc: #about.to_string(),
                }
            }
        })
        .collect();

    let global_note = if has_global {
        quote! {
            ::cli_ui::help::HelpEntry::Detail(
                "global flags can be placed before the subcommand name".to_string()
            )
        }
    } else {
        quote! { ::cli_ui::help::HelpEntry::Detail(String::new()) }
    };

    let example_lits: Vec<TS2> = examples.iter().map(|e| quote! { #e }).collect();

    quote! {
        let (__badge, __accent) = ::cli_ui::styles::theme_styles(#theme);
        let __cmds = ::cli_ui::help::HelpSection {
            title:   "Commands",
            entries: vec![ #(#entries),* ],
        };
        let __meta = ::cli_ui::help::HelpSection {
            title: "Meta",
            entries: vec![
                ::cli_ui::help::HelpEntry::Pair { key: "-h, --help".into(),    desc: "Print help".into() },
                ::cli_ui::help::HelpEntry::Pair { key: "-V, --version".into(), desc: "Print version".into() },
                ::cli_ui::help::HelpEntry::Pair { key: "    --completions <SHELL>".into(), desc: "Generate completions (bash, zsh, fish)".into() },
                ::cli_ui::help::HelpEntry::Pair { key: "    help <command>".into(), desc: "Show help for a command".into() },
                #global_note,
            ],
        };
        let __usage = format!("{} <COMMAND> [OPTIONS]", #name);
        ::cli_ui::help::render(
            #name, env!("CARGO_PKG_VERSION"), #about, #tagline, #url,
            &__usage, &[__cmds, __meta],
            &[#(#example_lits),*], #hint, __badge, __accent,
        );
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// completions() — smart scripts with subcommand-aware completion
// ─────────────────────────────────────────────────────────────────────────────

fn gen_completions(metas: &[VariantMeta], app_name: &str) -> TS2 {
    let cmd_names: Vec<String> = metas
        .iter()
        .flat_map(|m| std::iter::once(m.cmd_name.clone()).chain(m.aliases.clone()))
        .collect();
    let cmd_names_str = cmd_names.join(" ");

    // ── bash ─────────────────────────────────────────────────────────
    //
    // Two-level completion:
    //   - Before subcommand: complete subcommand names
    //   - After subcommand: delegate to `app --complete <subcmd> --flag <word>`
    //
    // We detect which subcommand is on the line by scanning COMP_WORDS.

    // Per-subcommand bash case arms: each one delegates to --complete.
    // Aliases share the arm via `name|alias1|alias2)`.
    let bash_subcmd_cases: Vec<TS2> = metas.iter().map(|m| {
        let name    = &m.cmd_name;
        let aliases = &m.aliases;
        let pattern = if aliases.is_empty() {
            name.clone()
        } else {
            std::iter::once(name.clone()).chain(aliases.iter().cloned())
                .collect::<Vec<_>>().join("|")
        };
        if m.inner_type.is_some() {
            quote! {
                script.push_str(&format!(
                    "        {})\n            COMPREPLY=($({} --complete {} \"$cur\" 2>/dev/null))\n            ;;\n",
                    #pattern, #app_name, #name
                ));
            }
        } else {
            quote! {
                script.push_str(&format!(
                    "        {})\n            COMPREPLY=()\n            ;;\n",
                    #pattern
                ));
            }
        }
    }).collect();

    // ── zsh ──────────────────────────────────────────────────────────
    //
    // Uses _arguments with subcommand dispatch via _dispatch or manual state.
    // We emit a complete compdef function that:
    //   1. At word 1: completes subcommand names with descriptions
    //   2. At word 2+: delegates to the inner type's _arguments spec

    let zsh_subcmd_specs: Vec<TS2> = metas
        .iter()
        .map(|m| {
            let name = &m.cmd_name;
            let about = &m.about;
            quote! { format!("{}:{}", #name, #about) }
        })
        .collect();

    // For inner types that implement NestedCompletions, generate a sub-function
    // that calls their completions("zsh") output. The tag "zsh-inner:<parent> <sub>"
    // carries the parent app name and subcommand name so dynamic dispatch lines
    // can call `<parent> --complete <sub> --flag $words[$CURRENT]` correctly.
    let zsh_subcmd_functions: Vec<TS2> = metas
        .iter()
        .filter_map(|m| {
            let name = &m.cmd_name;
            let inner_ty = m.inner_type.as_ref()?;
            let fn_name = format!("_{app_name}_{name}");
            Some(quote! {
                println!("{}() {{", #fn_name);
                <#inner_ty as ::cli_ui::NestedCompletions>::print_completions(
                    &format!("zsh-inner:{} {}", #app_name, #name)
                );
                println!("}}");
            })
        })
        .collect();

    // zsh dispatch lines: `name|alias1|alias2) _app_basename ;;`
    let zsh_dispatch_lines: Vec<TS2> = metas
        .iter()
        .map(|m| {
            let name = &m.cmd_name;
            let aliases = &m.aliases;
            let pattern = if aliases.is_empty() {
                name.clone()
            } else {
                std::iter::once(name.clone())
                    .chain(aliases.iter().cloned())
                    .collect::<Vec<_>>()
                    .join("|")
            };
            quote! {
                dispatch_lines.push_str(&format!(
                    "                {}) _{}_{} ;;\n",
                    #pattern, #app_name, #name,
                ));
            }
        })
        .collect();

    // ── fish ─────────────────────────────────────────────────────────
    //
    // fish has built-in subcommand-awareness via __fish_seen_subcommand_from.
    // We emit:
    //   - Subcommand completion lines (no condition)
    //   - Per-flag lines conditioned on __fish_seen_subcommand_from <subcmd>

    let fish_subcmd_lines: Vec<TS2> = metas
        .iter()
        .map(|m| {
            let name = &m.cmd_name;
            let about = &m.about;
            quote! {
                println!(
                    "complete -c {app} -f -n '__fish_use_subcommand' -a {cmd} -d '{desc}'",
                    app  = #app_name,
                    cmd  = #name,
                    desc = #about,
                );
            }
        })
        .collect();

    // For inner types: emit their fish completions conditioned on subcommand
    let fish_inner_completions: Vec<TS2> = metas
        .iter()
        .filter_map(|m| {
            let name = &m.cmd_name;
            let inner_ty = m.inner_type.as_ref()?;
            Some(quote! {
                // The inner type's fish completions need to be conditioned on
                // having seen this subcommand. We delegate via NestedCompletions
                // with a "fish-subcmd:<name>" tag so the inner generator can
                // prefix each line with the right condition.
                <#inner_ty as ::cli_ui::NestedCompletions>::print_completions(
                    &format!("fish-subcmd:{}", #name)
                );
            })
        })
        .collect();

    // bash case pattern needs | separator: "download|upload"
    let cmd_names_case = cmd_names.join("|");

    quote! {
        match shell {
            // ── bash ─────────────────────────────────────────────────
            "bash" => {
                let mut script = String::new();
                script.push_str(&format!(
r#"_{n}() {{
    local cur prev subcmd opts
    cur="${{COMP_WORDS[COMP_CWORD]}}"
    prev="${{COMP_WORDS[COMP_CWORD-1]}}"
    opts="{cmds} --help --version --completions"

    # Find which subcommand (if any) is already on the line
    subcmd=""
    for word in "${{COMP_WORDS[@]}}"; do
        case "$word" in
            {cmds_case})
                subcmd="$word"
                break
                ;;
        esac
    done

    if [[ -z "$subcmd" ]]; then
        # No subcommand yet — complete subcommand names + global flags
        COMPREPLY=($(compgen -W "$opts" -- "$cur"))
        return
    fi

    # Subcommand known — delegate flag completion to --complete protocol
    case "$subcmd" in
"#,
                    n         = #app_name,
                    cmds      = #cmd_names_str,
                    cmds_case = #cmd_names_case,
                ));
                #(#bash_subcmd_cases)*
                script.push_str(&format!(
r#"        *)
            COMPREPLY=($(compgen -W "--help" -- "$cur"))
            ;;
    esac
}}
complete -F _{n} {n}
"#,
                    n = #app_name
                ));
                print!("{script}");
            }

            // ── zsh ──────────────────────────────────────────────────
            "zsh" => {
                // Emit sub-functions first (inner type specs)
                #(#zsh_subcmd_functions)*

                let subcmd_specs = vec![#(#zsh_subcmd_specs),*].join("\n        ");
                // Build the dispatch table: `name|alias1|alias2) _app_name ;;`
                // Aliases route to the base name's function.
                let mut dispatch_lines = String::new();
                #(#zsh_dispatch_lines)*

                println!(r#"#compdef {n}

_{n}() {{
    local state

    _arguments \
        '--help[Print help]' \
        '--version[Print version]' \
        '--completions[Generate completions]:shell:(bash zsh fish)' \
        ': :_{n}_commands' \
        '*:: :->subcmd'

    case $state in
        subcmd)
            case $words[1] in
{dispatch}            esac
            ;;
    esac
}}

_{n}_commands() {{
    local -a cmds
    cmds=(
        {specs}
    )
    _describe 'command' cmds
}}

compdef _{n} {n}
"#,
                    n        = #app_name,
                    dispatch = dispatch_lines,
                    specs    = subcmd_specs,
                );
            }

            // ── fish ─────────────────────────────────────────────────
            "fish" => {
                // Disable file completion globally for this command
                println!("complete -c {} -f", #app_name);

                // Global flags (always available)
                println!("complete -c {n} -l help    -d 'Print help'",    n = #app_name);
                println!("complete -c {n} -l version -d 'Print version'", n = #app_name);
                println!(
                    "complete -c {n} -l completions -r -a 'bash zsh fish' -d 'Generate completions'",
                    n = #app_name
                );

                // Subcommand names (only when no subcommand seen yet)
                #(#fish_subcmd_lines)*

                // Per-subcommand flags (conditioned on __fish_seen_subcommand_from)
                #(#fish_inner_completions)*
            }

            other => {
                ::cli_ui::print_error(
                    &format!("unknown shell: `{other}`. Supported: bash, zsh, fish"));
                ::std::process::exit(1);
            }
        }
    }
}
