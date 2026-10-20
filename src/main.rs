mod args;
mod detector;
mod render;
mod stream;

use args::{Args, Level};
use detector::Kind;
use serde::Serialize;
use std::io::{self, BufWriter, Write};
use std::process::exit;

#[derive(Serialize)]
struct JsonLine<'a> {
    kind: &'static str,
    line: &'a str,
    file: Option<&'a str>,
    line_number: usize,
}

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
    let mut out = BufWriter::new(stdout.lock());
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
                io::BufReader::new(file),
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

fn process_reader<R: io::BufRead, W: Write>(
    reader: R,
    file: Option<String>,
    args: &Args,
    color: bool,
    counts: &mut Counts,
    out: &mut W,
) -> io::Result<()> {
    if args.context == 0 {
        for (index, line) in reader.lines().enumerate() {
            let line = line?;
            let raw_kind = detector::inspect(&line);
            counts.add(raw_kind);
            let kind = raw_kind.filter(|kind| allowed(*kind, args));
            let line_number = index + 1;
            if args.json {
                if let Some(kind) = kind {
                    let value = JsonLine {
                        kind: kind.as_str(),
                        line: &line,
                        file: file.as_deref(),
                        line_number,
                    };
                    serde_json::to_writer(&mut *out, &value)?;
                    writeln!(out)?;
                }
            } else if let Some(kind) = kind {
                let location = file
                    .as_deref()
                    .map(|name| format!("{name}:{line_number}: "))
                    .unwrap_or_default();
                writeln!(out, "{location}{}", render::flash(&line, kind, color))?;
            } else if !args.only {
                let location = file
                    .as_deref()
                    .map(|name| format!("{name}:{line_number}: "))
                    .unwrap_or_default();
                writeln!(out, "{location}{line}")?;
            }
        }
        return Ok(());
    }

    let lines = reader.lines().collect::<io::Result<Vec<_>>>()?;
    let raw_kinds: Vec<_> = lines.iter().map(|line| detector::inspect(line)).collect();
    for kind in &raw_kinds {
        counts.add(*kind);
    }
    let kinds: Vec<_> = raw_kinds
        .into_iter()
        .map(|kind| kind.filter(|kind| allowed(*kind, args)))
        .collect();
    let selected = stream::selected_lines(&kinds, args.only, args.context);

    for (index, (line, kind)) in lines.iter().zip(kinds.iter()).enumerate() {
        if !selected[index] {
            continue;
        }
        if args.json {
            if let Some(kind) = kind {
                let value = JsonLine {
                    kind: kind.as_str(),
                    line,
                    file: file.as_deref(),
                    line_number: index + 1,
                };
                serde_json::to_writer(&mut *out, &value)?;
                writeln!(out)?;
            }
        } else {
            let location = file
                .as_deref()
                .map(|name| format!("{name}:{}: ", index + 1))
                .unwrap_or_default();
            if let Some(kind) = kind {
                writeln!(out, "{location}{}", render::flash(line, *kind, color))?;
            } else if !args.only || args.context > 0 {
                writeln!(out, "{location}{line}")?;
            }
        }
    }
    Ok(())
}
