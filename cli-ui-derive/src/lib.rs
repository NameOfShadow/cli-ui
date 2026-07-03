//! Proc-macro implementation for `cli-ui`.
//!
//! This crate is an implementation detail — use `cli-ui` directly.

use proc_macro::TokenStream;
use syn::{parse_macro_input, DeriveInput};

mod attrs;
mod codegen_command;
mod codegen_options;

/// Derive typed argument parsing for a struct.
///
/// See the `cli-ui` crate documentation for full usage and attribute reference.
#[proc_macro_derive(CliOptions, attributes(cli, arg))]
pub fn derive_cli_options(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    codegen_options::impl_cli_options(&input)
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}

/// Derive typed subcommand dispatch for an enum.
///
/// See the `cli-ui` crate documentation for full usage and attribute reference.
#[proc_macro_derive(CliCommand, attributes(cli))]
pub fn derive_cli_command(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    codegen_command::impl_cli_command(&input)
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}
