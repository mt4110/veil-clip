# veil-clip initial design

[日本語](DESIGN.md) · [README](../README.en.md)

Status: detection/TTL core and macOS observation-only CLI implemented; replacement and automatic startup not implemented, 2026-10-06. The first platform is macOS and the license is MIT. Limited macOS basic-operation checks are recorded below. Broader compatibility, performance, and product value remain unverified.

## Current implementation scope

`src/lib.rs` exposes the core in `src/detector.rs` and `src/engine.rs`.
Types holding pending content are private; input Debug output is redacted. Actions and events contain no target text.
The caller supplies `Instant` values; the engine rejects backwards time and invalid sequencing.
Supply the time after each read completes and detection becomes possible, rather than before starting the read.
The protocol is `observe`→`ReadAgain`→`confirm` with a fresh read→`report_write` if a write was requested.
Issuing a write request is not success. A reported failure stops the engine and returns `ExitFailure`. Actual OS operations and process exit are the caller's responsibility. The current CLI fixes `Mode::Observe`, uses a read-only backend, and stops without executing unexpected write requests.

Automated verification covers startup/polling input, deadline boundaries, content changes, failed final checks, UTF-8 size limits, stopping, and mock writes.
Mocks verify argument rejection, bilingual messages, log suppression, read recovery, scheduling, stop flags, and wait wakeup. Process tests for help, version, and invalid arguments exit before backend access.
Synthetic-data validation on macOS confirmed actual reads, expiry notifications, cancellation, and Ctrl+C-driven SIGINT shutdown. Resume, real-device access failures/races, and performance remain unverified. The lock component is implemented and verified, but not connected to the observation CLI or future apply mode.

Local verification on 2026-10-06: macOS arm64, Rust 1.98.1; formatting, clippy with warnings denied, 54 core, CLI, and lock tests (including example reruns and the worker helper test), build, and documentation checks passed.
cargo-audit 0.22.2 also passed against Cargo.lock, reporting no known vulnerabilities or warnings in the RustSec database used at that time.
CI YAML syntax was checked, but remote CI has not run. Automated tests do not access the real clipboard. Only the authorized manual validation below copied synthetic data through the OS and read it with the monitor.

### macOS observation CLI validation (2026-10-06)

Environment: macOS 27.0.1 (26A434), arm64, Rust 1.98.1, locked dependencies, debug binary built with `mise run build`, Japanese default. Two independent monitor processes ran under a PTY. This was authorized OS validation, separate from automated tests.

| Check | Result |
| --- | --- |
| Copy a synthetic AWS ID with pbcopy and start | Detection and expiry notifications; 5.004 seconds between received logs; full dummy text unchanged after expiry |
| In a separate process, copy synthetic ordinary text after detection | Content-change cancellation; no expiry notification through six seconds after detection; full ordinary text unchanged |
| Send a terminal Ctrl+C character to each PTY | Both processes emitted a stopped notification and exited with code 0 |
| Output from both processes | Contained neither the dummy ID nor the synthetic ordinary text |

No content was saved in logs; pbpaste output was used only for in-memory equality checks. The original clipboard was not saved/restored; the final clipboard contains synthetic ordinary text. Changes were made by the validation's pbcopy calls; the monitor has no write operation.
Timing came from the harness's monotonic log-reception clock. The 5.004-second observation is one trial, not copy-to-detection latency or a strict five-second/performance guarantee. No defect reproduced; no code fix or fix-driven regression run was needed.
Unverified: real-device English startup, non-text/multiple formats, resume, access failures, other-application races, long-running operation, CPU/RSS, and other operating systems. These are not included in this result.

## 1. Value and scope

Hypothesis: copied credentials from sources outside a password manager need a helper to reduce accidental retention.
The first deliverable is a local foreground resident CLI. Automatic login startup, a GUI, networking, clipboard history, application allowlists, and rule distribution are outside the initial scope.
This is not general DLP, protection against malicious processes, or complete secret erasure.

There is no evidence to justify interrupting the primary product. Keep this a small practical evaluation.
Stop without expansion if existing automatic clearing suffices, there is no meaningful gap in copy sources, or disruption exceeds the benefit.

## 2. Corrections to the sample

