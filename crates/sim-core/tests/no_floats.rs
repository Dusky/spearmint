//! Spec 3.1: integer or fixed-point math only. No floats anywhere in the sim.
//!
//! `#![deny(clippy::float_arithmetic)]` in the crate root catches float *operations*,
//! but only when clippy runs. This reads the source, so a float cannot enter the core
//! through an ordinary `cargo test` — including as a struct field, a cast, or a
//! constant that nothing has done arithmetic on yet.

use std::path::{Path, PathBuf};

/// Rejected outright. `as f32` and `0.5f64` are both covered by the type names.
const FORBIDDEN: [&str; 2] = ["f32", "f64"];

#[test]
fn the_core_contains_no_floats() {
    let source_root = PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/src"));
    let mut offences = Vec::new();

    for file in rust_files(&source_root) {
        let text = std::fs::read_to_string(&file).expect("source file should be readable");
        for (number, line) in text.lines().enumerate() {
            // Comments are prose, and the prose here explains at length why floats are
            // banned. Scan the code only.
            let code = code_before_comment(line);
            for needle in FORBIDDEN {
                if contains_token(code, needle) {
                    offences.push(format!(
                        "{}:{}: {}",
                        file.display(),
                        number + 1,
                        line.trim()
                    ));
                }
            }
        }
    }

    assert!(
        offences.is_empty(),
        "floats are forbidden in the simulation core (spec 3.1):\n{}",
        offences.join("\n")
    );
}

/// Drops a trailing `//` comment, ignoring one that opens inside a string literal.
///
/// Block comments are not handled; this crate does not use them, and clippy's
/// `float_arithmetic` lint is the belt to this braces.
fn code_before_comment(line: &str) -> &str {
    let bytes = line.as_bytes();
    let mut in_string = false;
    let mut index = 0;
    while index < bytes.len() {
        match bytes[index] {
            b'\\' if in_string => index += 1, // skip the escaped byte
            b'"' => in_string = !in_string,
            b'/' if !in_string && bytes.get(index + 1) == Some(&b'/') => return &line[..index],
            _ => {}
        }
        index += 1;
    }
    line
}

/// Matches `f64` as a whole token, so an identifier or hex constant that merely
/// contains those characters does not trip the check.
fn contains_token(line: &str, needle: &str) -> bool {
    let bytes = line.as_bytes();
    let mut from = 0;
    while let Some(offset) = line[from..].find(needle) {
        let start = from + offset;
        let end = start + needle.len();
        let before_ok = start == 0 || !is_word_byte(bytes[start - 1]);
        let after_ok = end >= bytes.len() || !is_word_byte(bytes[end]);
        if before_ok && after_ok {
            return true;
        }
        from = start + 1;
    }
    false
}

const fn is_word_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}

fn rust_files(root: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let mut pending = vec![root.to_path_buf()];
    while let Some(directory) = pending.pop() {
        let entries = std::fs::read_dir(&directory).expect("source directory should be readable");
        for entry in entries {
            let path = entry.expect("directory entry should be readable").path();
            if path.is_dir() {
                pending.push(path);
            } else if path.extension().is_some_and(|extension| extension == "rs") {
                found.push(path);
            }
        }
    }
    assert!(!found.is_empty(), "found no source files to scan");
    found.sort();
    found
}
