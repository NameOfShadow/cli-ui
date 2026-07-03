#![allow(missing_docs)]
//! Streaming log output — ports `@clack/prompts` `stream.{info,warn,error,success,step,message}`.
//!
//! Writes a header line, then forwards every line of `iter` through the
//! framed `│  ` prefix.

use crate::styles::{paint, DIM, WHITE};

const BAR: &str = "│";
const S_INFO: &str = "●";
const S_SUCCESS: &str = "◆";
const S_STEP: &str = "◇";
const S_WARN: &str = "▲";
const S_ERROR: &str = "■";

pub fn message<I>(iter: I, symbol: &str)
where
    I: IntoIterator,
    I::Item: AsRef<str>,
{
    eprintln!("{}", paint(DIM, BAR));
    let mut started = false;
    for chunk in iter {
        let chunk = chunk.as_ref();
        for (i, line) in chunk.split('\n').enumerate() {
            if i > 0 || started {
                eprintln!();
                eprint!("{}  ", paint(DIM, BAR));
            } else {
                eprint!("{}  ", symbol);
            }
            eprint!("{}", paint(WHITE, line));
            started = true;
        }
    }
    eprintln!();
}

pub fn info<I>(iter: I)
where
    I: IntoIterator,
    I::Item: AsRef<str>,
{
    message(iter, &paint(super::settings::colors().accent, S_INFO));
}
pub fn success<I>(iter: I)
where
    I: IntoIterator,
    I::Item: AsRef<str>,
{
    message(iter, &paint(super::settings::colors().success, S_SUCCESS));
}
pub fn step<I>(iter: I)
where
    I: IntoIterator,
    I::Item: AsRef<str>,
{
    message(iter, &paint(super::settings::colors().success, S_STEP));
}
pub fn warn<I>(iter: I)
where
    I: IntoIterator,
    I::Item: AsRef<str>,
{
    message(iter, &paint(super::settings::colors().error, S_WARN));
}
pub fn warning<I>(iter: I)
where
    I: IntoIterator,
    I::Item: AsRef<str>,
{
    warn(iter);
}
pub fn error<I>(iter: I)
where
    I: IntoIterator,
    I::Item: AsRef<str>,
{
    message(iter, &paint(super::settings::colors().cancel, S_ERROR));
}
