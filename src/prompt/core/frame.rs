//! A `Frame` is one screen-worth of prompt output, one entry per terminal row.

/// One screen's worth of prompt output. Each `String` is exactly one
/// terminal row (newlines are added by the runner). Owning the row split
/// here means the runner can track line counts without re-parsing strings.
pub type Frame = Vec<String>;

/// Information the runner hands to `Prompt::render` on every frame.
pub struct RenderCtx<'a> {
    /// `Some(msg)` when the previous `handle()` returned [`Reject`]; prompts
    /// should switch their header glyph to `▲`, color the input bar and
    /// bottom frame yellow, and place `msg` on the bottom frame line.
    ///
    /// [`Reject`]: super::Step::Reject
    pub error: Option<&'a str>,
}
