//! alba never wraps text: wrapping breaks the pixel art, so every line must fit its area.

mod common;

use common::source_files;

#[test]
fn source_never_uses_wrap() {
    let offenders: Vec<_> = source_files()
        .into_iter()
        .filter(|(_, text)| text.contains("Wrap"))
        .map(|(path, _)| path)
        .collect();

    assert!(
        offenders.is_empty(),
        "ratatui's Wrap found in {offenders:?}"
    );
}
