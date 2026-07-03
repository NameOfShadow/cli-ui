//! Attribute parsing — #[cli(...)] and #[arg(...)] on structs and fields.

use quote::quote;
use syn::{Attribute, Expr, Field, Lit, Type};

// ─────────────────────────────────────────────────────────────────────────────
// Validator kinds
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Clone, Debug)]
pub enum Validator {
    Exists,
    IsFile,
    IsDir,
    Ext(Vec<String>),
    Range {
        min: Option<String>,
        max: Option<String>,
    },
    OneOf(Vec<String>),
    MinLen(usize),
    MaxLen(usize),
    Glob(String),
    Custom(String), // stored as token string, parsed in codegen
    WarnIf(Box<Validator>),
    /// validate(env("VAR", fallback)) — read from env, validate result
    Env {
        var: String,
        fallback: Option<String>,
    },
}

#[derive(Clone, Debug)]
pub enum Action {
    CreateDir,
    CreateDirAll,
}

// ─────────────────────────────────────────────────────────────────────────────
// Top-level #[cli(...)] on structs / enums
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Default, Clone)]
pub struct CliAttrs {
    pub name: Option<String>,
    pub about: Option<String>,
    pub tagline: Option<String>,
    pub url: Option<String>,
    pub examples: Vec<String>,
    pub hint: Option<String>,
    pub theme: Option<String>,
    pub global: Option<syn::Path>,
}

impl CliAttrs {
    pub fn parse(attrs: &[Attribute]) -> Self {
        let mut out = Self::default();
        for attr in attrs {
            if !attr.path().is_ident("cli") {
                continue;
            }
            let _ = attr.parse_nested_meta(|m| {
                let key = ident_str(&m.path);
                match key.as_str() {
                    "name" => out.name = Some(lit_str(&m.value()?.parse::<Lit>()?)),
                    "about" => out.about = Some(lit_str(&m.value()?.parse::<Lit>()?)),
                    "tagline" => out.tagline = Some(lit_str(&m.value()?.parse::<Lit>()?)),
                    "url" => out.url = Some(lit_str(&m.value()?.parse::<Lit>()?)),
                    "example" => out.examples.push(lit_str(&m.value()?.parse::<Lit>()?)),
                    "hint" => out.hint = Some(lit_str(&m.value()?.parse::<Lit>()?)),
                    "theme" => out.theme = Some(lit_str(&m.value()?.parse::<Lit>()?)),
                    "global" => out.global = Some(m.value()?.parse::<syn::Path>()?),
                    _ => {}
                }
                Ok(())
            });
        }
        out
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Per-field #[arg(...)]
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Default, Clone)]
pub struct ArgAttrs {
    pub positional: bool,
    pub section: Option<String>,
    pub short: Option<char>,
    pub long: Option<String>,
    pub default_val: Option<DefaultVal>,
    pub negatable: bool,
    pub multi: bool,
    pub skip: bool,
    pub group: Option<String>,
    pub conflicts_with: Vec<String>,
    pub requires: Vec<String>,
    pub requires_any: Vec<String>,
    pub required_unless: Vec<String>,
    pub required_unless_any: Vec<String>,
    pub validators: Vec<Validator>,
    pub actions: Vec<Action>,
    /// Dynamic completions: function name returning Vec<String>
    pub complete: Option<String>, // stored as token string
    pub doc: String,
}

#[derive(Clone, Debug)]
pub enum DefaultVal {
    Lit(String),
    Env {
        var: String,
        fallback: Option<String>,
    },
    Fn(String), // stored as token string, parsed in codegen
}

