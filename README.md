# 🔦 flashlog

Highlight log lines that matter — errors, leaked secrets, stack traces — right in the terminal.

```bash
journalctl -f | flashlog
cargo build 2>&1 | flashlog
tail -f /var/log/nginx/error.log | flashlog
```

Innocent lines pass through untouched. Suspicious lines get tagged and inverted:

- ✗ FATAL — panics, segfaults, core dumps
- ⛏ SECRET — AWS keys, `sk-…`/`ghp_…` tokens, bearer headers, passwords in plaintext
- ✗ error — level words, exit status, build failures
- ↳ trace — stack trace frames

## Options

```
--only    print only suspicious lines
--json    emit suspicious lines as JSON objects
```

Rust, no runtime, one dependency (`regex`). Nothing leaves your machine.

## Build

```bash
cargo build --release
```
