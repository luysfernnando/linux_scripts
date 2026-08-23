# Code Review

## When

After writing/modifying code, before commit to shared branches, on security-sensitive or architectural changes, before merging PRs.

## Checks

Applies [coding-style.md](coding-style.md) principles, plus:

- Readable, well-named; functions <50 lines, files <800 lines, nesting ≤4 levels; no mutation
- No hardcoded secrets, unvalidated input, string-concat SQL, or unsanitized output
- Auth/authz correct on any changed endpoint; errors leak nothing sensitive
- No N+1 queries, missing pagination, unbounded queries, or missing caching on hot paths

## Severity → action

| Level | Action |
|---|---|
| CRITICAL (security/data loss) | Block, must fix |
| HIGH (bug/quality) | Should fix before merge |
| MEDIUM (maintainability) | Consider fixing |
| LOW (style) | Optional |

Approve = no CRITICAL/HIGH. HIGH-only = merge with caution. Any CRITICAL = block.

Before requesting review: CI green, no merge conflicts, branch up to date ([workflow.md](workflow.md)).
