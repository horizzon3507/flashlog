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

/// Heuristic rules, kept in one place. Ordered; the lowest matching index wins,
/// exactly like scanning this list top to bottom.
const RULES: &[Rule] = &[
    // Secrets first: a leaked key inside an error line is still a secret.
    Rule {
        re: r"AKIA[0-9A-Z]{16}",
        kind: Kind::Secret,
    },
    Rule {
        re: r"(?i)\bsk-[A-Za-z0-9_-]{20,}",
        kind: Kind::Secret,
    },
    Rule {
        re: r"\bghp_[A-Za-z0-9]{36,}",
        kind: Kind::Secret,
    },
    Rule {
        re: r"\bxox[bp]-[A-Za-z0-9-]{10,}",
        kind: Kind::Secret,
    },
    Rule {
        re: r"-----BEGIN [A-Z ]*PRIVATE KEY-----",
        kind: Kind::Secret,
    },
    Rule {
        re: r"(?i)\b(bearer|authorization)\s*[:=]\s*\S{8,}",
        kind: Kind::Secret,
    },
    Rule {
        re: r"(?i)\bgh[pousr]_[A-Za-z0-9]{20,}",
        kind: Kind::Secret,
    },
    Rule {
        re: r"(?i)\b(token|api[_-]?key|password|secret)\b.{0,24}(redacted|<redacted>|\*{6,})",
        kind: Kind::Secret,
    },
    Rule {
        re: r#"(?i)\b(password|passwd|secret|api[_-]?key|token)\s*[=:]\s*['"]?\S{12,}"#,
        kind: Kind::Secret,
    },
    // Fatal outranks plain errors.
    Rule {
        re: r"(?i)\b(fatal|critical|panicked|segmentation fault|segfault|core dumped|abort trap)\b|\bpanic!\b",
        kind: Kind::Fatal,
    },
    Rule {
        re: r"(?i)\b(out of memory|oom-killed|killed process \d+)\b",
        kind: Kind::Fatal,
    },
    Rule {
        re: r"(?i)\b(panic occurred|unhandled exception|uncaught exception)\b",
        kind: Kind::Fatal,
    },
    Rule {
        re: r"(?i)^panic:\s",
        kind: Kind::Fatal,
    },
    // Stack trace shapes: file:line anchors, frames, and traceback headers.
    Rule {
        re: r"^\s+at\s\S+\(?.*:\d+:\d+\)?\s*$",
        kind: Kind::StackTrace,
    },
    Rule {
        re: r"^\s*at\s+[\w.$]+\.[\w$<>]+\([^()]*\)\s*$",
        kind: Kind::StackTrace,
    },
    Rule {
        re: r#"^\s*File ".*", line \d+(, in .+)?\s*$"#,
        kind: Kind::StackTrace,
    },
    Rule {
        re: r"^\s*Caused by:\s",
        kind: Kind::StackTrace,
    },
    Rule {
        re: r"^goroutine \d+ \[[^\]]+\]:",
        kind: Kind::StackTrace,
    },
    Rule {
        re: r"^\s*\d{1,3}:\s+\S*::\S",
        kind: Kind::StackTrace,
    },
    Rule {
        re: r"^\s*(#\d+|\d{1,3}:)\s+0x[0-9a-fA-F]+\s+\S",
        kind: Kind::StackTrace,
    },
    Rule {
        re: r"\S+\.(rs|py|ts|tsx|js|go|c|cpp|h|java|rb|cs):\d+(:\d+)?",
        kind: Kind::StackTrace,
    },
    Rule {
        re: r"Traceback \(most recent call last\):",
        kind: Kind::StackTrace,
    },
    // Exit status / build failures.
    Rule {
        re: r"(?i)\bexit( code| status)?[: ]+[1-9]\d*\b",
        kind: Kind::Error,
    },
    Rule {
        re: r"(?i)\bfailed to (compile|build|link|start|connect|bind|load|open|write|read)\b",
        kind: Kind::Error,
    },
    Rule {
        re: r"(?i)^\s*(\[(warn|warning)\]|(warn|warning):)",
        kind: Kind::Error,
    },
    // Uppercase level tokens and test/build verdicts (case-sensitive on purpose:
    // prose uses lowercase, which the prefix rule above already covers).
    Rule {
        re: r"\bWARN(ING)?\b",
        kind: Kind::Error,
    },
    Rule {
        re: r"\bFAILED\b",
        kind: Kind::Error,
    },
    // Exception class lines: `ValueError: ...`, `NullPointerException: ...`.
    Rule {
        re: r"^[A-Za-z_][\w.]*?(Error|Exception|Interrupt|Exit)(:\s.*)?$",
        kind: Kind::Error,
    },
    // Common runtime failure phrases and HTTP 5xx statuses.
    Rule {
        re: r"(?i)\b(connection refused|permission denied|no space left on device|cannot allocate memory|too many open files)\b",
        kind: Kind::Error,
    },
    Rule {
        re: r"(?i)\b(internal server error|bad gateway|service unavailable|gateway timeout)\b",
        kind: Kind::Error,
    },
    Rule {
        re: r"\bstatus[=: ] ?5\d\d\b|\bHTTP/[0-9.]+ 5\d\d\b",
        kind: Kind::Error,
    },
    // Plain level words.
    Rule {
        re: r"(?i)\[(error|err)\]|\b(error|err)[!:]\s|\bexception\b|\btraceback\b|\berror\b",
        kind: Kind::Error,
    },
];

