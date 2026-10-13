//! alba never wraps text: wrapping breaks the pixel art, so every line must fit its area.

use std::{fs, path::Path};

fn rust_files(directory: &Path) -> Vec<std::path::PathBuf> {
    let mut files = Vec::new();
    for entry in fs::read_dir(directory).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            files.extend(rust_files(&path));
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            files.push(path);
        }
    }
    files
}

#[test]
fn source_never_uses_wrap() {
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let files = rust_files(&source);
    assert!(
        !files.is_empty(),
        "no Rust files under {}",
        source.display()
    );

    let offenders: Vec<_> = files
        .iter()
        .filter(|path| fs::read_to_string(path).unwrap().contains("Wrap"))
        .collect();

    assert!(
        offenders.is_empty(),
        "ratatui's Wrap found in {offenders:?}"
    );
}
