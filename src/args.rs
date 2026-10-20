use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorMode {
    Auto,
    Always,
    Never,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Level {
    Fatal,
    Secret,
    Error,
    Trace,
}

impl Level {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "fatal" => Some(Self::Fatal),
            "secret" => Some(Self::Secret),
            "error" => Some(Self::Error),
            "trace" => Some(Self::Trace),
            _ => None,
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct Args {
    pub only: bool,
    pub json: bool,
    pub color_explicit: bool,
    pub color: ColorMode,
    pub context: usize,
    pub summary: bool,
    pub levels: Option<Vec<Level>>,
    pub exclude: Vec<Level>,
    pub files: Vec<PathBuf>,
}

impl Default for Args {
    fn default() -> Self {
        Self {
            only: false,
            json: false,
            color_explicit: false,
            color: ColorMode::Auto,
            context: 0,
            summary: false,
            levels: None,
            exclude: Vec::new(),
            files: Vec::new(),
        }
    }
}

pub fn parse<I>(args: I) -> Result<Args, String>
where
    I: IntoIterator<Item = String>,
{
    let mut parsed = Args::default();
    let mut args = args.into_iter();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--only" => parsed.only = true,
            "--json" => parsed.json = true,
            "--summary" => parsed.summary = true,
            "--color" => {
                parsed.color_explicit = true;
                let value = args
                    .next()
                    .ok_or("--color requires auto, always, or never")?;
                parsed.color = match value.as_str() {
                    "auto" => ColorMode::Auto,
                    "always" => ColorMode::Always,
                    "never" => ColorMode::Never,
                    _ => {
                        return Err(format!(
                            "invalid color mode '{value}' (expected auto, always, or never)"
                        ))
                    }
                };
            }
            "--context" => {
                let value = args
                    .next()
                    .ok_or("--context requires a non-negative integer")?;
                parsed.context = value
                    .parse()
                    .map_err(|_| format!("invalid context count '{value}'"))?;
            }
            "--levels" => {
                parsed.levels = Some(parse_levels(
                    &args
                        .next()
                        .ok_or("--levels requires a comma-separated list")?,
                )?)
            }
            "--exclude" => parsed.exclude.extend(parse_levels(
                &args
                    .next()
                    .ok_or("--exclude requires a comma-separated list")?,
            )?),
            "-h" | "--help" => return Err("help".into()),
            "-V" | "--version" => return Err("version".into()),
            "--" => {
                parsed.files.extend(args.map(PathBuf::from));
                break;
            }
            value if value.starts_with('-') && value != "-" => {
                return Err(format!("unknown option '{value}'"))
            }
            value => parsed.files.push(PathBuf::from(value)),
        }
    }
    if parsed.json && parsed.color_explicit && parsed.color != ColorMode::Never {
        return Err("--color always cannot be combined with --json".into());
    }
    Ok(parsed)
}

fn parse_levels(value: &str) -> Result<Vec<Level>, String> {
    if value.is_empty() {
        return Err("level list cannot be empty".into());
    }
    value
        .split(',')
        .map(|name| {
            Level::parse(name).ok_or_else(|| {
                format!("unknown level '{name}' (expected fatal,secret,error,trace)")
            })
        })
        .collect()
}

pub fn usage() -> &'static str {
    "flashlog — highlight errors, secrets, and stack traces\n\
     Usage: <command> | flashlog [options] [FILE ...]\n\
     Options:\n\
       --only                 print only suspicious lines\n\
       --json                 emit matching lines as JSON objects\n\
       --color auto|always|never  control terminal colors\n\
       --context N            show N nearby lines on each side of matches\n\
       --summary              print counts by category to stderr\n\
       --levels LIST          include categories: fatal,secret,error,trace\n\
       --exclude LIST         exclude categories: fatal,secret,error,trace\n\
       -h, --help             show this help\n\
       -V, --version          show version\n\
     Use FILE '-' to read stdin explicitly."
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_args(values: &[&str]) -> Result<Args, String> {
        parse(values.iter().map(|value| (*value).to_owned()))
    }

    #[test]
    fn parses_file_context_and_filters() {
        let parsed = parse_args(&["--context", "2", "--levels", "fatal,error", "app.log"]).unwrap();
        assert_eq!(parsed.context, 2);
        assert_eq!(parsed.levels, Some(vec![Level::Fatal, Level::Error]));
        assert_eq!(parsed.files, vec![PathBuf::from("app.log")]);
    }

    #[test]
    fn rejects_unknown_level_and_json_color_conflict() {
        assert!(parse_args(&["--levels", "warn"])
            .unwrap_err()
            .contains("unknown level"));
        assert!(parse_args(&["--json", "--color", "always"])
            .unwrap_err()
            .contains("cannot be combined"));
        assert!(parse_args(&["--json", "--color", "auto"]).is_err());
        assert!(parse_args(&["--json", "--color", "never"]).is_ok());
    }
}
