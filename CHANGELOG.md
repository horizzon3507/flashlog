# Changelog

We follow [Semantic Versioning](https://semver.org/) and [Keep a Changelog](https://keepachangelog.com/). flashlog is a single CLI surface.

<details>
<summary>To see more about versioning, expand this.</summary>

Every version string starts with `v` (required), e.g. `v0.2.0-stable`, `v0.1.0-stable`.

The installable surface is **CLI** (`flashlog`). Other apps swap in their own names the same way — e.g. **GNOME**, **Desktop**, **Web** — whatever they actually ship.

| Part | What you install | Example |
| --- | --- | --- |
| **CLI** | `flashlog` in the terminal | `v0.1.2-stable` |

With one surface there is no `m` in the tag and no per-surface sections — just the version notes.

Each release heading is the version and date (`## v0.2.0-stable · 26/11/2026

Much faster scanning, broader error detection, and a smaller binary. This version was made for CLI with a stable release channel on 26/11/2026 (v0.2.0-stable).

- Compiled every detection rule into one `regex::RegexSet`, so each line is scanned once instead of once per rule. On a 500,000-line mixed log (33 MB) the same modes run in ~195 ms instead of ~335 ms on the maintainer's machine.
- Reworked `--only --context N` from "load the whole file, then filter" into a streaming sliding window: peak memory on a 330 MB log dropped from ~501 MB to ~9 MB, and input size no longer bounds memory.
- Switched line reading to buffer-borrowing scanning, removing one allocation per line, and write output directly without intermediate strings.
- Removed the runtime `serde`/`serde_json` dependency: JSON output is written directly with correct escaping. `serde_json` stays as a dev-dependency for tests.
- Trimmed release dependencies and profile (`regex` without its unused Unicode tables and engines, fat LTO, `opt-level = "z"`, one codegen unit, abort on panic, stripped symbols): the release binary went from 1,889,048 to 1,194,776 bytes (~37% smaller).
- Detection now also catches `CRITICAL` log records, `segfault`, Go `panic:` lines, Python `File "...", line N` frames and exception-class lines (`ValueError: ...`), Java `at com.example.Foo.bar(Foo.java:N)` frames, `Caused by:` chains, Go `goroutine N [...]:` dumps, Rust short backtrace frames (`  0: std::...`), uppercase `WARN`/`WARNING`/`FAILED` verdicts, `connection refused`, `permission denied`, `no space left on device`, `cannot allocate memory`, `too many open files`, HTTP 5xx statuses, and `.java`/`.rb`/`.cs` source locations.
- Lines with invalid UTF-8 are now inspected with lossy decoding instead of aborting the run.

## v0.1.2-stable · 04/11/2026`); under it, a short summary ends with a plain sentence like: “This version was made for CLI with a stable release channel on 04/11/2026 (v0.1.2-stable).”

### What the channel suffix means

| Suffix | In plain words |
| --- | --- |
| **-alpha** | Very early. Expect missing pieces and lots of bugs. |
| **-beta** | Mostly there, but still rough. Fine to try; not the “official” install. |
| **-stable** | Ready for daily use. This is what we put on GitHub Releases and the AUR. |

We only call something **stable** when we mean it.

</details>

## v0.1.2-stable · 04/11/2026

Relicensed the project under Apache-2.0. This version was made for CLI with a stable release channel on 04/11/2026 (v0.1.2-stable).

- Replaced the MIT license with Apache License 2.0 and updated the crate `license` field to `Apache-2.0`.

## v0.1.1-stable · 19/10/2026

Adds direct file inspection, output controls, and context around matches. This version was made for CLI with a stable release channel on 19/10/2026 (v0.1.1-stable).

- Read one or more files directly, or stdin with `-`; include file paths and line numbers in matching output.
- Emit valid JSON records with category, source, and line number fields.
- Control terminal colors with `--color auto|always|never`, respecting `NO_COLOR` in automatic mode.
- Filter categories with `--levels` and `--exclude`, include nearby lines with `--context`, and print per-category counts with `--summary`.
- Report invalid options and input errors with useful messages and appropriate exit statuses.
- Expand detection for common token families, warnings, and fatal memory failures.
- Add unit and CLI integration coverage for argument parsing, output, filtering, context, and file handling.

## v0.1.0-stable · 19/10/2026

Highlights errors, fatal failures, stack traces, and likely secrets in command output. This version was made for CLI with a stable release channel on 19/10/2026 (v0.1.0-stable).

- Streams standard input to standard output, passing ordinary lines through unchanged.
- Detects likely secrets, including AWS keys, common token formats, bearer credentials, plaintext passwords, and private key headers.
- Tags fatal failures, common errors, and stack trace lines with colored terminal output.
- `--only` filters output to suspicious lines; `--json` emits matches as JSON objects.
- Includes a release build profile with link-time optimization and binary stripping.
