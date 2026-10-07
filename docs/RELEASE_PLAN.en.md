# macOS observation CLI release-readiness plan

**2026-10-07: Development and release preparation are frozen. The following preserves implementation/investigation records, not an active work plan.** [Reasons and reopening conditions](ARCHIVE_DECISION.en.md)

[日本語](RELEASE_PLAN.md)

Updated: 2026-10-07. Approval for this task covers planning M0–M2. Writing this plan does not authorize feature implementation, device operations, distribution, or scheduled execution.

Latest M0 decision: the [Stickies/Notes password desk evaluation](PASSWORD_USE_CASE.en.md) fails the current detection/notification fit. Pause M1/M2 for this workflow. The following is a conditional plan, not a commitment to proceed.

## Intended completion

The first candidate is a manually started macOS CLI that detects potentially sensitive text and reports it, with Japanese by default and English support.
Notifications currently go to terminal stderr, with no OS or ChatGPT notification. No content, excerpts, or hashes are stored or transmitted. Replacement, paste blocking, history clearing, autostart, and GUI are outside these completion criteria.

General security demand does not establish demand or revenue for this tool. There is no established reason to displace the main product. macOS developers are the initial audience hypothesis; M0 must identify a concrete situation. Wider distribution follows evidence that an initial user can use it.

## M0: Establish audience and value

Status: incomplete. Plan prepared; use case and notification value unverified.

- The user identifies one real copying situation: source/destination categories, difficulty, and current measures only.
- Check applicable existing measures and separate verified capabilities, limitations, and unknowns.
- Use the [five-working-day template](USE_CASE_VALIDATION.en.md) during ordinary work, without creating incidents or collecting contents/screens.
- Determine whether a terminal notification leads to a useful action, without implying erasure or prevention of disclosure.

Completion: describe the situation, remaining gap, action after notification, and reason to use notification alone. Record count alone is insufficient.

Stop if checked alternatives suffice, notification serves no useful role, or automatic erasure/universal paste blocking is required. With insufficient observation, remain unverified and pause; do not extend automatically. Releasing a CLI does not prove commercial value.

## M1: Validate observation CLI quality

Status: partially complete. Existing detection/TTL/CLI/standalone lock and basic checks are complete; release-oriented device checks remain.

| Task | Method and owner | Completion |
| --- | --- | --- |
| Read-only execution path, secret-free output, dependencies/licenses | Agent inspects code and runs appropriate mise checks | CLI cannot execute writes; record output/failure limits |
| Japanese/English detection, expiry, cancellation, shutdown | User or explicitly authorized device check with synthetic data | Record environment/results and verify the tool does not modify the clipboard |
| Resume and non-text input | User performs controlled device procedure | No misleading protection claim for stale input; verify resumed reads and shutdown |
| Read failure/recovery and output failure | Agent checks mocks; mark safely unreproducible device failures unverified | Explain transitions; never silently treat failure as protection success |
| Work-session resource and notification burden | Controlled synthetic session measuring CPU/memory/notifications | Record environment, input size, duration, method; assess unusable burden or continuing growth |

Set numerical acceptance limits before measurement based on the intended use; make no unmeasured performance promises. Fix only reproduced defects within their cause. Do not weaken safety checks to pass tests.

Blockers: exposed contents, unintended writes, unusable notification/shutdown, unresolved resource growth, or implementation/documentation mismatch. Decide whether blocked OS reads delaying shutdown are acceptable for release; do not conceal the limit through indefinite research.

## M2: Prepare delivery to the first user

Status: not started. Decide distribution after M0 value and M1 quality gates pass.

- Align Japanese/English purpose, rules, notification meaning, start/stop/update/removal instructions.
- Prepare a short synthetic-data first-use procedure and verify an initial user can follow it.
- Choose one delivery route: source installation or a macOS binary, after checking burden and necessary signing/distribution requirements. Do not imply existing signing/notarization.
- State release version, supported macOS/CPU configurations, dependency licenses and update method. Ubuntu core CI success is not Linux monitoring support.
- Verify/prepare a private security reporting channel and align SECURITY in both languages. GitHub settings changes require specific approval.
- Prepare reviewable candidate artifacts, validation, and limitations. User approval for tags, release and external distribution is the final step.

Completion: an initial user can install, test synthetic data, stop, understand notification-only limits, and report problems privately; finish pre-release review. This does not establish nationwide compatibility or protective efficacy.

Stop distribution if required access/installation effort outweighs value, maintenance/report response is unsustainable, or serious quality issues remain.

## Daily progress

Choose one pending task per session and state purpose, artifact, and check method first. Finish with changes, validation, unresolved issues, and one next task. Bound work by task size; do not promise unmeasured time or cost.

| Order | Small deliverable | Prerequisite |
| --- | --- | --- |
| 1 (this task) | Bilingual plan, stop conditions and mapping to existing records | Complete; not product completion |
| 2 | One M0 use case and alternative comparison | User supplies situation |
| 3 | One M1 synthetic device procedure | M0 notification value established; specific approval before device operations |
| 4 onward | One device-result review or reproduced-defect fix | Necessary quality tasks only |
| After M1 | One M2 installation/distribution preparation task at a time | Final approval before publication |

No daily schedule is configured. The user chooses manual requests or scheduled execution. Scheduling requires a Japan-time execution time, explicit model, scope, and stop conditions. With missing input or a pending decision, record the wait rather than implement. Do not automatically perform new device operations, permission changes, publication, scope expansion, or model-setting changes.

## Early feasibility decisions

- Technical: observation CLI exists; safe general-purpose automatic replacement and universal paste blocking are not established outcomes.
- Value: M0 decides whether terminal notification suffices. If protection is required, do not rename notification as protection; stop this plan.
- Maintenance: focus on one small audience and installation path rather than multiple OSes, GUI and auditing simultaneously.
- Decisions: report blockers as soon as found, with location, evidence, impact, and an alternative or stop recommendation. Do not defer judgment by promising completion.
