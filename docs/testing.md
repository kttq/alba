# Testing

How this project tests. `tdd` reads this file before it writes a test. `retro` changes it, and only when a session gives evidence for a lesson or an exemplar.

## Commands

CI runs these on Linux and macOS. Run them before each commit:

- `cargo fmt --check`
- `cargo clippy --all-targets -- -D warnings`
- `cargo test`

Accept a new or changed `insta` snapshot with `INSTA_UPDATE=always cargo test`, then read the `.snap` file before you commit it. This works without `cargo-insta`.

## Seams

- **App core** (`alba::app::App`): drive it with `tick(now)` and `key(event)`, and render it into ratatui's `TestBackend` at a fixed instant in an explicit time zone. Build it through `Harness` in `tests/common`: a `FakeWeatherSource` stands in for the network, and fresh temporary directories hold the cache and the log. It owns everything the pane shows, and the fetch, cache, and log flow behind it.
- **Face layout** (`App::face` and `alba::layout::lay_out`): build the face once at a fixed instant, then lay it out in any pane. Assert the layout report: the chosen size tier, and each placed item with its required size and its given area. `layout::minimum_size` gives a tier's smallest pane for that face. It owns the tier choice, the Strip drop order, and the centring, so the size sweep in `tests/size_sweep.rs` enters here instead of rendering every size.
- **Weather station** (`alba::weather::WeatherStation`): build it through `WeatherStationHarness` in `tests/common`, with a `FakeWeatherSource` and a fresh temporary cache directory. Drive it with `tick(now)` and `request_refresh()` at fixed instants, and assert its weather state, its fetch indicator, and when the fake receives each request. It owns the fetch schedule and the weather state. Enter here for a state that the render cannot show yet, such as fetching against offline.
- **Background weather source** (`alba::weather::BackgroundSource`): give it a fetch closure that blocks on a channel. It owns one-fetch-at-a-time and results that arrive once.
- **Pure functions** (`Condition::from_wmo`, `Paths::resolve`, `next_fetch`, `Weather::freshness`, `Motion::at`): call them directly. `Paths::resolve` takes the environment as a lookup closure, so no test sets a real variable.
- **Config parsing** (`alba::config::Config::parse`): feed it TOML text. Assert the typed config, or an error's line and a stable fragment of its reason. It owns validation and line numbers.
- **Config file** (`alba::config::load`): point it at a path under a fresh temporary home. It owns the first-run template and unreadable files.
- **Source search** (`tests/no_wrap.rs`): reads `src/` and fails on a banned API. It owns rules the compiler cannot express.
- **The real binary** (`CARGO_BIN_EXE_alba`): quit keys, terminal restore, resize, colors, and anything only the running process shows. Start it through `Alba` in `tests/common/pty.rs` on a scratch `HOME` written by `FixtureHome` in `tests/common/fixture_home.rs`. Read the screen through `vt100`: its text, and each cell's colors and dim flag. Every run there denies the network on macOS, and the tests that read the log prove that no other test fetches. Wait for a condition with a deadline, not for a fixed time, and compare a resized pane with a fresh start at that size, not with fixed rows, so the tests survive a layout change. Harness self-tests, with fake binaries, live in `tests/drive_harness.rs`. The look in a real terminal stays with the `verify-alba` skill, which drives the same harness through the `drive` example.

## Exemplars

- [`text_line_is_centred_in_the_pane`](../tests/app.rs): a character snapshot of the whole pane.
- [`pane_narrower_than_the_clock_renders_nothing`](../tests/app.rs): a plain `assert_buffer_lines`, for an exact expected grid.
- [`missing_config_writes_the_template_and_loads_defaults`](../tests/config.rs): a file test in a fresh `temporary_home`, compared against a copy of the spec's text.
- [`failed_fetches_show_no_weather`](../tests/weather.rs): a table of failure cases through `Harness`, each a fixture changed as JSON.
- [`each_quit_key_exits_on_its_own_and_restores_the_terminal`](../tests/binary.rs): the real binary on a pseudo-terminal, from a fixture home to a restored terminal.

## Lessons

- Change a structured fixture as data: parse it, set the field, and serialize it. Keep text edits for malformed input, so a format change in the fixture breaks no test.
