//! Viewport helper — ports `@clack/prompts` `limitOptions()`.
//!
//! Slides a window of size `max` over a list so the active cursor stays
//! visible. Returns `(start, end)` half-open indices, plus flags telling
//! callers whether to render `↑ N more` / `↓ N more` indicators.

pub struct Viewport {
    pub start: usize,
    pub end: usize,
    pub above: usize,
    pub below: usize,
}

pub fn limit_options(total: usize, cursor: usize, max: usize) -> Viewport {
    if total <= max {
        return Viewport {
            start: 0,
            end: total,
            above: 0,
            below: 0,
        };
    }
    let half = max / 2;
    let start = if cursor < half {
        0
    } else if cursor + (max - half) > total {
        total - max
    } else {
        cursor - half
    };
    let end = (start + max).min(total);
    Viewport {
        start,
        end,
        above: start,
        below: total - end,
    }
}