| Area | Problem | Proposed correction |
| --- | --- | --- |
| Five-second blocking sleep | Stops polling and overwrites later copies unconditionally | Keep polling with a monotonic deadline |
| Initial last_content | Ignores sensitive content already present at startup | Scan the first read |
| Case-insensitive AWS expression | Also matches lowercase examples | Uppercase patterns with ASCII boundary checks |
| Private-key expression | Misses RSA PRIVATE KEY and OPENSSH PRIVATE KEY; matches PRIVATE KEY | Explicit header list |
| Full last_content | Retains ordinary clipboard text | Retain only one pending target |
| Complete destruction message | Implies erasure of history and other copies | Report replacement of the current clipboard |
| Near-zero CPU claim | Unmeasured and input/backend dependent | Measure with stated conditions |
| Transparent WSL2 sharing assumption | Depends on the actual integration | Validate Windows native and WSL2 separately |

## 3. Interface and defaults

The observation CLI is available on macOS; basic real-device validation was performed within the scope recorded below. `--apply` is a future proposal and currently rejected before initialization:

```text
veil-clip                     # Observe; read without writing
veil-clip --apply             # Currently rejected; proposed future replacement
veil-clip --lang en           # English help and logs
veil-clip --help
veil-clip --version
```

- Initially fix polling at 500ms and TTL at 5 seconds; defer configuration files and arbitrary timing options.
- Start TTL at detection, not at copy time. Previous retention time is unknown.
- Do not accept secret diagnostic input through arguments, environment variables, or stdin.
- Proposed future replacement replaces the whole text with `[REDACTED BY VEIL-CLIP]`; no partial redaction or undo.
- At startup, display observation mode, scope, timing, and history/latency limits in the selected language.
- Log only event types and rule IDs. Observation can redetect unchanged content periodically; throttle using event type and time, without retaining text for suppression.
- An observation deadline event states that no replacement was made.
- Ctrl+C stops the process without a final write or restoration of previous content.
- Future apply mode must acquire an OS-released, per-user exclusive lock before clipboard access; refuse startup if acquisition fails. A PID file alone is insufficient.

The CLI writes to stderr and suppresses identical events for 30 seconds. Cancellation resets detection/expiry suppression for a new target. Read recovery produces a separate static notification and resets suppression for read-failure skipped/cancelled events. No text or hashes are retained as history.
`ContentNotAvailable` does not distinguish empty from non-text input, so both are treated as ordinary skipped input. Occupied, unsupported, and other failures use static categories only.
The Ctrl+C handler sets a stop flag and wakes the wait. Shutdown may be delayed until a blocked OS read completes.

## 4. State and timing

Use one thread for clipboard operations. No async runtime or thread per secret is needed.
Represent deadlines with `Instant`; after OS operations, sleep until the next poll or deadline.
OS calls may block, so neither interval is a strict latency guarantee.

```text
Idle + matching text -> Pending(target text, rule IDs, deadline)
Pending + same text before deadline -> preserve deadline
Pending + different text -> release old target; scan the new input
Pending + non-text / empty / read error / oversized input -> release; Idle
Pending + deadline -> read again; require full equality
  Different or failed read: no write; scan readable new input
  Same + observation: report no change; release target
  Same + apply: attempt set_text once; release target
  Write success: report current clipboard replacement; Idle
  Write failure: report failure; exit nonzero; do not retry stale content
```

A newly observed secret gets its own TTL. Repeated reads of identical text never extend the deadline.
Text-only polling cannot distinguish recopying identical content, A→B→A between polls, format-only changes, or all simultaneous representations.
Replacement can affect accompanying non-text formats.

`arboard` exposes separate reads and writes, not an atomic conditional replacement. A race remains between the last read and write. The instance lock only coordinates veil-clip processes, not other applications.
If preservation of every intervening copy is mandatory, reject this replacement approach and choose observation or redesign around platform-specific operations.

Initialization failure exits nonzero without content in the error output. Classify read errors: empty/non-text is ordinary input; access failures produce static category warnings.
Cancel the pending target on a failed read, throttle repeated warnings by time, and keep polling at 500ms. Recovery starts a new TTL from the new observation.
This avoids writing to content that cannot be confirmed, at the cost of possibly retaining the secret beyond its original TTL during failure and recovery.
After system sleep, read again before acting. No protection is promised while stopped or suspended.

