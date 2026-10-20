use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};

fn run(input: &str, args: &[&str]) -> std::process::Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_flashlog"))
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(input.as_bytes())
        .unwrap();
    child.wait_with_output().unwrap()
}

#[test]
fn json_is_valid_and_escapes_control_characters() {
    let output = run("ok\nerror: quote \" and slash \\\n", &["--json"]);
    let text = String::from_utf8(output.stdout).unwrap();
    let value: serde_json::Value = serde_json::from_str(text.trim()).unwrap();
    assert_eq!(value["kind"], "error");
    assert_eq!(value["line"], "error: quote \" and slash \\");
}

#[test]
fn only_context_keeps_adjacent_lines() {
    let output = run(
        "before\nerror: failed\nafter\nlast\n",
        &["--only", "--context", "1"],
    );
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "before\n✗ error error: failed\nafter\n"
    );
}

#[test]
fn filters_and_summary_report_counts() {
    let output = run(
        "error: bad\nthread panicked\n",
        &["--levels", "fatal", "--summary"],
    );
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "error: bad\n✗ FATAL thread panicked\n"
    );
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("lines: 2 | matches: 2 (fatal 1"));
}

#[test]
fn reads_named_file_and_reports_location_in_json() {
    let path = std::env::temp_dir().join(format!("flashlog-test-{}.log", std::process::id()));
    fs::write(&path, "ok\nerror: disk failed\n").unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_flashlog"))
        .arg("--json")
        .arg(&path)
        .output()
        .unwrap();
    fs::remove_file(&path).unwrap();
    let text = String::from_utf8(output.stdout).unwrap();
    let value: serde_json::Value = serde_json::from_str(text.trim()).unwrap();
    assert_eq!(value["file"], path.display().to_string());
    assert_eq!(value["line_number"], 2);
}

#[test]
fn malformed_option_has_usage_error_and_exit_code_two() {
    let output = Command::new(env!("CARGO_BIN_EXE_flashlog"))
        .arg("--context")
        .arg("nope")
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8(output.stderr)
        .unwrap()
        .contains("invalid context count"));
}

#[test]
fn missing_file_is_reported_and_fails() {
    let output = Command::new(env!("CARGO_BIN_EXE_flashlog"))
        .arg("/definitely/missing/flashlog-input.log")
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8(output.stderr)
        .unwrap()
        .contains("flashlog-input.log"));
}

#[test]
fn no_color_prevents_ansi_sequences() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_flashlog"))
        .arg("--color")
        .arg("never")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .env("NO_COLOR", "1")
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(b"error: failed\n")
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "✗ error error: failed\n"
    );
}

#[test]
fn multiple_files_keep_separate_line_numbers_and_stdin_can_be_mixed() {
    let base = std::env::temp_dir().join(format!("flashlog-many-{}", std::process::id()));
    fs::create_dir_all(&base).unwrap();
    let first = base.join("first.log");
    let second = base.join("second.log");
    fs::write(&first, "error: first\n").unwrap();
    fs::write(&second, "ok\nerror: second\n").unwrap();
    let mut child = Command::new(env!("CARGO_BIN_EXE_flashlog"))
        .arg("--json")
        .arg(&first)
        .arg("-")
        .arg(&second)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(b"error: stdin\n")
        .unwrap();
    let output = child.wait_with_output().unwrap();
    fs::remove_file(first).unwrap();
    fs::remove_file(second).unwrap();
    fs::remove_dir(PathBuf::from(base)).unwrap();
    let values: Vec<serde_json::Value> = String::from_utf8(output.stdout)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert_eq!(values.len(), 3);
    assert_eq!(values[0]["line_number"], 1);
    assert!(values[1]["file"].is_null());
    assert_eq!(values[2]["line_number"], 2);
}

#[test]
fn color_auto_respects_no_color_for_terminal_independent_rendering() {
    let output = Command::new(env!("CARGO_BIN_EXE_flashlog"))
        .arg("--color")
        .arg("auto")
        .env("NO_COLOR", "1")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let mut child = output;
    child
        .stdin
        .take()
        .unwrap()
        .write_all(b"error: failed\n")
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(!String::from_utf8(output.stdout).unwrap().contains("\x1b"));
}
