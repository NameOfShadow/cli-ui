//! Composable input validators — ports `@clack/core`'s `runValidation` with a
//! library-of-rules on top, and ports `utils/string.ts`-style helpers.
//!
//! A validator returns `Ok(())` if the value is acceptable, or `Err(msg)` with
//! the error string to show below the input. Validators compose with `&` (AND)
//! and `|` (OR) and can be tagged with custom messages via [`Validator::msg`].
//!
//! # Example — composing rules in user code
//! ```no_run
//! use cli_ui::prompt::{secret, text, min_chars, has_upper, has_lower,
//!     has_digit, has_special, word_count};
//!
//! let password = secret("New password")
//!     .rule(
//!         min_chars(12)
//!             .and(has_upper())
//!             .and(has_lower())
//!             .and(has_digit())
//!             .and(has_special())
//!             .msg("≥12 chars with upper/lower/digit/special required"),
//!     )
//!     .run()?;
//!
//! let seed = text("Recovery phrase")
//!     .rule(word_count(12))
//!     .run()?;
//! # Ok::<(), cli_ui::prompt::PromptError>(())
//! ```
//!
//! # Integrating an external validation crate
//!
//! Implement the [`Validate`] trait for any wrapper around your library of
//! choice (`validator`, `garde`, etc.):
//!
//! ```no_run
//! use cli_ui::prompt::validate::Validator;
//!
//! fn from_external(rule: impl Fn(&str) -> Result<(), String> + Send + Sync + 'static) -> Validator {
//!     Validator::new(rule)
//! }
//! ```

use std::sync::Arc;

/// Anything that can decide whether an input string is acceptable.
///
/// Implement this for any external validation library to bridge it into
/// the prompt's `.rule(...)` chain.
pub trait Validate: Send + Sync + 'static {
    /// Return `Ok(())` to accept the value, `Err(msg)` to reject and show
    /// `msg` as the user-facing error.
    fn check(&self, value: &str) -> Result<(), String>;
}

impl<F> Validate for F
where
    F: Fn(&str) -> Result<(), String> + Send + Sync + 'static,
{
    fn check(&self, value: &str) -> Result<(), String> {
        self(value)
    }
}

/// Validator handle returned by the helpers in this module. Composes with
/// [`Validator::and`] / [`Validator::or`] / [`Validator::msg`].
#[derive(Clone)]
pub struct Validator(Arc<dyn Validate>);

impl Validator {
    /// Wrap any [`Validate`] implementation — including a plain closure
    /// `|s: &str| -> Result<(), String>` — into a [`Validator`].
    pub fn new<V: Validate>(v: V) -> Self {
        Self(Arc::new(v))
    }

    /// Run the validator. Mirrors [`Validate::check`].
    pub fn check(&self, v: &str) -> Result<(), String> {
        self.0.check(v)
    }

    /// Both must pass. Returns the first error encountered.
    pub fn and(self, other: Validator) -> Validator {
        let a = self.0;
        let b = other.0;
        Validator::new(move |v: &str| {
            a.check(v)?;
            b.check(v)
        })
    }

    /// At least one must pass. Returns the *second* error if both fail
    /// (usually more user-friendly than "you failed multiple things").
    pub fn or(self, other: Validator) -> Validator {
        let a = self.0;
        let b = other.0;
        Validator::new(move |v: &str| {
            if a.check(v).is_ok() {
                return Ok(());
            }
            b.check(v)
        })
    }

    /// Replace the error message produced by this validator.
    pub fn msg(self, message: impl Into<String>) -> Validator {
        let inner = self.0;
        let msg = message.into();
        Validator::new(move |v: &str| inner.check(v).map_err(|_| msg.clone()))
    }
}

impl Validate for Validator {
    fn check(&self, v: &str) -> Result<(), String> {
        self.0.check(v)
    }
}

impl<F> From<F> for Validator
where
    F: Fn(&str) -> Result<(), String> + Send + Sync + 'static,
{
    fn from(f: F) -> Self {
        Validator::new(f)
    }
}

// ── Rule builders ────────────────────────────────────────────────────────────

/// Reject empty input.
pub fn required() -> Validator {
    Validator::new(|v: &str| {
        if v.trim().is_empty() {
            Err("Value is required".into())
        } else {
            Ok(())
        }
    })
}

/// At least `n` characters (counted by Unicode chars).
pub fn min_chars(n: usize) -> Validator {
    Validator::new(move |v: &str| {
        if v.chars().count() < n {
            Err(format!("Must be at least {n} characters"))
        } else {
            Ok(())
        }
    })
}

/// At most `n` characters.
pub fn max_chars(n: usize) -> Validator {
    Validator::new(move |v: &str| {
        if v.chars().count() > n {
            Err(format!("Must be at most {n} characters"))
        } else {
            Ok(())
        }
    })
}

/// Exact length in characters.
pub fn exact_chars(n: usize) -> Validator {
    Validator::new(move |v: &str| {
        let c = v.chars().count();
        if c != n {
            Err(format!("Must be exactly {n} characters (got {c})"))
        } else {
            Ok(())
        }
    })
}

