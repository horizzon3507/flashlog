mod args;
mod detector;
mod render;

use args::{Args, Level};
use detector::Kind;
use std::collections::VecDeque;
use std::io::{self, BufRead, BufWriter, Write};
use std::process::exit;

#[derive(Default)]
struct Counts {
    total: usize,
    fatal: usize,
    secret: usize,
    error: usize,
    trace: usize,
}

impl Counts {
    fn add(&mut self, kind: Option<Kind>) {
        self.total += 1;
        match kind {
            Some(Kind::Fatal) => self.fatal += 1,
            Some(Kind::Secret) => self.secret += 1,
            Some(Kind::Error) => self.error += 1,
            Some(Kind::StackTrace) => self.trace += 1,
            None => {}
        }
    }

    fn matches(&self) -> usize {
        self.fatal + self.secret + self.error + self.trace
    }
}

fn parse_args() -> Args {
    match args::parse(std::env::args().skip(1)) {
        Ok(args) => args,
        Err(error) if error == "help" => {
            println!("{}", args::usage());
            exit(0);
        }
        Err(error) if error == "version" => {
            println!("flashlog {}", env!("CARGO_PKG_VERSION"));
            exit(0);
        }
        Err(error) => {
            eprintln!("flashlog: {error}\n\n{}", args::usage());
            exit(2);
        }
    }
}

fn main() {
    let args = parse_args();
    if let Err(error) = run(&args) {
        eprintln!("flashlog: {error}");
        exit(1);
    }
}

fn run(args: &Args) -> io::Result<()> {
    let stdin = io::stdin();
    let stdout = io::stdout();
    let is_terminal = std::io::IsTerminal::is_terminal(&stdout);
    let color = render::color_enabled(
        args.color,
        is_terminal,
        std::env::var_os("NO_COLOR").is_some(),
    );
    let mut out = BufWriter::with_capacity(64 * 1024, stdout.lock());
    let files = if args.files.is_empty() {
        vec![std::path::PathBuf::from("-")]
    } else {
        args.files.clone()
    };
    let mut counts = Counts::default();

    for path in files {
        if path == std::path::Path::new("-") {
            process_reader(stdin.lock(), None, args, color, &mut counts, &mut out)?;
        } else {
            let file = std::fs::File::open(&path).map_err(|error| {
                io::Error::new(error.kind(), format!("{}: {error}", path.display()))
            })?;
            process_reader(
                io::BufReader::with_capacity(64 * 1024, file),
                Some(path.display().to_string()),
                args,
                color,
                &mut counts,
                &mut out,
            )?;
        }
    }
    out.flush()?;
    if args.summary {
        eprintln!(
            "lines: {} | matches: {} (fatal {}, secret {}, error {}, trace {})",
            counts.total,
            counts.matches(),
            counts.fatal,
            counts.secret,
            counts.error,
            counts.trace
        );
    }
    Ok(())
}

fn allowed(kind: Kind, args: &Args) -> bool {
    let level = match kind {
        Kind::Fatal => Level::Fatal,
        Kind::Secret => Level::Secret,
        Kind::Error => Level::Error,
        Kind::StackTrace => Level::Trace,
    };
    args.levels
        .as_ref()
        .map_or(true, |levels| levels.contains(&level))
        && !args.exclude.contains(&level)
}

/// Streams newline-delimited chunks out of a buffered reader without one
/// allocation per line: lines that fit in the reader's buffer are borrowed
/// directly, and only lines spanning a buffer refill touch the carry buffer.
struct LineScanner<R> {
    reader: R,
    carry: Vec<u8>,
}

impl<R: BufRead> LineScanner<R> {
    fn new(reader: R) -> Self {
        Self {
            reader,
            carry: Vec::new(),
        }
    }

    fn for_each_line(&mut self, mut f: impl FnMut(&[u8]) -> io::Result<()>) -> io::Result<()> {
        loop {
            let buf = self.reader.fill_buf()?;
            if buf.is_empty() {
                if !self.carry.is_empty() {
                    let carry = std::mem::take(&mut self.carry);
                    f(trim_cr(&carry))?;
                }
                return Ok(());
            }
            match buf.iter().position(|&byte| byte == b'\n') {
                Some(pos) => {
                    if self.carry.is_empty() {
                        f(trim_cr(&buf[..pos]))?;
                    } else {
                        self.carry.extend_from_slice(&buf[..pos]);
                        let carry = std::mem::take(&mut self.carry);
                        f(trim_cr(&carry))?;
                    }
                    self.reader.consume(pos + 1);
                }
                None => {
                    let len = buf.len();
                    self.carry.extend_from_slice(buf);
                    self.reader.consume(len);
                }
            }
        }
    }
}

fn trim_cr(line: &[u8]) -> &[u8] {
    line.strip_suffix(b"\r").unwrap_or(line)
}

fn inspect_bytes(line: &[u8]) -> Option<Kind> {
    match std::str::from_utf8(line) {
        Ok(text) => detector::inspect(text),
        Err(_) => detector::inspect(&String::from_utf8_lossy(line)),
    }
}

fn process_reader<R: BufRead, W: Write>(
    reader: R,
    file: Option<String>,
    args: &Args,
    color: bool,
    counts: &mut Counts,
    out: &mut W,
) -> io::Result<()> {
    let mut scanner = LineScanner::new(reader);
    let mut line_number = 0usize;
    // --only --context N sliding window: non-matching lines held back while
    // they might still precede a match, and a countdown of trailing context
    // lines to print after one. When --only is off, every line prints and the
    // window is irrelevant, so it stays at zero cost.
    let context = if args.only && !args.json { args.context } else { 0 };
    let mut before: VecDeque<Vec<u8>> = VecDeque::new();
    let mut after = 0usize;

    scanner.for_each_line(|line| {
        line_number += 1;
        let raw_kind = inspect_bytes(line);
        counts.add(raw_kind);
        let kind = raw_kind.filter(|kind| allowed(*kind, args));

        if args.json {
            if let Some(kind) = kind {
                render::write_json_line(out, kind, line, file.as_deref(), line_number)?;
            }
            return Ok(());
        }

        let write_location = |out: &mut W| -> io::Result<()> {
            if let Some(name) = file.as_deref() {
                write!(out, "{name}:{line_number}: ")?;
            }
            Ok(())
        };

        if !args.only {
            write_location(out)?;
            return render::write_line(out, line, kind, color);
        }

        if let Some(kind) = kind {
            let mut pending_number = line_number - before.len();
            for pending in before.drain(..) {
                write_location_pending(out, file.as_deref(), pending_number)?;
                render::write_line(out, &pending, None, color)?;
                pending_number += 1;
            }
            write_location(out)?;
            render::write_line(out, line, Some(kind), color)?;
            after = context;
        } else if after > 0 {
            write_location(out)?;
            render::write_line(out, line, None, color)?;
            after -= 1;
        } else if context > 0 {
            before.push_back(line.to_vec());
            if before.len() > context {
                before.pop_front();
            }
        }
        Ok(())
    })
}

fn write_location_pending<W: Write>(
    out: &mut W,
    file: Option<&str>,
    line_number: usize,
) -> io::Result<()> {
    if let Some(name) = file {
        write!(out, "{name}:{line_number}: ")?;
    }
    Ok(())
}
