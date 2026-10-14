//! Examples and ticket or ADR numbers in a comment go stale; the code and the commit body carry them.

mod common;

use common::source_files;

const EXAMPLE_MARKERS: [&str; 4] = ["as in `", "like `", "e.g.", "for example"];

#[test]
fn comments_hold_no_examples_or_references() {
    let mut offenders = Vec::new();
    for (path, text) in source_files() {
        for (index, line) in text.lines().enumerate() {
            let line = line.trim_start();
            if line.starts_with("//") && (has_example(line) || has_reference(line)) {
                offenders.push(format!("{}:{}: {line}", path.display(), index + 1));
            }
        }
    }

    assert!(
        offenders.is_empty(),
        "move these examples or references out of the comments, into the commit body:\n{}",
        offenders.join("\n")
    );
}

fn has_example(comment: &str) -> bool {
    EXAMPLE_MARKERS
        .iter()
        .any(|marker| comment.contains(marker))
}

fn has_reference(comment: &str) -> bool {
    comment.contains("ADR")
        || comment
            .as_bytes()
            .windows(2)
            .any(|pair| pair[0] == b'#' && pair[1].is_ascii_digit())
}
