# M0: Copying passwords from Stickies and locked Notes

**2026-10-07: Development and release preparation are frozen. The following preserves implementation/investigation records, not an active work plan.** [Reasons and reopening conditions](ARCHIVE_DECISION.en.md)

[日本語](PASSWORD_USE_CASE.md)

Assessed: 2026-10-07. A desk evaluation based on the user's reported workflow, current code, and official documentation. No app/clipboard operations, incident verification, or performance measurement occurred.

## Workflow and unknowns

Frequent workflows are “password in a sticky note → terminal/browser” and “copy from a locked note → terminal.” The user is concerned about disclosure through password managers.
No actual secrets were collected. Whether terminal input is a secret prompt or shell command line, whether the browser target is the legitimate login field, clipboard-history/sync settings, and the exact Notes authentication method are unknown. Biometric note unlocking is not assumed to be SSO.

## Synthetic desk simulation

Use `DUMMY-PASSWORD-ONLY-7!` as an ordinary-password dummy and `AKIA0000000000000000` as an existing-rule dummy. Neither is a provisioned credential.
This is an assumed timeline, not an OS measurement, with no ordinary-text replacement or automatic clearing.

| Time/action | Ordinary password dummy | AWS-shaped dummy |
| --- | --- | --- |
| t=0 copy | Does not match current AWS/private-key conditions | Matches AWS format |
| t=2 seconds paste into correct field | veil-clip does not observe success or destination | Same |
| Five seconds after detection | No deadline for this content | Reread and notify if unchanged; no clearing |
| t=30 seconds attempt paste elsewhere | Polling/terminal notifications cannot block it | Same, even after notification |
| Source sticky/locked note | veil-clip does not change the stored source | Same |

Predictions follow `src/detector.rs`, observation-only `src/monitor.rs`, and read-only `src/clipboard.rs`. Excluding the ordinary dummy does not mean all passwords are excluded: an incidental AWS-shaped substring could match.
The same string can serve as a password or ordinary text; content alone cannot reliably distinguish its purpose. Length/character heuristics retain false positives and negatives.

## Existing measures

- Locked Notes: Apple documents end-to-end encryption and password/biometric access. This describes stored notes, not protection of copied content in another app. [Apple Platform Security](https://support.apple.com/guide/security/secure-features-in-the-notes-app-sec1782bcab1/web)
- Apple Passwords: autofill for supported sites/apps can reduce manual copying. The source does not establish support for every terminal workflow or automatic clearing after copying. [Apple guidance](https://support.apple.com/en-us/120758)
- 1Password: Mac settings document removing copied information after 90 seconds. This is a copy route through 1Password, not universal clearing of copies from Stickies/Notes. It does not establish erasure of previously captured contents or all history. [Clipboard settings](https://support.1password.com/copy-passwords/)
- 1Password Universal Autofill: documented for Mac apps/browsers and sudo prompts in terminal apps; requires Accessibility access. Compatibility with this user's exact destinations remains unverified. [Autofill](https://support.1password.com/mac-universal-autofill/)
- 1Password storage: its design documents end-to-end encryption and a Secret Key combined with the account password. Vendor design documentation does not prove absence of incidents or disclosure. [Security model](https://support.1password.com/1password-security/)

Visible sticky notes, clipboard retention/mispaste, storage compromise, counterfeit input destinations, and endpoint compromise are distinct problems. Shell command-line input differs from a secret prompt; history retention depends on program/settings and was not inspected here.

## Decision and stopping conditions

**Additional value of current veil-clip for this workflow is not established. M0 fails for this workflow; do not proceed to M1/M2.** This rejects the fit of format detection/terminal notification, not the existence of the concern.
Do not rush publication by adding general-password heuristics. There is no established basis for nationwide distribution as a protective tool.

Next decision inputs are whether the primary fear concerns the storage provider, clipboard retention, or mispaste, and the exact category of terminal input. If a concrete reason prevents adoption of existing measures, evaluate that single situation as a redesign candidate. New features or automatic replacement require a separate decision.
An explicit user action identifying a sensitive copy could avoid format guessing, but remains a hypothesis with workflow effort and competing-copy races; it is neither established as a solution nor recommended for immediate implementation.

This desk evaluation does not replace five working days of actual observation. However, mismatch between workflow and detection scope is sufficient to pause release preparation without waiting for additional device tests.