/// Exactly `n` whitespace-separated words.
pub fn word_count(n: usize) -> Validator {
    Validator::new(move |v: &str| {
        let c = v.split_whitespace().count();
        if c != n {
            Err(format!("Must be exactly {n} words (got {c})"))
        } else {
            Ok(())
        }
    })
}

/// At least one of `min..=max` words.
pub fn words_between(min: usize, max: usize) -> Validator {
    Validator::new(move |v: &str| {
        let c = v.split_whitespace().count();
        if c < min || c > max {
            Err(format!("Must be {min}–{max} words (got {c})"))
        } else {
            Ok(())
        }
    })
}

/// At least one ASCII uppercase character.
pub fn has_upper() -> Validator {
    Validator::new(|v: &str| {
        if v.chars().any(|c| c.is_ascii_uppercase()) {
            Ok(())
        } else {
            Err("Must contain an uppercase letter".into())
        }
    })
}

/// At least one ASCII lowercase character.
pub fn has_lower() -> Validator {
    Validator::new(|v: &str| {
        if v.chars().any(|c| c.is_ascii_lowercase()) {
            Ok(())
        } else {
            Err("Must contain a lowercase letter".into())
        }
    })
}

/// At least one ASCII digit.
pub fn has_digit() -> Validator {
    Validator::new(|v: &str| {
        if v.chars().any(|c| c.is_ascii_digit()) {
            Ok(())
        } else {
            Err("Must contain a digit".into())
        }
    })
}

/// At least one ASCII punctuation/special character.
pub fn has_special() -> Validator {
    Validator::new(|v: &str| {
        if v.chars().any(|c| c.is_ascii_punctuation()) {
            Ok(())
        } else {
            Err("Must contain a special character".into())
        }
    })
}

/// Letters only (Unicode alphabetic).
pub fn alpha_only() -> Validator {
    Validator::new(|v: &str| {
        if v.chars().all(|c| c.is_alphabetic()) {
            Ok(())
        } else {
            Err("Letters only".into())
        }
    })
}

/// Letters and digits only.
pub fn alphanumeric() -> Validator {
    Validator::new(|v: &str| {
        if v.chars().all(|c| c.is_alphanumeric()) {
            Ok(())
        } else {
            Err("Letters and digits only".into())
        }
    })
}

/// Restrict to a given set of characters.
pub fn only_chars(allowed: &'static str) -> Validator {
    Validator::new(move |v: &str| {
        if v.chars().all(|c| allowed.contains(c)) {
            Ok(())
        } else {
            Err(format!("Allowed characters: {allowed}"))
        }
    })
}

/// Reject if any of the given characters appear.
pub fn forbid_chars(forbidden: &'static str) -> Validator {
    Validator::new(move |v: &str| {
        if let Some(c) = v.chars().find(|c| forbidden.contains(*c)) {
            Err(format!("'{c}' is not allowed"))
        } else {
            Ok(())
        }
    })
}

/// Parsable as integer in `[min, max]`.
pub fn int_between(min: i64, max: i64) -> Validator {
    Validator::new(move |v: &str| match v.trim().parse::<i64>() {
        Ok(n) if (min..=max).contains(&n) => Ok(()),
        Ok(n) => Err(format!("Must be between {min} and {max} (got {n})")),
        Err(_) => Err("Must be an integer".into()),
    })
}

/// Parsable as float in `[min, max]`.
pub fn float_between(min: f64, max: f64) -> Validator {
    Validator::new(move |v: &str| match v.trim().parse::<f64>() {
        Ok(n) if n >= min && n <= max => Ok(()),
        Ok(n) => Err(format!("Must be between {min} and {max} (got {n})")),
        Err(_) => Err("Must be a number".into()),
    })
}

/// Looks like an email (very permissive: `something@something.something`).
pub fn email() -> Validator {
    Validator::new(|v: &str| {
        let s = v.trim();
        let parts: Vec<&str> = s.splitn(2, '@').collect();
        if parts.len() == 2
            && !parts[0].is_empty()
            && parts[1].contains('.')
            && !parts[1].starts_with('.')
            && !parts[1].ends_with('.')
        {
            Ok(())
        } else {
            Err("Must be a valid email".into())
        }
    })
}

/// Starts with `prefix`.
pub fn starts_with(prefix: &'static str) -> Validator {
    Validator::new(move |v: &str| {
        if v.starts_with(prefix) {
            Ok(())
        } else {
            Err(format!("Must start with `{prefix}`"))
        }
    })
}

/// Ends with `suffix`.
pub fn ends_with(suffix: &'static str) -> Validator {
    Validator::new(move |v: &str| {
        if v.ends_with(suffix) {
            Ok(())
        } else {
            Err(format!("Must end with `{suffix}`"))
        }
    })
}

/// One of the listed values, case-sensitive.
pub fn one_of(values: &'static [&'static str]) -> Validator {
    Validator::new(move |v: &str| {
        if values.contains(&v) {
            Ok(())
        } else {
            Err(format!("Must be one of: {}", values.join(", ")))
        }
    })
}

// ── End of primitives ────────────────────────────────────────────────────────
//
// Domain-specific bundles (BIP-39 seed phrases, password-strength policies,
// US zip codes, ICAO airport codes, …) belong in *user* code or downstream
// crates — composing the primitives above with `.and()` / `.or()` / `.msg()`
// keeps this module general-purpose.
