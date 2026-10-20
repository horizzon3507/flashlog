# Changelog

We follow [Semantic Versioning](https://semver.org/) and [Keep a Changelog](https://keepachangelog.com/). flashlog is a single CLI surface.

<details>
<summary>To see more about versioning, expand this.</summary>

Every version string starts with `v` (required), e.g. `v0.2.0-stable`, `v0.1.0-stable`.

The installable surface is **CLI** (`flashlog`). Other apps swap in their own names the same way — e.g. **GNOME**, **Desktop**, **Web** — whatever they actually ship.

| Part | What you install | Example |
| --- | --- | --- |
| **CLI** | `flashlog` in the terminal | `v0.1.0-stable` |

With one surface there is no `m` in the tag and no per-surface sections — just the version notes.

Each release heading is the version and date (`## v0.1.0-stable · 19/10/2026`); under it, a short summary ends with a plain sentence like: “This version was made for CLI with a stable release channel on 19/10/2026 (v0.1.0-stable).”

### What the channel suffix means

| Suffix | In plain words |
| --- | --- |
| **-alpha** | Very early. Expect missing pieces and lots of bugs. |
| **-beta** | Mostly there, but still rough. Fine to try; not the “official” install. |
| **-stable** | Ready for daily use. This is what we put on GitHub Releases and the AUR. |

We only call something **stable** when we mean it.

</details>

## v0.1.0-stable · 19/10/2026

Highlights errors, fatal failures, stack traces, and likely secrets in command output. This version was made for CLI with a stable release channel on 19/10/2026 (v0.1.0-stable).

- Streams standard input to standard output, passing ordinary lines through unchanged.
- Detects likely secrets, including AWS keys, common token formats, bearer credentials, plaintext passwords, and private key headers.
- Tags fatal failures, common errors, and stack trace lines with colored terminal output.
- `--only` filters output to suspicious lines; `--json` emits matches as JSON objects.
- Includes a release build profile with link-time optimization and binary stripping.
