use std::sync::OnceLock;

/// What kind of suspicious thing a line is. Drives the visual treatment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Error,
    StackTrace,
    Secret,
    Fatal,
}

impl Kind {
    pub fn as_str(self) -> &'static str {
        match self {
            Kind::Error => "error",
            Kind::StackTrace => "stack-trace",
            Kind::Secret => "secret",
            Kind::Fatal => "fatal",
        }
    }
}

/// One pure rule: regular expression + the kind it signals.
struct Rule {
    re: &'static str,
    kind: Kind,
}

/// Heuristic rules, kept in one place. Ordered; first hit wins.
const RULES: &[Rule] = &[
    // Secrets first: a leaked key inside an error line is still a secret.
    Rule { re: r"AKIA[0-9A-Z]{16}", kind: Kind::Secret },
    Rule { re: r"(?i)\bsk-[A-Za-z0-9_-]{20,}", kind: Kind::Secret },
    Rule { re: r"\bghp_[A-Za-z0-9]{36,}", kind: Kind::Secret },
    Rule { re: r"\bxox[bp]-[A-Za-z0-9-]{10,}", kind: Kind::Secret },
    Rule { re: r"-----BEGIN [A-Z ]*PRIVATE KEY-----", kind: Kind::Secret },
    Rule { re: r"(?i)\b(bearer|authorization)\s*[:=]\s*\S{8,}", kind: Kind::Secret },
    Rule { re: r#"(?i)\b(password|passwd|secret|api[_-]?key|token)\s*[=:]\s*['"]?\S{6,}"#, kind: Kind::Secret },
    // Fatal outranks plain errors.
    Rule { re: r"(?i)\b(fatal|panicked|segmentation fault|core dumped)\b|\bpanic!\b", kind: Kind::Fatal },
    // Stack trace shapes (Rust, Python, JS, Go): file:line anchors and frames.
    Rule { re: r"^\s+at\s\S+\(?.*:\d+:\d+\)?\s*$", kind: Kind::StackTrace },
    Rule { re: r"^\s*(#\d+|\d{1,3}:)\s+0x[0-9a-fA-F]+\s+\S", kind: Kind::StackTrace },
    Rule { re: r"\S+\.(rs|py|ts|tsx|js|go|c|cpp|h):\d+(:\d+)?", kind: Kind::StackTrace },
    Rule { re: r"Traceback \(most recent call last\):", kind: Kind::StackTrace },
    // Exit status / build failures.
    Rule { re: r"(?i)\bexit( code| status)?[: ]+[1-9]\d*\b", kind: Kind::Error },
    Rule { re: r"(?i)\bfailed to (compile|build|link|start|connect|bind|load)\b", kind: Kind::Error },
    // Plain level words.
    Rule { re: r"(?i)\[(error|err)\]|\b(error|err)[!:]\s|\bexception\b|\btraceback\b|\berror\b", kind: Kind::Error },
];

fn compiled() -> &'static Vec<(regex::Regex, Kind)> {
    static RE: OnceLock<Vec<(regex::Regex, Kind)>> = OnceLock::new();
    RE.get_or_init(|| {
        RULES
            .iter()
            .filter_map(|r| regex::Regex::new(r.re).ok().map(|re| (re, r.kind)))
            .collect()
    })
}

/// Inspect one line. Returns the winning kind, or None when the line is innocent.
pub fn inspect(line: &str) -> Option<Kind> {
    compiled().iter().find(|(re, _)| re.is_match(line)).map(|(_, k)| *k)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catches_errors() {
        assert_eq!(inspect("[error] connection refused"), Some(Kind::Error));
        assert_eq!(inspect("error: could not compile `flashlog`"), Some(Kind::Error));
        assert_eq!(inspect("exit status 1"), Some(Kind::Error));
    }

    #[test]
    fn catches_fatal_over_error() {
        assert_eq!(inspect("thread 'main' panicked at src/main.rs:2:5:"), Some(Kind::Fatal));
        assert_eq!(inspect("Segmentation fault (core dumped)"), Some(Kind::Fatal));
    }

    #[test]
    fn catches_secrets() {
        assert_eq!(inspect("AWS_KEY=AKIAIOSFODNN7EXAMPLE"), Some(Kind::Secret));
        assert_eq!(inspect("token: sk-abcdefghijklmnopq123456"), Some(Kind::Secret));
        assert_eq!(inspect("-----BEGIN RSA PRIVATE KEY-----"), Some(Kind::Secret));
    }

    #[test]
    fn catches_stack_traces() {
        assert_eq!(inspect("    at foo (bar.js:10:15)"), Some(Kind::StackTrace));
        assert_eq!(inspect("  14: 0x55f2a1 - core::ptr::drop_in_place"), Some(Kind::StackTrace));
        assert_eq!(inspect("src/main.rs:42:9"), Some(Kind::StackTrace));
    }

    #[test]
    fn innocent_lines_pass() {
        assert_eq!(inspect("2026-10-18 09:12:03 all systems nominal"), None);
        assert_eq!(inspect("hello world"), None);
        assert_eq!(inspect(" temperament: cozy"), None);
    }
}
