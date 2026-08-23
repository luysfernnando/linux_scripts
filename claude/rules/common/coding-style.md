# Coding Style

KISS, DRY, YAGNI. Simplest solution that works; extract only real (not speculative) repetition; build only what's needed now.

## Decision ladder (ponytail)

Before writing new code, in order: necessity (needed at all?) → reuse (exists in codebase?) → stdlib → platform-native feature → already-installed dependency → one-liner → minimal implementation.

Never cut for brevity: input validation, error handling, secrets/auth checks, accessibility. [code-review.md](code-review.md) enforces these at review time.
