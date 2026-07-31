---
name: token-efficient-docs
description: Use when writing or reviewing CLAUDE.md, AGENTS.md, SKILL.md, docs/*.md, or any file loaded into an AI agent's context repeatedly (system prompt, project instructions, skill body). Cuts token cost of AI-facing documentation without losing technical substance. Trigger phrases: "otimizar CLAUDE.md", "documentação eficiente em tokens", "reduzir contexto", "token-efficient docs", "compress skill", "cut CLAUDE.md size".
allowed-tools: Read, Write, Edit, Glob, Grep
---

# Token-Efficient AI Docs

Doc read by AI, not human. No prose-for-readability, no line-wrap, no rationale unless it changes a decision. Every line costs tokens on every load — cut ruthlessly.

## Golden rules

1. **Table > prose.** Decision logic in a table beats narrative explanation — fewer tokens, faster parse, no ambiguity.
2. **Allowlist > blocklist.** State what's permitted ("Repo direto — sempre actions Ash") not long lists of forbidden actions — blocklists miss cases and cause retry loops when the AI hits an unlisted violation.
3. **1 concrete example > abstract explanation.** A before/after code snippet beats a paragraph of "you should write clear X". Two lines of example prevent hundreds of tokens in failed retries.
4. **Rule enforced by tooling → 1 line, not a paragraph.** If a pre-commit hook/CI already blocks it (`mix precommit`, lint, format), state "hook enforces X" — don't re-explain the rule in prose.
5. **Drop what the model already knows.** No explaining what LiveView/React/Ecto/a foreign key is. Keep only project-specific decisions: naming conventions, paths, gotchas, non-obvious architecture choices.
6. **No rationale/history unless it changes behavior.** "We removed X because Y" → useful only if it prevents someone reintroducing X. Otherwise cut — the AI doesn't need convincing, just the current rule.
7. **Merge wrapped lines into one.** Human-readable soft-wrap (breaking a sentence at ~80 chars) adds newline tokens for zero benefit to an AI reader — write each fact as one line/bullet, however long.
8. **Size target:** project instruction file (CLAUDE.md-equivalent) → 50–100 lines is the sweet spot. Past ~200 lines, split into topic files (`docs/domain/*.md`) and link them from an index table — don't let one file grow unbounded.
9. **Progressive disclosure for skills:** SKILL.md `description` frontmatter loads always (cheap, ~30-150 tokens) — full body loads only when triggered. Keep description precise/keyword-dense (drives correct triggering), push bulk detail into the body or linked files.
10. **One canonical example > many similar ones.** If 3 bullets show the same pattern with different names, keep 1 and say "same pattern for X, Y, Z" — don't repeat structure.

## Process for compressing an existing doc

1. Read full file.
2. Mark every paragraph: **rule** (keep), **rationale/history** (cut unless load-bearing), **already-known-by-model** (cut), **duplicate of another section** (merge).
3. Convert prose rules → table rows or single-line bullets where possible.
4. Join soft-wrapped multi-line sentences into one line.
5. Move enforced-by-hook rules to one-liners.
6. If file > 200 lines after this pass, split by topic, leave an index.
7. Re-read once — verify no code block, path, command, or exact error string was altered (those must survive compression byte-for-byte).
8. Run structural pass (steps 1-7) by hand — it needs judgment (what's rationale vs rule, what to merge). Once structure is settled, run `caveman:caveman-compress` (`/caveman-compress <filepath>`) as final filler pass — strips articles/hedging/pleasantries the structural pass leaves behind. Don't run it first: it only removes filler words, it doesn't cut rationale, merge duplicates, or convert prose to tables — structural cuts save far more tokens than filler removal.

## Never cut

- Code blocks, file paths, commands, exact error strings, config keys, version numbers — copy verbatim.
- Any rule the codebase's tooling can't enforce automatically (hooks/lint/CI catch syntax, not domain logic like "créditos não bloqueiam criação de pleito").
- Non-obvious gotchas that caused a real bug ("`@form` nil no 1º render crasha — sempre placeholder síncrono").

## Sources

- [CLAUDE.md Token Optimization Cheat Sheet](https://gist.github.com/yurukusa/556f67c493a2729ce9b1703f5003a227)
- [claude-meta/token-efficiency SKILL.md](https://github.com/Delphine-L/claude_global/blob/main/skills/claude-meta/token-efficiency/SKILL.md)
- [Claude Code Token Optimization: Full System Guide](https://buildtolaunch.substack.com/p/claude-code-token-optimization)
- [12 Ways to Cut Token Consumption in Claude Code](https://www.firecrawl.dev/blog/claude-code-token-efficiency)
