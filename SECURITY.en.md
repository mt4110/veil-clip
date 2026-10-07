# Security policy

[日本語](SECURITY.md) · [README](README.en.md)

## Support status

The detection/TTL core and macOS observation-only CLI are implemented; there are no supported releases.
The CLI only reads and reports events, never replaces content, and rejects `--apply` before initialization.
The planned scope remains a helper for replacement of the current regular clipboard.
It does not guarantee erasure of history, synced data, copies already obtained by other applications, paste destinations, source files, swap, or core dumps.
Read/write races, false negatives, false positives, OS suspension, and delays remain possible.

## Sensitive data

The current core performs no OS operations, logging, persistence, or network transmission.
It temporarily holds supplied target text in memory while pending; action requests, events, and Debug output exclude the text. Complete memory erasure is not guaranteed.
The CLI temporarily reads text into memory and writes only static categories and states to stderr. Invalid argument values and raw OS error descriptions are not printed.
See the [design](docs/DESIGN.en.md). Synthetic-data validation on macOS confirmed reads, expiry notifications, cancellation on ordinary-text copies, and Ctrl+C shutdown. Resume, access failures, and races with other applications remain unverified on real devices. Neither limited real-device checks nor mocks establish OS-wide protection.

The lock component coordinates cooperating processes of the same user. Its probe creates an empty file in a user-specific directory, writing no text, hashes, or PID. POSIX ownership/mode and links are checked; additional ACLs and defense against malicious same-UID processes or administrators are not guaranteed. It is not connected to the observation CLI yet.

## Reporting vulnerabilities

**As of 2026-10-06, GitHub private vulnerability reporting is disabled, and no dedicated private channel has been configured.**

Never post real credentials, clipboard text, or exploitable vulnerability details in public issues or pull requests.
Until a private channel exists, open a public issue saying only that you need a private security contact; withhold the details.
Release preparation remains incomplete until the channel is established and this policy updated.

After the channel exists, reports should identify the version/commit, OS, reproduction using synthetic dummy data, and impact.
No response or remediation deadline is promised yet.

Clipboard replacement does not invalidate exposed credentials. Follow the issuer's revocation and rotation procedures.
