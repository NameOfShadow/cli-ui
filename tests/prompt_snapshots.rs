//! Snapshot tests for prompt visuals.
//!
//! Each test exercises a prompt's `Prompt` trait directly — no terminal
//! required. It drives the state machine with a scripted key sequence,
//! captures the rendered frame as a stripped (ANSI-removed) `Vec<String>`,
//! and asserts that frame against an inline expected snapshot.
//!
//! Why this lives in `tests/` rather than `#[cfg(test)]`: the snapshots
//! cover the *public* prompt contract (key handling + frame layout). If
//! refactors break either the snapshot fails — it's the safety net the
//! prior architecture had no place to put.

use cli_ui::prompt::core::{Key, Prompt, RenderCtx, Step};
use cli_ui::prompt::{confirm, select, text};

/// Strip ANSI escape sequences so snapshots are stable across themes.
fn strip(line: &str) -> String {
    let mut out = String::with_capacity(line.len());
    let mut chars = line.chars();
    while let Some(c) = chars.next() {
        if c == '\x1b' {
            // CSI: skip until a letter that terminates the sequence
            if matches!(chars.next(), Some('[')) {
                for c in chars.by_ref() {
                    if c.is_ascii_alphabetic() {
                        break;
                    }
                }
            }
        } else {
            out.push(c);
        }
    }
    out
}

fn render_frame<P: Prompt>(p: &P, error: Option<&str>) -> Vec<String> {
    p.render(RenderCtx { error })
        .iter()
        .map(|l| strip(l))
        .collect()
}

fn render_answered<P: Prompt>(p: &P, v: &P::Output) -> Vec<String> {
    p.render_answered(v).iter().map(|l| strip(l)).collect()
}

// ─────────────────────────────────────────────────────────────────────────────
// text
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn text_initial_frame_renders_label_and_placeholder() {
    let p = text("Your name").placeholder("Anya");
    let f = render_frame(&p, None);
    assert_eq!(f.len(), 3);
    assert!(f[0].contains("◆") && f[0].contains("Your name"));
    assert!(f[1].contains("Anya"));
    assert!(f[2].contains("└"));
}

#[test]
fn text_handles_typing_and_submits() {
    let mut p = text("Your name");
    for c in "Alice".chars() {
        assert!(matches!(p.handle(Key::Char(c)), Step::Continue));
    }
    match p.handle(Key::Enter) {
        Step::Submit(s) => assert_eq!(s, "Alice"),
        other => panic!("expected Submit, got {other:?}"),
    }
}

#[test]
fn text_validation_rejects_then_accepts() {
    use cli_ui::prompt::min_chars;
    let mut p = text("Your name").rule(min_chars(3));
    for c in "Al".chars() {
        let _ = p.handle(Key::Char(c));
    }
    match p.handle(Key::Enter) {
        Step::Reject(_) => {}
        other => panic!("expected Reject, got {other:?}"),
    }
    let _ = p.handle(Key::Char('i'));
    match p.handle(Key::Enter) {
        Step::Submit(s) => assert_eq!(s, "Ali"),
        other => panic!("expected Submit after fix, got {other:?}"),
    }
}

#[test]
fn text_error_frame_switches_to_triangle() {
    let p = text("Your name");
    let f = render_frame(&p, Some("Bad"));
    assert!(
        f[0].contains("▲"),
        "expected ▲ in error state, got {:?}",
        f[0]
    );
    let last = f.last().unwrap();
    assert!(
        last.contains("Bad"),
        "expected error msg on last line, got {last:?}"
    );
}

#[test]
fn text_answered_frame_keeps_question_and_value() {
    let p = text("Your name");
    let f = render_answered(&p, &"Alice".to_string());
    assert_eq!(
        f.len(),
        3,
        "answered frame should be label + value + connector"
    );
    assert!(f[0].contains("◇") && f[0].contains("Your name"));
    assert!(f[1].contains("Alice"));
}

#[test]
fn text_escape_cancels() {
    let mut p = text("Your name");
    assert!(matches!(p.handle(Key::Escape), Step::Cancel));
}

#[test]
fn text_left_right_home_end_move_cursor() {
    let mut p = text("L");
    for c in "abc".chars() {
        let _ = p.handle(Key::Char(c));
    }
    let _ = p.handle(Key::Home);
    let _ = p.handle(Key::Char('Z'));
    match p.handle(Key::Enter) {
        Step::Submit(s) => assert_eq!(s, "Zabc"),
        other => panic!("got {other:?}"),
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// confirm
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn confirm_default_yes_and_left_toggles() {
    let mut p = confirm("Continue?").default(true);
    let f = render_frame(&p, None);
    assert!(f[1].contains("Yes"));
    let _ = p.handle(Key::Left);
    match p.handle(Key::Enter) {
        Step::Submit(v) => assert!(!v),
        other => panic!("got {other:?}"),
    }
}

#[test]
fn confirm_y_n_shortcut() {
    let mut p = confirm("Continue?");
    match p.handle(Key::Char('n')) {
        Step::Submit(v) => assert!(!v),
        other => panic!("got {other:?}"),
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// select
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn select_cursor_navigates_and_submits() {
    let mut p = select("Pick one")
        .option("a", "Alpha")
        .option("b", "Beta")
        .option("c", "Gamma");
    let _ = p.handle(Key::Down);
    let _ = p.handle(Key::Down);
    match p.handle(Key::Enter) {
        Step::Submit(s) => {
            assert_eq!(s.value, "c");
            assert_eq!(s.label, "Gamma");
            assert_eq!(s.index, 2);
        }
        other => panic!("got {other:?}"),
    }
}

#[test]
fn select_does_not_wrap_past_ends() {
    let mut p = select("Pick one").option("a", "Alpha").option("b", "Beta");
    let _ = p.handle(Key::Up); // cursor was at 0 already
    match p.handle(Key::Enter) {
        Step::Submit(s) => assert_eq!(s.index, 0),
        other => panic!("got {other:?}"),
    }
}

#[test]
fn select_frame_marks_active_option() {
    let p = select("Pick one").option("a", "Alpha").option("b", "Beta");
    let f = render_frame(&p, None);
    // The option list starts after label + spacer (2 lines).
    let alpha_line = f.iter().find(|l| l.contains("Alpha")).unwrap();
    let beta_line = f.iter().find(|l| l.contains("Beta")).unwrap();
    assert!(
        alpha_line.contains("●"),
        "active row should use filled radio, got {alpha_line:?}"
    );
    assert!(
        beta_line.contains("○"),
        "idle row should use hollow radio, got {beta_line:?}"
    );
}
