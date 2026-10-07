# Archive decision

[日本語](ARCHIVE_DECISION.md)

Decision date: 2026-10-07. Status: development and release preparation frozen. At the user's direction, archive the GitHub repository as read-only. Preserve code, tests and investigation records.

## Reasons

The reported workflow was copying ordinary passwords from Stickies or locked Notes into a terminal or browser. Current detection covers AWS access-key-ID formats and explicitly listed private-key headers. It cannot reliably identify ordinary passwords, and the CLI only reports events; it does not erase contents or block pasting. Additional value for this workflow was not established.

The user also identified no other use case justifying further development and chose freezing instead of continued work. Existing implementation effort and general security demand are not sufficient reasons to continue.
Stored-data protection, clipboard retention and mispaste are distinct concerns. Existing password management/autofill options are candidates, but their adequacy across the user's entire environment was not demonstrated. See the [password workflow desk evaluation](PASSWORD_USE_CASE.en.md).

## Evidence and limits

- Preserve detection/TTL core, macOS observation CLI, standalone lock and tests. The lock is not integrated with the CLI.
- Records include limited synthetic device validation and PR #1 Ubuntu/macOS CI success and merge.
- Password workflow analysis is a desk evaluation based on user reports, source code and official documentation. No real secrets were obtained or incidents reproduced.
- Five working days of observation were not completed. Do not describe this as completed observation or proof that nobody needs the tool.
- Replacement, autostart, GUI, auditing and paste interception are not implemented. Separate clipboard reads/writes retain a race that can lose a new copy.

The conclusion is: “For the personal workflow examined, no need justifying further development/publication of the current approach was established, and no alternative use case was identified.”

## While frozen

Stop feature work, release preparation, additional five-day observation and daily progress. No schedule was configured. There are no supported releases or provided security updates. Do not treat the existing code as a protective product.
Plans and procedures remain historical records; pending tasks are not current execution instructions.
GitHub archival stops repository editing; it does not stop/delete local files or manually started processes.

## Reopening conditions

1. Identify a concrete source, destination, problem and required behavior.
2. Explain verified shortcomings of existing measures in that situation, rather than relying on lack of use or impressions.
3. Define safety conditions and validation, including data minimization, false positives and preservation of other copies.
4. Agree on a small scope, stopping conditions and impact on the main product, then obtain user approval to resume.

Do not reopen automatically for an idea. Unarchive GitHub with explicit approval and recheck toolchain, dependencies, CI, device conditions and reporting channel. Historical passes do not establish future behavior or safety.
