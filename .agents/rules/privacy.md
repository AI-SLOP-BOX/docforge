# Privacy & Secret Sanitization Rules

## Critical Constraints
- **NEVER** write, hardcode, or commit any personal names, developer usernames, macOS user home paths (`/Users/...`), Windows user paths (`C:\Users\...`), or local machine paths into code, comments, test fixtures, error messages, configurations, or git commits.
- All file and font lookups must be dynamic (e.g. embedded via `include_bytes!`, relative to `current_exe()`, or bundled in app resources), NEVER absolute user paths.
- The Git author for this repository is strictly "Nagisa PDF Team" `<team@nagisapdf.internal>`.
- Any git commit must be checked to ensure zero personal identifiable information (PII) is included in commit diffs or commit messages.
