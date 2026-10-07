# Milestones and tasks

[日本語](MILESTONES.md)

Reviewed: 2026-10-07. The current objective is to determine whether ordinary development work justifies further development. Feature development is paused.
“Complete” applies only to the stated scope; it does not establish overall product safety or usefulness.

| Milestone | Status | Evidence and remaining tasks |
| --- | --- | --- |
| M1 Detection and TTL core | Complete | `src/detector.rs`, `src/engine.rs`, synthetic tests for AWS boundaries, raw JSON/URL text without decoding, and cancellation on changes or failures. Core Apply is an action protocol with mocks only. |
| M2 macOS observation CLI | Complete | `src/main.rs`, `src/clipboard.rs`, `src/monitor.rs`: reads and notifications only. Rejects `--apply` before initialization. Bilingual help/logs and automated suppression/shutdown tests. |
| M3 Basic macOS device validation | Complete | Recorded on 2026-10-06: synthetic detection, five-second notification, cancellation on ordinary text, PTY Ctrl+C shutdown. Not rerun in this review. |
| M4 Standalone macOS lock | Complete | `src/instance_lock.rs`, `tests/instance_lock.rs`, `examples/lock-check.rs`: process contention, normal/SIGKILL release, permissions and link rejection. Actual user-path verification is recorded on 2026-10-06. |
| M5 OSS documentation and local checks | Complete | MIT, bilingual README/design/SECURITY/CONTRIBUTING, mise and CI configuration. `mise run check` and `mise run audit` passed on 2026-10-07. No public release. |
| M6 Five working days of use-case validation | Unverified | [Templates](USE_CASE_VALIDATION.en.md) are ready; observations have not been submitted. Whether existing measures suffice remains undecided. |
| M7 Broader device, compatibility and performance checks | Unverified | Resume, real read failures, multiple formats, long runs, CPU/memory, other OSes. Decide checks for the particular situation that justifies development. |
| M8 CLI lock integration and replacement | On hold | Not implemented. Locking cannot prevent another application's copy; separate reads/writes retain a race. ACLs, directory lifetime and a getconf timeout also require consideration before integration. |
| M9 Autostart, GUI, auditing and paste monitoring | On hold | Not implemented. No automatic expansion or pivot. |
| M10 Public release | On hold | The Draft PR is for review. Remote CI, a security reporting channel and necessary device validation require separate checks. Merge, tags and release require a separate decision. |

## Next tasks

1. The user records five actual working days in a private note without secret contents.
2. Use a sanitized summary to determine whether existing measures resolve the issue and whether a concrete problem is unacceptable for personal use.
3. End further development if no gap is established; otherwise discuss design and necessary tests for one situation only.

Limited observation opportunities do not prove lack of need. Do not automatically extend the period or implementation scope.
Automated tests do not access the clipboard, but lock tests use files and child processes. Lock fixtures remain under target.
Read remote CI results in the PR Checks. Local success does not imply remote success.
