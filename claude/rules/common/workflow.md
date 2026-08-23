# Development Workflow

## Reuse check (before new non-trivial code)

Skip for bug fixes, one-line edits, or config tweaks. Applies when building a new module, algorithm, or integration: check codebase for an existing implementation first, then the language's package registry (npm/PyPI/crates.io/...) and library docs, before hand-rolling. Prefer a battle-tested dependency over new code that solves an already-solved problem.

## Loop

Reuse check → plan (phases, deps, risks) → code ([coding-style.md](coding-style.md)) → review ([code-review.md](code-review.md)) → commit.

## Commit message

```
<type>: <description>

<optional body>
```
Types: feat, fix, refactor, docs, test, chore, perf, ci

## PR

Diff full branch history (`git diff [base]...HEAD`, not just last commit) → comprehensive summary + test-plan TODOs → push `-u` if new branch. Pre-req: CI green, no conflicts, branch current with target.