fn compiled() -> &'static regex::RegexSet {
    static RE: OnceLock<regex::RegexSet> = OnceLock::new();
    RE.get_or_init(|| {
        regex::RegexSet::new(RULES.iter().map(|rule| rule.re))
            .expect("all detector rules must compile")
    })
}

/// Inspect one line. Returns the winning kind, or None when the line is innocent.
///
/// All rules are compiled into a single regex set, so one scan of the line
/// reports every rule that matches; the lowest index keeps first-hit-wins
/// priority without running each pattern separately.
pub fn inspect(line: &str) -> Option<Kind> {
    compiled()
        .matches(line)
        .iter()
        .next()
        .map(|index| RULES[index].kind)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_rule_compiles_inside_the_set() {
        let set = compiled();
        assert_eq!(set.len(), RULES.len());
    }

    #[test]
    fn catches_errors() {
        assert_eq!(inspect("[error] connection refused"), Some(Kind::Error));
        assert_eq!(
            inspect("error: could not compile `flashlog`"),
            Some(Kind::Error)
        );
        assert_eq!(inspect("exit status 1"), Some(Kind::Error));
    }

    #[test]
    fn catches_fatal_over_error() {
        assert_eq!(
            inspect("thread 'main' panicked at src/main.rs:2:5:"),
            Some(Kind::Fatal)
        );
        assert_eq!(
            inspect("Segmentation fault (core dumped)"),
            Some(Kind::Fatal)
        );
    }

    #[test]
    fn catches_secrets() {
        assert_eq!(inspect("AWS_KEY=AKIAIOSFODNN7EXAMPLE"), Some(Kind::Secret));
        assert_eq!(
            inspect("token: sk-abcdefghijklmnopq123456"),
            Some(Kind::Secret)
        );
        assert_eq!(
            inspect("-----BEGIN RSA PRIVATE KEY-----"),
            Some(Kind::Secret)
        );
    }

    #[test]
    fn catches_stack_traces() {
        assert_eq!(inspect("    at foo (bar.js:10:15)"), Some(Kind::StackTrace));
        assert_eq!(
            inspect("  14: 0x55f2a1 - core::ptr::drop_in_place"),
            Some(Kind::StackTrace)
        );
        assert_eq!(inspect("src/main.rs:42:9"), Some(Kind::StackTrace));
    }

    #[test]
    fn does_not_treat_short_secret_labels_as_credentials() {
        assert_eq!(inspect("password: public"), None);
        assert_eq!(inspect("token: local"), None);
    }

    #[test]
    fn catches_more_token_families_and_fatal_events() {
        assert_eq!(
            inspect("GITHUB_TOKEN=gho_abcdefghijklmnopqrstuvwxyz123456"),
            Some(Kind::Secret)
        );
        assert_eq!(inspect("OOM-killed process 881"), Some(Kind::Fatal));
        assert_eq!(inspect("uncaught exception in worker"), Some(Kind::Fatal));
    }

    #[test]
    fn avoids_plain_words_that_look_like_errors() {
        assert_eq!(inspect("error: connection failed"), Some(Kind::Error));
        assert_eq!(inspect("errorless completion"), None);
        assert_eq!(inspect("warning: deprecated API"), Some(Kind::Error));
    }

    #[test]
    fn innocent_lines_pass() {
        assert_eq!(inspect("2026-10-18 09:12:03 all systems nominal"), None);
        assert_eq!(inspect("hello world"), None);
        assert_eq!(inspect(" temperament: cozy"), None);
        assert_eq!(inspect("WARNINGS are nice when they are painted"), None);
        assert_eq!(inspect("the team failed forward gracefully"), None);
    }

    #[test]
    fn catches_level_tokens_and_critical_events() {
        assert_eq!(
            inspect("2026-11-26 10:00:05 CRITICAL disk almost full"),
            Some(Kind::Fatal)
        );
        assert_eq!(inspect("panic: runtime error: index out of range"), Some(Kind::Fatal));
        assert_eq!(
            inspect("2026-11-26T23:00:00Z WARN cache miss rate high"),
            Some(Kind::Error)
        );
        assert_eq!(inspect("segfault in worker thread"), Some(Kind::Fatal));
    }

    #[test]
    fn catches_more_stack_trace_dialects() {
        assert_eq!(
            inspect(r#"  File "app.py", line 42, in main"#),
            Some(Kind::StackTrace)
        );
        assert_eq!(
            inspect("    at com.example.App.main(App.java:17)"),
            Some(Kind::StackTrace)
        );
        assert_eq!(
            inspect("Caused by: java.io.IOException: broken pipe"),
            Some(Kind::StackTrace)
        );
        assert_eq!(inspect("goroutine 34 [running]:"), Some(Kind::StackTrace));
        assert_eq!(
            inspect("  0: std::panicking::rust_panic_with_hook"),
            Some(Kind::StackTrace)
        );
        assert_eq!(inspect("Main.java:17"), Some(Kind::StackTrace));
    }

    #[test]
    fn catches_exception_class_lines() {
        assert_eq!(
            inspect("ValueError: invalid literal for int()"),
            Some(Kind::Error)
        );
        assert_eq!(inspect("NullPointerException"), Some(Kind::Error));
        assert_eq!(inspect("KeyboardInterrupt"), Some(Kind::Error));
    }

    #[test]
    fn catches_runtime_failures_and_http_5xx() {
        assert_eq!(
            inspect("dial tcp 127.0.0.1:5432: connect: connection refused"),
            Some(Kind::Error)
        );
        assert_eq!(
            inspect("mkdir /data: permission denied"),
            Some(Kind::Error)
        );
        assert_eq!(inspect("write failed: no space left on device"), Some(Kind::Error));
        assert_eq!(inspect("upstream returned 502 Bad Gateway"), Some(Kind::Error));
        assert_eq!(inspect("HTTP/1.1 503 Service Unavailable"), Some(Kind::Error));
        assert_eq!(inspect("request finished status=500"), Some(Kind::Error));
        assert_eq!(inspect("FAILED tests/test_login.py::test_admin"), Some(Kind::Error));
        assert_eq!(inspect("status=200 ok"), None);
    }
}
