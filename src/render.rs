use crate::args::ColorMode;
use crate::detector::Kind;
use std::io::{self, Write};

pub fn color_enabled(mode: ColorMode, stdout_is_terminal: bool, no_color: bool) -> bool {
    match mode {
        ColorMode::Always => true,
        ColorMode::Never => false,
        ColorMode::Auto => stdout_is_terminal && !no_color,
    }
}

fn label(kind: Kind) -> &'static str {
    match kind {
        Kind::Fatal => "✗ FATAL",
        Kind::Secret => "⛏ SECRET",
        Kind::Error => "✗ error",
        Kind::StackTrace => "↳ trace",
    }
}

fn tag(kind: Kind) -> &'static str {
    match kind {
        Kind::Fatal => "\x1b[1;91m✗ FATAL\x1b[0m",
        Kind::Secret => "\x1b[7m⛏ SECRET\x1b[0m",
        Kind::Error => "\x1b[1;31m✗ error\x1b[0m",
        Kind::StackTrace => "\x1b[36m↳ trace\x1b[0m",
    }
}

/// Write one output line, with label and optional highlighting, without
/// building an intermediate String.
pub fn write_line<W: Write>(
    out: &mut W,
    line: &[u8],
    kind: Option<Kind>,
    color: bool,
) -> io::Result<()> {
    match (kind, color) {
        (None, _) => out.write_all(line)?,
        (Some(kind), false) => {
            out.write_all(label(kind).as_bytes())?;
            out.write_all(b" ")?;
            out.write_all(line)?;
        }
        (Some(kind), true) => {
            out.write_all(tag(kind).as_bytes())?;
            out.write_all(b" \x1b[7m")?;
            out.write_all(line)?;
            out.write_all(b"\x1b[0m")?;
        }
    }
    out.write_all(b"\n")
}

/// Write one JSON match object: kind, line, file (null on stdin), line_number.
/// Escapes exactly like a JSON string must, without a serialization library.
pub fn write_json_line<W: Write>(
    out: &mut W,
    kind: Kind,
    line: &[u8],
    file: Option<&str>,
    line_number: usize,
) -> io::Result<()> {
    let line = String::from_utf8_lossy(line);
    out.write_all(b"{\"kind\":\"")?;
    out.write_all(kind.as_str().as_bytes())?;
    out.write_all(b"\",\"line\":\"")?;
    write_json_escaped(out, line.as_bytes())?;
    match file {
        Some(file) => {
            out.write_all(b"\",\"file\":\"")?;
            write_json_escaped(out, file.as_bytes())?;
            out.write_all(b"\",\"line_number\":")?;
        }
        None => out.write_all(b"\",\"file\":null,\"line_number\":")?,
    }
    write!(out, "{line_number}}}")?;
    out.write_all(b"\n")
}

fn write_json_escaped<W: Write>(out: &mut W, bytes: &[u8]) -> io::Result<()> {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    for &byte in bytes {
        match byte {
            b'"' => out.write_all(b"\\\"")?,
            b'\\' => out.write_all(b"\\\\")?,
            b'\n' => out.write_all(b"\\n")?,
            b'\r' => out.write_all(b"\\r")?,
            b'\t' => out.write_all(b"\\t")?,
            0x08 => out.write_all(b"\\b")?,
            0x0c => out.write_all(b"\\f")?,
            0x00..=0x1f => {
                out.write_all(b"\\u00")?;
                out.write_all(&[HEX[(byte >> 4) as usize], HEX[(byte & 0xf) as usize]])?;
            }
            _ => out.write_all(&[byte])?,
        }
    }
    Ok(())
}

#[cfg(test)]
fn flash(line: &str, kind: Kind, color: bool) -> String {
    if color {
        let mut rendered = String::with_capacity(line.len() + 24);
        rendered.push_str(tag(kind));
        rendered.push_str(" \x1b[7m");
        rendered.push_str(line);
        rendered.push_str("\x1b[0m");
        rendered
    } else {
        let mut rendered = String::with_capacity(line.len() + 10);
        rendered.push_str(label(kind));
        rendered.push(' ');
        rendered.push_str(line);
        rendered
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

    #[test]
    fn write_line_matches_flash_output() {
        for kind in [Kind::Fatal, Kind::Secret, Kind::Error, Kind::StackTrace] {
            for color in [false, true] {
                let mut buf = Vec::new();
                write_line(&mut buf, b"failed", Some(kind), color).unwrap();
                assert_eq!(buf, format!("{}\n", flash("failed", kind, color)).as_bytes());
            }
        }
        let mut buf = Vec::new();
        write_line(&mut buf, b"quiet line", None, true).unwrap();
        assert_eq!(buf, b"quiet line\n");
    }

    #[test]
    fn json_output_escapes_control_characters() {
        let mut buf = Vec::new();
        write_json_line(&mut buf, Kind::Error, b"a \"quote\" and \\slash\t\x01", Some("f.log"), 7).unwrap();
        assert_eq!(
            String::from_utf8(buf).unwrap(),
            "{\"kind\":\"error\",\"line\":\"a \\\"quote\\\" and \\\\slash\\t\\u0001\",\"file\":\"f.log\",\"line_number\":7}\n"
        );
        let mut buf = Vec::new();
        write_json_line(&mut buf, Kind::Fatal, b"boom", None, 3).unwrap();
        assert_eq!(
            String::from_utf8(buf).unwrap(),
            "{\"kind\":\"fatal\",\"line\":\"boom\",\"file\":null,\"line_number\":3}\n"
        );
    }
}
