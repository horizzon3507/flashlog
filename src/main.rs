mod detector;
mod render;
mod stream;

use std::io::{self, BufWriter, Write};
use std::process::exit;

struct Args {
    /// Only print matching (suspicious) lines.
    only: bool,
    /// Output matching lines as JSON.
    json: bool,
}

fn usage() {
    eprintln!(
        "flashlog — highlight lines that matter: errors, secrets, stack traces\n\
         \n\
         Usage: <command-that-logs> | flashlog [options]\n\
         \n\
         Options:\n\
           --only    print only suspicious lines\n\
           --json    emit suspicious lines as JSON objects\n\
           -h, --help  show this help"
    );
}

fn parse_args() -> Args {
    let mut args = Args { only: false, json: false };
    for arg in std::env::args().skip(1) {
        match arg.as_str() {
            "--only" => args.only = true,
            "--json" => args.json = true,
            "-h" | "--help" => {
                usage();
                exit(0);
            }
            other => {
                eprintln!("flashlog: unknown option '{other}'");
                usage();
                exit(2);
            }
        }
    }
    args
}

fn main() -> io::Result<()> {
    let args = parse_args();
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut out = BufWriter::new(stdout.lock());

    for line in stream::lines(stdin.lock()) {
        let verdict = detector::inspect(&line);
        match verdict {
            Some(kind) => {
                if args.json {
                    writeln!(
                        out,
                        "{{\"kind\":\"{}\",\"line\":{}}}",
                        kind.as_str(),
                        serde_json_escape(&line)
                    )?;
                } else {
                    writeln!(out, "{}", render::flash(&line, kind))?;
                }
            }
            None => {
                if !args.only {
                    writeln!(out, "{line}")?;
                }
            }
        }
    }
    out.flush()
}

fn serde_json_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    format!("\"{out}\"")
}
