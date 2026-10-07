# Contributing

[日本語](CONTRIBUTING.md) · [README](README.en.md)

## Before contributing

Issues and pull requests in Japanese or English are welcome. The detection/TTL core and macOS observation-only CLI are implemented; replacement and automatic startup are not.
For behavior changes, explain the use case, the gap in existing tools, and failure impact first.
Do not expand into a GUI, automatic startup, new platforms, or additional detection rules before establishing need.
Keep discussion focused on behavior and evidence; no personal attacks or harassment.

Follow [SECURITY.en.md](SECURITY.en.md) for security concerns, withholding public vulnerability details.
Do not submit real secrets, personal clipboard text, or unprocessed logs.

## Environment and checks

mise manages Rust 1.98.1. Review the configuration before running:

```sh
mise trust
mise install
mise run toolchain
mise run check
mise run audit
```

`check` runs documentation, formatting, clippy, core and CLI tests, and build checks.
`audit` checks known dependency advisories using network access to RustSec; it is not proof of safety.
Run Cargo through `mise exec -- cargo ...` or mise tasks. Include Cargo.lock in commits.

## Changes

1. Read the [design](docs/DESIGN.en.md) and limit scope and verification to the change.
2. Preserve existing uncommitted work; avoid unrelated changes.
3. Verify meaningful boundaries, failures, and regressions without weakening safety requirements.
4. Update Japanese and English documentation together for safety and operational changes.
5. Describe the reason, validation results, unverified behavior, and compatibility impact in the PR.

Automated tests must not read or write the actual clipboard.
For manual OS validation, understand that clipboard content can change or be lost, and use synthetic dummy data only.
Do not provision real test credentials. Attach environment, inputs, and measurement method to performance claims.

## License

Contributions must be provided under the [MIT License](LICENSE).
Preserve required third-party attribution and do not include material you lack rights to redistribute.