impl ArgAttrs {
    pub fn parse(field: &Field) -> Self {
        let mut out = Self::default();

        // collect doc comments
        out.doc = field
            .attrs
            .iter()
            .filter(|a| a.path().is_ident("doc"))
            .filter_map(|a| {
                if let syn::Meta::NameValue(nv) = &a.meta {
                    if let Expr::Lit(el) = &nv.value {
                        if let Lit::Str(s) = &el.lit {
                            return Some(s.value().trim().to_string());
                        }
                    }
                }
                None
            })
            .collect::<Vec<_>>()
            .join(" ");

        for attr in &field.attrs {
            if !attr.path().is_ident("arg") {
                continue;
            }
            let _ = attr.parse_nested_meta(|m| {
                let key = ident_str(&m.path);
                match key.as_str() {
                    "positional" => {
                        out.positional = true;
                    }
                    "negatable" => {
                        out.negatable = true;
                    }
                    "multi" => {
                        out.multi = true;
                    }
                    "skip" => {
                        out.skip = true;
                    }
                    "section" => {
                        out.section = Some(lit_str(&m.value()?.parse::<Lit>()?));
                    }
                    "long" => {
                        out.long = Some(lit_str(&m.value()?.parse::<Lit>()?));
                    }
                    "group" => {
                        out.group = Some(lit_str(&m.value()?.parse::<Lit>()?));
                    }
                    "short" => {
                        if let Lit::Char(c) = m.value()?.parse::<Lit>()? {
                            out.short = Some(c.value());
                        }
                    }
                    "default" => {
                        out.default_val = Some(parse_default_val(&m)?);
                    }
                    "conflicts_with" => {
                        out.conflicts_with = parse_str_list(&m)?;
                    }
                    "requires" => {
                        out.requires = parse_str_list(&m)?;
                    }
                    "requires_any" => {
                        out.requires_any = parse_str_list(&m)?;
                    }
                    "required_unless" => {
                        out.required_unless = parse_str_list(&m)?;
                    }
                    "required_unless_any" => {
                        out.required_unless_any = parse_str_list(&m)?;
                    }
                    "validate" => {
                        // parse nested: validate(exists, range(1,10), ...)
                        m.parse_nested_meta(|vm| {
                            if let Some(v) = parse_validator(&vm)? {
                                out.validators.push(v);
                            }
                            Ok(())
                        })?;
                    }
                    "action" => {
                        m.parse_nested_meta(|am| {
                            let k = ident_str(&am.path);
                            match k.as_str() {
                                "create_dir" => out.actions.push(Action::CreateDir),
                                "create_dir_all" => out.actions.push(Action::CreateDirAll),
                                other => return Err(am.error(format!("unknown action: `{other}`"))),
                            }
                            Ok(())
                        })?;
                    }
                    "complete" => {
                        let p: syn::Path = m.value()?.parse()?;
                        out.complete = Some(quote::quote!(#p).to_string());
                    }
                    _ => {}
                }
                Ok(())
            });
        }
        out
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Validator parser
// ─────────────────────────────────────────────────────────────────────────────

fn parse_validator(m: &syn::meta::ParseNestedMeta) -> syn::Result<Option<Validator>> {
    let key = ident_str(&m.path);
    let v = match key.as_str() {
        "exists" => Validator::Exists,
        "is_file" => Validator::IsFile,
        "is_dir" => Validator::IsDir,
        "ext" => {
            let exts = parse_paren_str_list(m)?;
            if exts.is_empty() {
                return Err(m.error("ext(): got empty list — check syntax: ext(\"csv\", \"json\")"));
            }
            Validator::Ext(exts)
        }
        "range" => {
            // range(min, max) | range(min) | range(..=max)
            let (min, max) = parse_range(m)?;
            Validator::Range { min, max }
        }
        "one_of" => {
            let vals = parse_paren_str_list(m)?;
            Validator::OneOf(vals)
        }
        "min_len" => {
            let n = parse_paren_usize(m)?;
            Validator::MinLen(n)
        }
        "max_len" => {
            let n = parse_paren_usize(m)?;
            Validator::MaxLen(n)
        }
        "glob" => {
            let s = parse_paren_str(m)?;
            Validator::Glob(s)
        }
        "custom" => {
            // custom = fn_name  — parse the path after =
            let path: syn::Path = m.value()?.parse()?;
            Validator::Custom(quote::quote!(#path).to_string())
        }
        "warn_if" => {
            // warn_if(exists) — parenthesized ident, not = value
            let content;
            syn::parenthesized!(content in m.input);
            let inner_key: syn::Ident = content.parse()?;
            let inner = match inner_key.to_string().as_str() {
                "exists" => Validator::Exists,
                "is_file" => Validator::IsFile,
                "is_dir" => Validator::IsDir,
                other => {
                    return Err(syn::Error::new(
                        inner_key.span(),
                        format!("unknown warn_if validator: `{other}`"),
                    ))
                }
            };
            Validator::WarnIf(Box::new(inner))
        }
        "env" => {
            // validate(env("VAR", fallback))
            let (var, fallback) = parse_env_args(m)?;
            Validator::Env { var, fallback }
        }
        other => {
            return Err(m.error(format!("unknown validator: `{other}`")));
        }
    };
    Ok(Some(v))
}

// ─────────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────────

pub fn ident_str(path: &syn::Path) -> String {
    path.get_ident().map(|i| i.to_string()).unwrap_or_default()
}

pub fn lit_str(lit: &Lit) -> String {
    match lit {
        Lit::Str(s) => s.value(),
        Lit::Bool(b) => b.value.to_string(),
        Lit::Int(i) => i.base10_digits().to_string(),
        other => quote!(#other).to_string(),
    }
}

pub fn expr_to_string(expr: &Expr) -> String {
    match expr {
        Expr::Lit(el) => lit_str(&el.lit),
        other => quote!(#other).to_string(),
    }
}

pub fn is_option(ty: &Type) -> bool {
    if let Type::Path(tp) = ty {
        tp.path
            .segments
            .first()
            .map(|s| s.ident == "Option")
            .unwrap_or(false)
    } else {
        false
    }
}

pub fn is_vec(ty: &Type) -> bool {
    if let Type::Path(tp) = ty {
        tp.path
            .segments
            .first()
            .map(|s| s.ident == "Vec")
            .unwrap_or(false)
    } else {
        false
    }
}

pub fn unwrap_generic(ty: &Type) -> &Type {
    if let Type::Path(tp) = ty {
        if let Some(seg) = tp.path.segments.first() {
            if let syn::PathArguments::AngleBracketed(ab) = &seg.arguments {
                if let Some(syn::GenericArgument::Type(t)) = ab.args.first() {
                    return t;
                }
            }
        }
    }
    ty
}

pub fn type_hint(ty: &Type) -> String {
    let s = quote!(#ty).to_string().replace(' ', "");
    let s = if (s.starts_with("Option<") || s.starts_with("Vec<")) && s.ends_with('>') {
        s[s.find('<').unwrap() + 1..s.len() - 1].to_string()
    } else {
        s
    };
    match s.as_str() {
        "PathBuf" | "std::path::PathBuf" => "PATH".to_string(),
        "String" | "std::string::String" => "STR".to_string(),
        "usize" | "u32" | "u64" | "u16" | "u8" | "i32" | "i64" | "i16" => "N".to_string(),
        "bool" => String::new(),
        "f32" | "f64" => "F".to_string(),
        other => other.to_string(),
    }
}

pub fn collect_doc(attrs: &[Attribute]) -> String {
    attrs
        .iter()
        .filter(|a| a.path().is_ident("doc"))
        .filter_map(|a| {
            if let syn::Meta::NameValue(nv) = &a.meta {
                if let Expr::Lit(el) = &nv.value {
                    if let Lit::Str(s) = &el.lit {
                        return Some(s.value().trim().to_string());
                    }
                }
            }
            None
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn parse_str_list(m: &syn::meta::ParseNestedMeta) -> syn::Result<Vec<String>> {
    parse_paren_str_list(m)
}

fn parse_paren_str_list(m: &syn::meta::ParseNestedMeta) -> syn::Result<Vec<String>> {
    let mut out = Vec::new();
    if m.input.peek(syn::token::Paren) {
        let content;
        syn::parenthesized!(content in m.input);
        while !content.is_empty() {
            let s: syn::LitStr = content.parse()?;
            out.push(s.value());
            if content.peek(syn::Token![,]) {
                let _: syn::Token![,] = content.parse()?;
            }
        }
    } else {
        // single value: requires = "other"
        let lit: Lit = m.value()?.parse()?;
        out.push(lit_str(&lit));
    }
    Ok(out)
}

fn parse_paren_str(m: &syn::meta::ParseNestedMeta) -> syn::Result<String> {
    let content;
    syn::parenthesized!(content in m.input);
    let s: syn::LitStr = content.parse()?;
    Ok(s.value())
}

fn parse_paren_usize(m: &syn::meta::ParseNestedMeta) -> syn::Result<usize> {
    let content;
    syn::parenthesized!(content in m.input);
    let n: syn::LitInt = content.parse()?;
    Ok(n.base10_parse::<usize>()?)
}

fn parse_range(m: &syn::meta::ParseNestedMeta) -> syn::Result<(Option<String>, Option<String>)> {
    let content;
    syn::parenthesized!(content in m.input);

    // try ..=max syntax first
    if content.peek(syn::Token![..]) {
        let _: syn::Token![..] = content.parse()?;
        if content.peek(syn::Token![=]) {
            let _: syn::Token![=] = content.parse()?;
            let max: syn::LitInt = content.parse()?;
            return Ok((None, Some(max.base10_digits().to_string())));
        }
        return Ok((None, None));
    }

    let min: syn::LitInt = content.parse()?;
    if content.is_empty() {
        return Ok((Some(min.base10_digits().to_string()), None));
    }
    let _: syn::Token![,] = content.parse()?;
    let max: syn::LitInt = content.parse()?;
    Ok((
        Some(min.base10_digits().to_string()),
        Some(max.base10_digits().to_string()),
    ))
}

fn parse_env_args(m: &syn::meta::ParseNestedMeta) -> syn::Result<(String, Option<String>)> {
    let content;
    syn::parenthesized!(content in m.input);
    let var: syn::LitStr = content.parse()?;
    let fallback = if content.peek(syn::Token![,]) {
        let _: syn::Token![,] = content.parse()?;
        let fb: Lit = content.parse()?;
        Some(lit_str(&fb))
    } else {
        None
    };
    Ok((var.value(), fallback))
}

fn parse_default_val(m: &syn::meta::ParseNestedMeta) -> syn::Result<DefaultVal> {
    // default = env("VAR", fallback)
    // default = fn_name()
    // default = "literal" / 42 / true
    let expr: Expr = m.value()?.parse()?;
    match &expr {
        Expr::Call(call) => {
            if let Expr::Path(p) = &*call.func {
                let name = p
                    .path
                    .get_ident()
                    .map(|i| i.to_string())
                    .unwrap_or_default();
                if name == "env" {
                    // env("VAR") or env("VAR", fallback)
                    let mut args = call.args.iter();
                    let var = match args.next() {
                        Some(Expr::Lit(el)) => lit_str(&el.lit),
                        _ => {
                            return Err(syn::Error::new_spanned(
                                &expr,
                                "env() expects a string literal",
                            ))
                        }
                    };
                    let fallback = args.next().map(|a| expr_to_string(a));
                    return Ok(DefaultVal::Env { var, fallback });
                }
            }
            // function call: calc_workers()
            Ok(DefaultVal::Fn(match &*call.func {
                Expr::Path(p) => quote::quote!(#p).to_string(),
                _ => return Err(syn::Error::new_spanned(&expr, "expected function path")),
            }))
        }
        _ => Ok(DefaultVal::Lit(expr_to_string(&expr))),
    }
}
