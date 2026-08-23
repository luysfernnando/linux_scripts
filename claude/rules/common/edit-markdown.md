---
paths:
  - "**/*.md"
  - "!**/README.md"
---
# Edit Markdown

Writing/editing any `.md` file → run `token-efficient-docs` skill before finishing. It already sequences `caveman-compress` as its own final pass — don't call `caveman-compress` first.

`README.md` excluded (human audience, needs readable prose) — unless the user explicitly asks to compress that specific README.

Cost-tier check: `project-context-optimizer` skill → `references/decision-framework.md` — decide if the content should even live in a doc (could be a hook/CLAUDE.md/etc instead).