## 5. Detection rules

These are format heuristics. No remote credential validation or key validity check.

| Rule ID | Initial scope | Limits |
| --- | --- | --- |
| aws-access-key-id | Uppercase AKIA or ASIA followed by 16 ASCII uppercase letters/digits | IDs only; no standalone secret access keys or session tokens |
| private-key-header | Exact headers below | Documentation can match; no validation of the key body |

Find AWS candidates using `(?:AKIA|ASIA)[A-Z0-9]{16}`. Perform boundary checks in Rust because `regex` does not support look-around: reject candidates adjacent to an ASCII alphanumeric character or underscore.
This conservatively avoids matching within larger identifiers; it is not a claim of complete format coverage. Continue searching after a rejected candidate.

Detect these storage-format headers, not algorithm names such as Ed25519:

```text
-----BEGIN PRIVATE KEY-----
-----BEGIN ENCRYPTED PRIVATE KEY-----
-----BEGIN RSA PRIVATE KEY-----
-----BEGIN EC PRIVATE KEY-----
-----BEGIN OPENSSH PRIVATE KEY-----
```

Generic passwords, arbitrary API tokens, JWTs, public keys, certificates, and unlisted key formats are outside scope.
Evaluate false-positive replacement and actual missing cases before expanding rules.

## 6. Data and resources

Reading text necessarily places a UTF-8 string in process memory. Discard ordinary text after that read.
Keep only one pending target, never pass it to logging or persistence, and avoid unnecessary clones. The target and a fresh read temporarily coexist for full equality checks.

Provisional scanning limit: 1MiB of UTF-8 bytes. This is an operational policy, not a measured performance threshold.
On overflow, cancel pending state, skip writes, and issue a content-free warning.
The check occurs after `get_text` allocates the string; it does not bound total process memory or defend against all oversized input.

Release pending text on deadline, change, failure, or shutdown; blocking OS calls may delay release.
Dropping a String does not guarantee zeroization. Swap, core dumps, and OS/library copies are outside control. If zeroization becomes a release requirement, review additional dependencies and ownership separately.

Logs contain only mode, rule IDs, state events, and static error categories. No text, excerpts, hashes, source application names, history, or raw Debug output.
No networking, telemetry, persistent data, or credential-validation API.

## 7. Modules, dependencies, and platforms

Implemented modules: `src/detector.rs` for rules, `src/engine.rs` for state transitions, and `src/lib.rs` for the public boundary. `src/cli.rs` handles arguments/messages, `src/monitor.rs` schedules reads and reports events, `src/clipboard.rs` is the macOS read-only backend, and `src/main.rs` handles startup/shutdown.
The engine takes time and read results as inputs and returns actions. Tests use mocks and controlled time, never the real clipboard.

- macOS-only dependencies: `arboard` 3.6.1 (`default-features = false`) and `ctrlc` 3.5.2. No image feature is used. The ctrlc termination feature is disabled; only Ctrl+C is handled.
- Adopted `regex` 1.13.1 with only `std` and `perf`; compile once when constructing the Detector.
- AWS detection uses regex plus ASCII boundary checks; key headers use literal searches.
- Cargo.toml and Cargo.lock exist. License declarations for 49 external resolved crates were checked. The lock also includes dependencies resolved for other platforms; this is not evidence of platform support.
- arboard declares MIT OR Apache-2.0; ctrlc declares MIT/Apache-2.0. Safe path operations use rustix 1.1.5 with only std/fs/process features; locking itself uses the standard File API. rustix declares Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT; the lockfile audit reported no warnings.
- Pin development Rust at 1.98.1 in mise and run future Cargo commands through mise. No MSRV declared yet.
- Include Cargo.lock in commits. mise tasks and CI configuration cover formatting, clippy, core and CLI tests, build, and dependency auditing.
- cargo-audit 0.22.2 is pinned as a development tool in mise; audits access the public RustSec database. Passing an audit alone does not establish safety.
- CI is configured for automated checks on Ubuntu 24.04 and macOS 15, with dependency auditing on Ubuntu. Remote execution is unverified; this does not imply Linux clipboard support.

