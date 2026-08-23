# Coding Style

KISS, DRY, YAGNI: simplest working solution; extract only real (not speculative) repetition; build only what's needed now.

Decision ladder before new code: necessity → reuse (codebase) → stdlib → platform-native → installed dependency → one-liner → minimal implementation.

Never cut for brevity: input validation, error handling, secrets/auth checks, accessibility — [code-review.md](code-review.md) enforces at review time.
