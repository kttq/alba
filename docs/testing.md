# Testing

How this project tests. `tdd` reads this file before it writes a test. `retro` changes it, and only when a session gives evidence for a lesson or an exemplar.

## Commands

CI runs these on Linux and macOS. Run them before each commit:

- `cargo fmt --check`
- `cargo clippy --all-targets -- -D warnings`
- `cargo test`

Accept a new or changed `insta` snapshot with `INSTA_UPDATE=always cargo test`, then read the `.snap` file before you commit it. This works without `cargo-insta`.

## Seams

- **App core** (`alba::app::App`): drive it with `key(event)` and render it into ratatui's `TestBackend` at a fixed instant in an explicit time zone. It owns everything the pane shows.
- **Config parsing** (`alba::config::Config::parse`): feed it TOML text. Assert the typed config, or an error's line and a stable fragment of its reason. It owns validation and line numbers.
- **Config file** (`alba::config::load`): point it at a path under a fresh temporary home. It owns the first-run template and unreadable files.
- **Source search** (`tests/no_wrap.rs`): reads `src/` and fails on a banned API. It owns rules the compiler cannot express.
- **The real binary**: CLI flags, terminal restore, and tick timing. No automated test enters here; the verification skill drives it.

## Exemplars

- [`text_clock_is_centred_in_the_pane`](../tests/app.rs): a character snapshot of the whole pane.
- [`pane_narrower_than_the_clock_renders_nothing`](../tests/app.rs): a plain `assert_buffer_lines`, for an exact expected grid.
- [`missing_config_writes_the_template_and_loads_defaults`](../tests/config.rs): a file test in a fresh `temporary_home`, compared against a copy of the spec's text.