| Environment | Initial status | Real-device validation needed |
| --- | --- | --- |
| macOS | First validation target | Copy changes, multiple formats, shutdown, resume, access failures |
| Windows native | Deferred; not supported yet | Occupied clipboard, concurrency, history, build/distribution |
| Linux X11 | Deferred; not supported yet | Selection ownership and exit behavior |
| Linux Wayland | Deferred; not supported yet | data-control support, features, compositor |
| WSL2 | Not supported yet | Actual Windows clipboard integration |

Library portability is not evidence of tested OS support. Prefer a Windows CI/toolchain for the initial MSVC build; do not assume cross-compilation is a shortcut.

## 8. Verification and stopping conditions

After implementation is authorized, proceed in order. Real clipboard access, replacement, automatic startup, and publication each remain within their authorized scope.

1. Implement detector/state logic and test without OS access.
2. Validate observation on macOS using synthetic dummy data only.
3. Validate dummy replacement only if the remaining race is accepted.
4. Evaluate practical need and disruption; decide whether to continue.
5. Prepare private reporting, documentation, CI, and OS evidence before a release decision.

| Required case | Expected outcome |
| --- | --- |
| Target, ordinary text, empty, image at startup | Only a target arms a deadline |
| Target A→ordinary B during TTL | Cancel A; never request a write for B |
| Target A→target C | Start a new TTL for C |
| Repeated identical reads | Keep the original deadline |
| Just before / exactly at deadline | No early write; final check only at or after deadline |
| Final read changed or failed | No write; release stale target |
| Observation deadline | Zero write calls |
| Write failure | No success log; nonzero exit; no retry |
| Restart, shutdown, resume | Fresh reads; no unconditional shutdown write |
| Boundaries, short/long IDs, headers, public key | Match only defined formats |
| Below / at / above size limit, Japanese text | Count bytes; no content in logs |
| Second apply instance | Lock failure before clipboard access |

Unit tests do not resolve TOCTOU, changes between polls, or history retention.
Performance evidence must include OS, Rust/dependency versions, input size, mode, duration, CPU/RSS, and detection-to-replacement latency. Do not claim near-zero overhead or improvement percentages without measurements.

Finish when safety checks and real-device evidence are complete and the tool has a reason to be used in actual copy/paste work.
Stop apply mode and investigate if ordinary copy loss is observed. If the remaining race is unacceptable, finish with observation or make an explicit platform-specific redesign decision; do not expand research automatically.

## 9. OSS and languages

Japanese is the default for README, design, SECURITY, and CONTRIBUTING; corresponding `.en.md` files provide English versions.
The changelog and issue/PR templates are bilingual, Japanese first. Use the official English license text.
Update both languages together for safety-related changes.

GitHub private vulnerability reporting was disabled when checked on 2026-10-06.
An enabled private reporting channel or a verified private contact is required before release. No setting was changed.

## 10. Implementation decisions reviewed (2026-10-06)

These are findings and implementation decisions. The lock component and probe example are implemented; zeroization and apply mode are not.

### Per-user exclusivity

