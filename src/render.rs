use crate::detector::Kind;

/// Render one suspicious line: resolved darkness. Blink would flicker per-print,
/// so we flash with a strong marker on arrival and keep it inverse-bold after.
pub fn flash(line: &str, kind: Kind) -> String {
    let tag = match kind {
        Kind::Fatal => "\x1b[1;91m✗ FATAL\x1b[0m",
        Kind::Secret => "\x1b[7m⛏ SECRET\x1b[0m",
        Kind::Error => "\x1b[1;31m✗ error\x1b[0m",
        Kind::StackTrace => "\x1b[36m↳ trace\x1b[0m",
    };
    format!("{tag} \x1b[7m{line}\x1b[0m")
}
