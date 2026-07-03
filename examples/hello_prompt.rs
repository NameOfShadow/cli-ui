//! The smallest possible prompt example.
//!
//! ```bash
//! cargo run --example hello_prompt
//! ```
//!
//! Read this top-to-bottom and you've seen the whole API surface you need
//! for ~80% of CLIs.

use cli_ui::prompt::prelude::*;

fn main() {
    intro("Set up a new project");

    let name = text("What's your name?")
        .default("Anya") // accepted on Enter when buffer is empty
        .placeholder("e.g. Anya") // dim hint shown before typing
        .run()
        .or_cancel("Cancelled.");

    let stack = select("Pick a stack")
        .option("rust", "Rust")
        .option("ts", "TypeScript")
        .option("py", "Python")
        .run()
        .or_cancel("Cancelled.");

    let init_git = confirm("Initialise git?")
        .default(true)
        .run()
        .or_cancel("Cancelled.");

    outro(format!(
        "{name} → {} ({} git)",
        stack.label,
        if init_git { "with" } else { "without" }
    ));
}
