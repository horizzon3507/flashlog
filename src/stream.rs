use std::io::BufRead;

/// Stream lines from stdin as they arrive. Lossless for text with valid UTF-8;
/// invalid bytes are replaced, matching the Unix filter tradition.
pub fn lines<R: BufRead>(reader: R) -> impl Iterator<Item = String> {
    reader
        .lines()
        .map(|l| l.unwrap_or_else(|_| String::new()))
}
