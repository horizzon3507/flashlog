use crate::args::ColorMode;
use crate::detector::Kind;

pub fn color_enabled(mode: ColorMode, stdout_is_terminal: bool, no_color: bool) -> bool {
    match mode {
        ColorMode::Always => true,
        ColorMode::Never => false,
        ColorMode::Auto => stdout_is_terminal && !no_color,
    }
}

pub fn flash(line: &str, kind: Kind, color: bool) -> String {
    let label = match kind {
        Kind::Fatal => "✗ FATAL",
        Kind::Secret => "⛏ SECRET",
        Kind::Error => "✗ error",
        Kind::StackTrace => "↳ trace",
    };
    if color {
        let tag = match kind {
            Kind::Fatal => "\x1b[1;91m✗ FATAL\x1b[0m",
            Kind::Secret => "\x1b[7m⛏ SECRET\x1b[0m",
            Kind::Error => "\x1b[1;31m✗ error\x1b[0m",
            Kind::StackTrace => "\x1b[36m↳ trace\x1b[0m",
        };
        format!("{tag} \x1b[7m{line}\x1b[0m")
    } else {
        format!("{label} {line}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn color_auto_respects_tty_and_no_color() {
        assert!(color_enabled(ColorMode::Auto, true, false));
        assert!(!color_enabled(ColorMode::Auto, false, false));
        assert!(!color_enabled(ColorMode::Auto, true, true));
        assert!(color_enabled(ColorMode::Always, false, true));
        assert!(!color_enabled(ColorMode::Never, true, false));
    }

    #[test]
    fn plain_flash_has_no_escape_sequences() {
        assert_eq!(flash("failed", Kind::Error, false), "✗ error failed");
    }
}
