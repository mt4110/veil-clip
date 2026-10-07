#!/bin/sh
set -eu

# 日本語・英語の必須文書と基本的な空白を検査する。
# Check required Japanese/English documents and basic whitespace.
for path in \
    README.md README.en.md \
    docs/DESIGN.md docs/DESIGN.en.md \
    docs/ARCHIVE_DECISION.md docs/ARCHIVE_DECISION.en.md \
    docs/MILESTONES.md docs/MILESTONES.en.md \
    docs/RELEASE_PLAN.md docs/RELEASE_PLAN.en.md \
    docs/PASSWORD_USE_CASE.md docs/PASSWORD_USE_CASE.en.md \
    docs/USE_CASE_VALIDATION.md docs/USE_CASE_VALIDATION.en.md \
    SECURITY.md SECURITY.en.md \
    CONTRIBUTING.md CONTRIBUTING.en.md \
    LICENSE CHANGELOG.md Cargo.toml Cargo.lock mise.toml .editorconfig .gitignore \
    scripts/check-docs.sh \
    .github/ISSUE_TEMPLATE/bug_report.md \
    .github/ISSUE_TEMPLATE/feature_request.md \
    .github/pull_request_template.md \
    .github/workflows/ci.yml
do
    if [ ! -s "$path" ]; then
        printf 'Missing or empty file: %s\n' "$path" >&2
        exit 1
    fi
    awk '
        /[ \t\r]+$/ {
            printf "%s:%d: trailing whitespace\n", FILENAME, FNR
            failed = 1
        }
        END { exit failed }
    ' "$path"
done

git diff --check
printf '%s\n' '文書チェック成功 / Documentation checks passed'
