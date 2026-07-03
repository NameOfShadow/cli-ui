//! Example: composable validators + clean Ctrl+C handling.
//!
//! ```bash
//! cargo run --example prompt_validate
//! ```

use cli_ui::prompt::prelude::*;

fn main() {
    let _ = colors(); // silence unused-import warning if you remove theming below

    intro("Validation playground");

    let abort = "Aborted — nothing was saved.";

    // 12-word seed phrase, composed from primitives.
    let lowercase_words = Validator::new(|v: &str| {
        if v.split_whitespace()
            .all(|w| w.chars().all(|c| c.is_ascii_lowercase()))
        {
            Ok(())
        } else {
            Err("Words must be lowercase ASCII".into())
        }
    });
    let _seed = text("Recovery phrase")
        .rule(word_count(12).and(lowercase_words))
        .run()
        .or_cancel(abort);

    let _pw = secret("Set a password")
        .rule(
            min_chars(12)
                .and(has_upper())
                .and(has_lower())
                .and(has_digit())
                .and(has_special())
                .msg("≥12 chars with upper, lower, digit, and a special char"),
        )
        .run()
        .or_cancel(abort);

    let _mail = text("Email")
        .placeholder("you@example.com")
        .rule(email())
        .run()
        .or_cancel(abort);

    let _port = text("Listen port")
        .default("8080")
        .rule(int_between(1024, 65535))
        .run()
        .or_cancel(abort);

    let _user = text("Username")
        .rule(
            min_chars(3)
                .and(max_chars(20))
                .and(alphanumeric())
                .msg("3–20 alphanumeric characters"),
        )
        .run()
        .or_cancel(abort);

    let _name = text("Display name")
        .validate(|s: &str| {
            if s.contains(char::is_whitespace) {
                Err("No spaces allowed".into())
            } else {
                Ok(())
            }
        })
        .run()
        .or_cancel(abort);

    outro("All validators passed!");
}
