# Claude Skills & Rules

Skills e rules Claude Code criadas por mim. Versionadas aqui, sincronizadas pra `~/.claude/`:

- Skills: `claude/skills/` → `~/.agents/skills/` (fonte compartilhada entre agentes) → symlink em `~/.claude/skills/` (onde Claude Code descobre).
- Rules: `claude/rules/` → rsync direto pra `~/.claude/rules/` (regras globais carregadas via `CLAUDE.md`/`RTK.md`).

## Setup

`bash claude/install.sh` — idempotente, roda de novo qualquer hora.

## Adicionar skill/rule nova

1. Skill: criar `claude/skills/<nome>/SKILL.md` (frontmatter `name`/`description`/`allowed-tools` + corpo).
2. Rule: criar/editar `claude/rules/common/<nome>.md` (geral) ou `claude/rules/<lang>/<nome>.md` (linguagem, ex: `rust/`).
3. `bash claude/install.sh`.

## Skills

| Skill | O que faz |
|---|---|
| `token-efficient-docs` | Otimiza CLAUDE.md/AGENTS.md/SKILL.md/docs pra IA — corta tokens, mantém substância técnica. |
| `rust-patterns` | Padrões idiomáticos Rust — ownership, error handling, traits, concorrência. |
| `rust-testing` | Padrões de teste Rust — unit/integration/async/property-based, TDD. |
| `project-context-optimizer` | Audita CLAUDE.md/rules/skills/hooks/agents e recomenda camada certa por custo de tokens. |
| `commit` | Commits limpos e separados por concern, mensagem via `/caveman:caveman-commit`, nunca trailer de IA. |

## Rules

| Categoria | Arquivos |
|---|---|
| `common/` | code-review, coding-style, workflow, delegation, context-economy, edit-markdown |
| `rust/` | coding-style, hooks, patterns, security, testing |
