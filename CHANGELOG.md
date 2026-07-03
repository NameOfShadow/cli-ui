# Changelog

All notable changes to **cli-ui** are documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
Versions follow [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] — 2026-07-03

The first usable release. Ships two pillars: derive-based argument parsing
and clack-style interactive prompts.

### Added — Public API ergonomics

- **Promoted rule library to the prompt root.** Every validator
  (`min_chars`, `has_upper`, `email`, `int_between`, `one_of`, … 20 in
  total) plus `Validator` and the `Validate` trait re-exported from
  `cli_ui::prompt::*`. Write `use cli_ui::prompt::{text, min_chars};`
  instead of `use cli_ui::prompt::validate::min_chars;`.
- **Promoted colour theme ops to the prompt root.** `colors`,
  `update_colors`, `set_colors`, and `Colors` re-exported from
  `cli_ui::prompt::*`. `settings::*` is still available for the rest.
- **New `.rule(Validator)` method** on `text`, `secret`, `multiline` —
  short, parallel to `.validate(closure)`, scans as "this prompt obeys
  these rules."
- **Prelude expansion**: `use cli_ui::prompt::prelude::*` now also
  brings in the rule library, `Validator`, the colour-theme ops, and
  `Colors`.

### Deprecated

- `.validate_with(Validator)` — renamed to `.rule(Validator)`. The old
  method still compiles with a deprecation warning.

### Added — CLI argument parsing

- `#[derive(CliOptions)]` — typed argument parsing with no boilerplate.
- `#[arg(...)]` attribute language: `positional`, `short`, `long`,
  `default`, `validate(...)`, `action(...)`, `section`, `conflicts_with`,
  `requires`.
- `#[cli(...)]` struct attributes: `about`, `theme`, `version_label`, …
- Auto-generated `--help`, `--version`, `--completions` (bash / zsh / fish).
- Section-aware help layout with key alignment.
- Built-in validators: `exists`, `is_file`, `is_dir`, `ext(...)`,
  `range(...)`, `one_of(...)`, `parsable_as_*`.
- `header!`, `phase!`, `step!`, `substep!`, `ok!`, `bail!`, `summary!`
  macros for consistent CLI output.
- `Progress` widget with counter, URL truncation, and local/remote labels.
- Themes (`green`, `blue`, `red`, `yellow`, `magenta`, `cyan`) with
  one-line opt-in.
- Pipe safety — ANSI stripped automatically when output is redirected.

### Added — Interactive prompts (`interactive` feature)

- Question prompts: `text`, `secret`, `multiline`, `confirm`, `select`,
  `multiselect`, `groupmultiselect`, `autocomplete`, `select_key`,
  `date::date`, `path::path`.
- Framing helpers: `intro` (cyan-pill badge), `outro`, `note`, `cancel`,
  `boxed::boxed`, plus framed `log::{info,warn,error,success,step}` and
  streamed `stream::{info,warn,error,success,step}` lines.
- Live-work widgets: `spinner` (background thread, dot or timer indicator),
  `progress` bar, `tasks` sequential runner, `task_log` with retained
  output on error.
- Sequential composition: `group::group` collects answers from chained
  prompts into a `BTreeMap`.
- Composable validators in `validate::*` — `required`, `min_chars`,
  `max_chars`, `exact_chars`, `word_count`, `words_between`, `has_upper`,
  `has_lower`, `has_digit`, `has_special`, `alpha_only`, `alphanumeric`,
  `only_chars`, `forbid_chars`, `int_between`, `float_between`, `email`,
  `starts_with`, `ends_with`, `one_of` — composed with `.and()`, `.or()`,
  `.msg()`. External libraries can implement the `Validate` trait.
- Full color theme in `settings::Colors` (eleven slots), live-overridable
  via `settings::update_colors`. `settings::Settings` adds vim keybindings
  (`h/j/k/l`), bold headers, hint visibility, default cancel/error
  messages.
- Inline editing in text/secret/multiline: ← → Home End Delete Ctrl-W
  Ctrl-A Ctrl-E plus an inverse-video cursor block.
- Pagination viewport for long select/multiselect lists with
  `↑ N more` / `↓ N more` indicators.
- `OnCancel` extension trait — `result.or_cancel("Aborted")` exits cleanly
  on Ctrl-C with a framed `■` banner.
- Connected `┌ │ … └` frame across an entire prompt session.
- Yellow error frame: when validation fails, the `◆` glyph becomes `▲`,
  the input bar turns yellow, and the bottom frame embeds the error
  message — no extra row inserted.
- Answered state preserves both question and submitted value, plus a
  connector `│` that bridges into the next prompt.

### Added — Architecture

- `prompt::core::Prompt` trait — `handle(Key) -> Step<T>`,
  `render(RenderCtx) -> Frame`, `render_answered(&T) -> Frame`.
- `prompt::core::run` generic runner owns raw mode, the key loop,
  line-count bookkeeping, the validate-passes-redraw-clean transition,
  the answered redraw, and cancel cleanup.
- `prompt::core::Step<T>` — `Continue | Submit(T) | Cancel | Reject(msg)`.
- `prompt::core::Frame` — `Vec<String>`, one entry per terminal row.
- All eleven prompts implement the trait; per-prompt code dropped from
  ~190 → ~110 lines on average.

### Added — Tests & docs

- 12 snapshot tests in `tests/prompt_snapshots.rs` driving the `Prompt`
  trait without a terminal.
- 18 doc tests across the module documentation.
- `examples/hello_prompt.rs` — minimal three-prompt walkthrough.
- `examples/prompt.rs` — every prompt type.
- `examples/prompt_validate.rs` — composable validators + Ctrl-C handling.
- `prompt::prelude` glob-import for quick scripts.
- README "Prompts" section with category table, validation, theming,
  cancellation, and extension docs.

[0.1.0]: https://github.com/NameOfShadow/cli-ui/releases/tag/v0.1.0
