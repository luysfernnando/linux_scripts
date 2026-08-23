# Development Workflow

Reuse check (skip for bug fixes/one-liners/config tweaks): new module/algorithm/integration → check codebase, then package registry (npm/PyPI/crates.io/...) + docs, before hand-rolling. Battle-tested dependency > new code for a solved problem.

Loop: reuse check → plan (phases, deps, risks) → code ([coding-style.md](coding-style.md)) → review ([code-review.md](code-review.md)) → commit.

Commit message:
```
<type>: <description>

<optional body>
```
Types: feat, fix, refactor, docs, test, chore, perf, ci

PR: diff full branch history (`git diff [base]...HEAD`, not just last commit) → summary + test-plan TODOs → push `-u` if new branch. Pre-req: CI green, no conflicts, branch current with target.