Prefer `std::fs::File::try_lock` without an additional crate. It has been stable since Rust 1.89.0; an API-only compilation succeeded with pinned Rust 1.98.1. It supports nonblocking acquisition and release when all related handles close. Independent-process normal exit and SIGKILL tests confirmed reacquisition using the same file inode. [Rust File API](https://doc.rust-lang.org/std/fs/struct.File.html#method.try_lock)

| Approach | Decision |
| --- | --- |
| Standard File exclusive lock | First choice; retain the owned File for the process lifetime |
| fd-lock 4.0.4 | Candidate if a RAII guard or future MSRV requires an alternative |
| fs2 0.4.3 | Candidate, deferred because required functionality overlaps the standard API |
| POSIX named semaphore | Do not adopt; releasing a reference is not restoring an acquired count |

fd-lock and fs2 coordinate cooperating processes; they do not protect against malicious processes running as the same user. [fd-lock](https://docs.rs/fd-lock/4.0.4/fd_lock/), [fs2](https://docs.rs/fs2/latest/fs2/trait.FileExt.html)
Named semaphores use `sem_wait` to acquire and `sem_post` to restore availability; `sem_close` releases a reference. From this distinction, we do not assume automatic count restoration after a crash. [sem_wait](https://developer.apple.com/library/archive/documentation/System/Conceptual/ManPages_iPhoneOS/man2/sem_wait.2.html), [sem_post](https://developer.apple.com/library/archive/documentation/System/Conceptual/ManPages_iPhoneOS/man2/sem_post.2.html), [sem_close](https://developer.apple.com/library/archive/documentation/System/Conceptual/ManPages_iPhoneOS/man2/sem_close.2.html)

Acceptance conditions for future apply-mode locking:

- Use a shared per-user path independent of version, working directory, and PID. The implementation uses `veil-clip/apply.lock` beneath the OS-managed path from `/usr/bin/getconf DARWIN_USER_DIR`. Run getconf with its environment cleared; do not depend on HOME, TMPDIR, working directory, version, or PID. Reject root and mismatched real/effective UIDs.
- Default to directory mode 0700 and regular-file mode 0600; verify owner, type, and permissions. Resolve invalid existing paths, symlinks, and substitution races; refuse startup if validation is inconclusive. Do not silently fall back to a network filesystem.
- Open without truncation and acquire before clipboard initialization. Distinguish contention from other failures with static errors; exit nonzero without waiting.
- Hold the handle until monitoring ends; avoid cloning or inheritance by children. Store no text, hashes, or PID. Leave the file in place on exit: deleting/recreating it can permit separate files to be locked simultaneously. File existence is not evidence of a stale lock.
- Test independent concurrent processes, normal exit, abnormal/forced exit followed by reacquisition, existing files, invalid permissions/links, and zero clipboard calls on acquisition failure.

The observation-only CLI does not currently need single-instance enforcement. This lock coordinates veil-clip instances; it does not remove races with other applications copying text.

Implementation: `src/instance_lock.rs`. OS-managed ancestors may be 0755, but every opened ancestor must belong to root or the current user and must not be group/other writable. The final parent must belong to the current user. Require 0700 for the dedicated directory and 0600, regular-file type, and link count 1 for the lock file. Refuse invalid permissions rather than repairing them automatically.
Normalize only the known macOS `/var` alias to `/private/var`; open remaining components with openat/NOFOLLOW/DIRECTORY. Create and inspect relative to held directory handles; recheck directory-entry inode identity after acquisition. CLOEXEC prevents exec inheritance; NONBLOCK avoids blocking on nonregular files. The probe does not fork or expose owned handles.

`examples/lock-check.rs` has no clipboard code. Run `mise exec -- cargo run --locked --example lock-check` to acquire until Enter or EOF. A concurrent invocation exits with code 1 and a static error. The empty lock file remains after normal or forced exit.
`tests/instance_lock.rs` uses isolated fixtures under target and independent child processes. Checks cover contention, normal/SIGKILL release, inode retention, no truncation, and rejected symlinks/hard links/permissions/nonregular files. Ownership and inode mismatches are metadata-policy tests; execution under a different UID is unverified. Fixtures are retained under target, without deletion.
On 2026-10-06, macOS 27.0.1 / arm64, the actual user-path probe also passed. Nonexistent HOME/TMPDIR overrides and a different working directory still contended on the same lock; normal exit and SIGKILL allowed reacquisition. No clipboard operation occurred in this phase.
Limits: POSIX ownership/mode are checked; additional macOS ACLs and network-mount detection are not implemented. Target the OS-provided standard local directory without alternate-path fallback. No guarantee prevents administrators or malicious same-UID processes from replacing paths, or deletion after acquisition. Review real deployment ACLs and directory lifetime before apply-mode integration. The observation CLI does not currently acquire this lock.

### AWS boundaries and structured text

Keep the current definition: reject a candidate if either existing adjacent byte is an ASCII alphanumeric or underscore; accept every other boundary. This is an exclusion rule, not an enumerated allowlist.

| Example (synthetic ID) | Expected under the current definition |
| --- | --- |
| `AKIA0000000000000000` | Match at start/end of input |
| `{"access_key_id":"AKIA0000000000000000"}` | Match the quoted JSON value |
| `?access_key_id=AKIA0000000000000000&next=1` | Match between `=` and `&` |
| `/AKIA0000000000000000/`, or surrounded by whitespace, newlines, tabs, quotes, brackets, punctuation | Match |
| `prefixAKIA0000000000000000`, `AKIA0000000000000000_` | Reject as part of an ASCII identifier |
| `日本語AKIA0000000000000000日本語` | Match with non-ASCII boundaries; not Unicode identifier validation |

Search the copied text without parsing JSON or URLs. Do not decode percent-encoded or Unicode-escaped IDs. Examples and explanatory text can match. No partial JSON/URL redaction is planned; future apply mode replaces the entire text.
Synthetic tests now cover both AKIA and ASIA in JSON values and URL parameters, paths, and fragments; all 128 ASCII boundary characters; and non-ASCII boundaries such as Japanese, full-width characters, combining marks, and emoji. Each ASCII alphanumeric and underscore is rejected on either side and both sides; each other ASCII character is accepted on either side and both sides. Replacing each ID character individually with a JSON Unicode escape or URL percent encoding does not reconstruct a match. No detector code change was needed.

Not decoding does not mean skipping an entire input containing encoding. For example, `%20AKIA0000000000000000` is rejected because the preceding raw byte is `0`, while `AKIA0000000000000000%41` matches because the following raw byte is `%`. A JSON `\u0041` immediately after the ID is likewise treated as a raw backslash boundary. These raw-text decisions are tested; parsed or decoded values are not validated.

### Log suppression without retained text or hashes

The Engine already emits no repeated Detected event for identical reads while Pending. Rule IDs and time alone cannot distinguish different texts matching the same rule; target text remains necessary for cancellation and final equality checks. Logging retention and deadline-state retention have different responsibilities.

On observation expiry, release target text and return to Idle. A subsequent identical read can detect again. EventLogger retains only events and last output times, suppressing repeats for 30 seconds. Detected events also distinguish rule combinations. Cancellation resets detection/expiry suppression.

Keep this approach and promise only: no repeated detection events while Pending; time-based suppression after expiry. Do not promise exactly one notification per distinct text. Immediately after expiry, a different target matching the same rule can also have its notification suppressed if no cancellation was observed. Absence of a notification is not evidence of absence of a target.
If one-time notification becomes necessary, compare longer retention of the full text with platform-specific change generations. Do not identify content solely by rule ID or bypass the no-hash requirement.

### Conditions for memory zeroization

Keep the current private Target and redacted Debug. Do not introduce an ineffective `ClipboardSecret(String)` solely for a hypothetical requirement. Introduce the wrapper together with zeroization when erasure of application-owned buffers becomes a release requirement.
Candidates are a wrapper around `zeroize::Zeroizing<String>` for a minimal design, or `secrecy::SecretString` for explicit text access. zeroize 1.9.0 covers the String buffer capacity, but not old copies from reallocations. secrecy 0.10.3 provides explicit access and Debug-leak prevention. [zeroize](https://docs.rs/zeroize/1.9.0/zeroize/), [secrecy](https://docs.rs/secrecy/0.10.3/secrecy/)

Wrap `get_text` results immediately without cloning and audit ownership through ReadOutcome, Input, Target, and confirmation. Include ordinary, empty, oversized, cancelled, stopped, and error paths, plus temporary confirmation buffers. Redact Debug; normally expose no Clone, Display, serialization, or extraction of an owned String. Borrow for detection/comparison. Wrapping only Pending misses other read buffers.
No erasure guarantee covers abort/forced exit without Drop, OS/arboard copies, old reallocation buffers, swap/core dumps, or other applications. Tests must not read freed memory or claim OS-wide erasure. Recheck maintenance, licensing, advisories, and features before adopting dependencies.

## 11. Primary references reviewed

- [arboard 3.6.1 Clipboard API and platform differences](https://docs.rs/arboard/3.6.1/arboard/struct.Clipboard.html)
- [arboard 3.6.1 features](https://docs.rs/crate/arboard/3.6.1/features)
- [arboard 3.6.1 errors](https://docs.rs/arboard/3.6.1/arboard/enum.Error.html)
- [regex 1.13.1 API](https://docs.rs/regex/1.13.1/regex/struct.Regex.html)
- [AWS IAM identifier prefixes](https://docs.aws.amazon.com/IAM/latest/UserGuide/reference_identifiers.html)
- [mise Rust management](https://mise.jdx.dev/lang/rust.html)
- [mise TOML tasks](https://mise.jdx.dev/tasks/toml-tasks.html)

- [ctrlc 3.5.2 API](https://docs.rs/ctrlc/3.5.2/ctrlc/)
