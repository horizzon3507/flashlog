# 🔦 flashlog

Highlight log lines that matter — errors, leaked secrets, stack traces — right in the terminal.

```bash
journalctl -f | flashlog
cargo build 2>&1 | flashlog
tail -f /var/log/nginx/error.log | flashlog
```

flashlog reads text from standard input one line at a time. Ordinary lines pass through unchanged; suspicious lines receive a label and terminal highlighting. It is a heuristic filter: it helps surface likely problems and credentials, but it does not interpret or fix the program that produced them.

## What it detects

- **FATAL** — panics, segmentation faults, and core dumps
- **SECRET** — AWS keys, common `sk-…`, `ghp_…` and Slack tokens, bearer credentials, plaintext password fields, and private key headers
- **error** — common error words, non-zero exit status, and build failures
- **trace** — common stack trace frames and source file locations

## Options

- `--only` prints only suspicious lines.
- `--json` emits suspicious lines as JSON objects with `kind` and `line` fields. Ordinary lines are omitted.
- `-h` or `--help` prints usage.

For example, to inspect only matching lines from today's journal:

```bash
journalctl --since today | flashlog --only
```

To feed a command's standard output and standard error through flashlog:

```bash
cargo build 2>&1 | flashlog
```

## Install

Build the release binary with Rust and Cargo:

```bash
cargo install --path .
```

Or build it locally and put it in a directory on your `PATH`:

```bash
cargo build --release
install -Dm755 target/release/flashlog ~/.local/bin/flashlog
```

## Build from source

```bash
git clone https://github.com/horizzon3507/flashlog.git
cd flashlog
cargo build --release
cargo test
```

flashlog is written in Rust, has no runtime service, and uses one dependency (`regex`). All filtering happens locally; input is not sent over the network.
