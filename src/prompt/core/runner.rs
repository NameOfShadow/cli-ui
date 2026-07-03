//! The generic prompt runner — owns raw mode, the key loop, line
//! bookkeeping, the validate → success transition, the answered redraw,
//! and Ctrl-C cleanup.
//!
//! Every prompt's `run()` method ultimately calls [`run`] with `self`.

use super::super::error::{PromptError, Result};
use super::{Frame, Prompt, RenderCtx, Step};

/// Run a prompt to completion.
///
/// Picks the interactive engine when stderr & stdin are TTYs; otherwise
/// delegates to [`Prompt::run_fallback`] for piped / CI environments.
pub fn run<P: Prompt>(p: P) -> Result<P::Output> {
    if super::super::engine::is_interactive() {
        run_interactive(p)
    } else {
        p.run_fallback()
    }
}

fn run_interactive<P: Prompt>(mut p: P) -> Result<P::Output> {
    use super::super::engine::interactive::{enter_raw, leave_raw, read_key};
    use super::super::render::{cleanup, clear_lines, hide_cursor, show_cursor};

    let mut out = std::io::stderr();
    let mut error: Option<String> = None;

    enter_raw()?;
    hide_cursor(&mut out).ok();

    let mut frame = p.render(RenderCtx {
        error: error.as_deref(),
    });
    write_frame(&mut out, &frame);

    let outcome: Result<P::Output> = loop {
        let key = match read_key() {
            Ok(k) => k,
            Err(_) => {
                cleanup(&mut out, frame.len());
                return Err(PromptError::Interrupted);
            }
        };
        match p.handle(key) {
            Step::Continue => {
                error = None;
            }
            Step::Reject(msg) => {
                error = Some(msg);
            }
            Step::Submit(value) => break Ok(value),
            Step::Cancel => {
                cleanup(&mut out, frame.len());
                return Err(PromptError::Interrupted);
            }
        }
        clear_lines(&mut out, frame.len()).ok();
        frame = p.render(RenderCtx {
            error: error.as_deref(),
        });
        write_frame(&mut out, &frame);
    };

    // If the *previous* frame happened to be an error frame but the user
    // immediately produced a valid Submit, repaint a clean frame so the
    // line counts the answered renderer is about to clear match reality.
    if error.is_some() {
        clear_lines(&mut out, frame.len()).ok();
        frame = p.render(RenderCtx { error: None });
        write_frame(&mut out, &frame);
    }

    show_cursor(&mut out).ok();
    leave_raw()?;

    let value = outcome?;
    clear_lines(&mut out, frame.len()).ok();
    write_answered(&p.render_answered(&value));
    Ok(value)
}

fn write_frame(out: &mut std::io::Stderr, frame: &Frame) {
    use std::io::Write;
    for line in frame {
        let _ = out.write_all(line.as_bytes());
        let _ = out.write_all(b"\r\n");
    }
    let _ = out.flush();
}

fn write_answered(frame: &Frame) {
    // Called after `leave_raw()` so plain `\n` line endings are correct.
    for line in frame {
        eprintln!("{line}");
    }
}
