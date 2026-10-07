# Milestones and tasks

**2026-10-07: Development and release preparation are frozen. The following preserves implementation/investigation records, not an active work plan.** [Reasons and reopening conditions](ARCHIVE_DECISION.en.md)

[日本語](MILESTONES.md)

Updated: 2026-10-07. The new M0–M2 below describe release-readiness phases. Earlier M1–M10 remain as implementation history; reused numbers do not establish new-phase completion.

| Release-readiness phase | Status | Next task |
| --- | --- | --- |
| M0 Audience and value | Fails for the reported password workflow | [Desk evaluation](PASSWORD_USE_CASE.en.md): current detection/notification does not fit. Pause M1/M2; clarify remaining gap |
| M1 Observation CLI quality | Partially complete | Prepare necessary bilingual/resume/non-text/resource device procedures |
| M2 Initial-user delivery preparation | Not started | After M0/M1: installation/distribution route, private reporting, pre-release review |

See the [release plan](RELEASE_PLAN.en.md) for completion, stop conditions and daily progress. Planning is complete; the M0 value decision is not.
PR #1 merged on 2026-10-07; its Ubuntu/macOS CI success was verified. Local tests, limited device records and remote CI are distinct evidence.

## Earlier implementation milestones (historical)

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
| M10 Public release | On hold | PR #1 is merged. At release, remote CI, a security reporting channel and necessary device validation require separate checks. Merge, tags and release require a separate decision. |
