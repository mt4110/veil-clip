# veil-clip

**2026-10-07: Development and release preparation are frozen. The following preserves implementation/investigation records, not an active work plan.** [Reasons and reopening conditions](docs/ARCHIVE_DECISION.en.md)

[日本語](README.md)

A local macOS CLI that detects potentially sensitive clipboard text and reports it in the terminal.

**The macOS observation-only CLI and detection/TTL core are implemented. Replacement and automatic startup are not implemented.**
The [release plan](docs/RELEASE_PLAN.en.md) and [observation template](docs/USE_CASE_VALIDATION.en.md) remain historical records. Following the password desk evaluation, no alternative use case justified further development, so the project was frozen. Five working days of observation were not completed.
The core performs no OS operations. The CLI reads through `arboard` and reports detection and expiry. Synthetic-data validation on macOS confirmed reads, expiry notifications, cancellation, and Ctrl+C shutdown.

## Implemented core

- Detect AKIA/ASIA-shaped AWS IDs and five private-key headers.
- Cancel the previous deadline on content changes; give a new target its own five-second TTL.
- Request a fresh read at expiry, then request replacement only after full equality confirmation.
- Never request replacement in observation mode. Stop without retry after a reported write failure.
- Retain no ordinary text and at most one target. Skip inputs exceeding 1MiB of UTF-8 bytes.
- Keep text out of action requests and events; redact input Debug output.

Core action requests do not perform OS writes. Detection and transitions are tested with synthetic data and in-memory mocks. Limited macOS validation was also performed. Resume, real-device failure handling, performance, and product value remain unverified.

## Purpose

Reduce accidental retention of credentials copied during development.
If a password manager's automatic clipboard clearing already meets your needs, using that feature avoids maintaining another application.
This project will first evaluate whether other copy sources create a useful gap.

## Current CLI behavior

- macOS is the first platform for real-device validation. Windows, Linux, and WSL2 are unverified.
- A foreground CLI explicitly started by the user.
- Poll text every 500ms, including content present at startup.
- Detect AWS access key IDs and an explicit list of private-key headers.
- Observation only: recheck 5 seconds after detection and report that no change was made; keep polling during the delay.
- Reject `--apply` before initialization. Future replacement requires a separate decision.
- No content, excerpts, or content hashes in logs or on disk; no application network transmission.
- Japanese by default, with English help, logs, and documentation.

Logs go to stderr. Repeated events are suppressed for 30 seconds; cancellation or read recovery resets relevant suppression. Logs are not a complete copy history.
Ctrl+C stops the process and wakes its wait. Immediate termination during a blocked OS read is not guaranteed.

```sh
# Commands that do not access the clipboard
mise exec -- cargo run --locked -- --help
mise exec -- cargo run --locked -- --lang en --help
mise exec -- cargo run --locked -- --version

# Start monitoring on macOS: reads the current clipboard
mise exec -- cargo run --locked --
# Monitor with English logs
mise exec -- cargo run --locked -- --lang en
```

Basic monitoring was verified on macOS 27.0.1 / arm64 on 2026-10-06. See the [real-device validation record](docs/DESIGN.en.md#macos-observation-cli-validation-2026-10-06). Use synthetic dummy data for manual validation.

## Limits

The current CLI only reports events and does not erase secrets. Future replacement would affect the current regular clipboard only. It cannot erase history, synced copies, data already read by another application, paste destinations, or source files.
Detection has false positives and false negatives. Matching an AWS access key ID does not detect a standalone secret access key.

Reads and writes through `arboard` are separate operations. Another copy can occur between the final check and replacement. The design cannot guarantee preservation of every new copy, complete erasure, prevention of disclosure, or protection within 5 seconds.
See the [design](docs/DESIGN.en.md) and [security policy](SECURITY.en.md).

## Development

[mise](https://mise.jdx.dev/) manages the pinned development toolchain, Rust 1.98.1. This is not an MSRV declaration.

```sh
# Review the configuration before trusting it
mise trust
# Install missing development tools, if needed
mise install
mise run toolchain
mise run check
mise run audit
```

Installing tools contacts their distribution services. `check` runs documentation checks, formatting, clippy, core and CLI tests, and builds sequentially.
`audit` uses cargo-audit 0.22.2, pinned in mise, to check Cargo.lock against the public RustSec database. It needs network access; passing it does not guarantee safety.
Keep Cargo.lock under version control and run Cargo through mise. No MSRV is declared.

A [GitHub Actions workflow](.github/workflows/ci.yml) runs the same checks. Local results and remote CI results are separate; PR #1’s Ubuntu/macOS CI success and merge were verified on 2026-10-07.

## Documentation

The lock component is implemented and verified on macOS, but not connected to the observation CLI. Run `mise exec -- cargo run --locked --example lock-check` for a clipboard-free probe. It creates a user-private directory and empty lock file, holding the lock until Enter or EOF. The file remains after exit. See the [design](docs/DESIGN.en.md) for limits.

- [Milestones and tasks](docs/MILESTONES.en.md) / [日本語](docs/MILESTONES.md)
- [Design and acceptance criteria](docs/DESIGN.en.md) / [日本語](docs/DESIGN.md)
- [Security policy](SECURITY.en.md) / [日本語](SECURITY.md)
- [Contributing](CONTRIBUTING.en.md) / [日本語](CONTRIBUTING.md)
- [変更履歴 / Changelog](CHANGELOG.md)

## License

[MIT License](LICENSE). The English license text is authoritative.
