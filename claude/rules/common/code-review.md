# Code Review

When: after writing/modifying code, before commit to shared branches, security-sensitive/architectural changes, before merging PRs.

Applies [coding-style.md](coding-style.md), plus:
- Readable, well-named; functions <50 lines, files <800 lines, nesting ≤4; no mutation
- No hardcoded secrets, unvalidated input, string-concat SQL, unsanitized output
- Auth/authz correct on changed endpoints; errors leak nothing sensitive
- No N+1 queries, missing pagination, unbounded queries, missing caching on hot paths

| Severity | Action |
|---|---|
| CRITICAL (security/data loss) | Block, must fix |
| HIGH (bug/quality) | Fix before merge |
| MEDIUM (maintainability) | Consider fixing |
| LOW (style) | Optional |

Approve = no CRITICAL/HIGH. Any CRITICAL blocks. Pre-req: CI green, no conflicts, branch current ([workflow.md](workflow.md)).
