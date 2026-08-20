# Claude Skills & Rules

Skills e rules Claude Code criadas por mim. Versionadas aqui, sincronizadas pra `~/.claude/`:

- Skills: `claude/skills/` → `~/.agents/skills/` (fonte compartilhada entre agentes) → symlink em `~/.claude/skills/` (onde Claude Code descobre).
- Rules: `claude/rules/` → rsync direto pra `~/.claude/rules/` (regras globais carregadas via `CLAUDE.md`/`RTK.md`).

## Setup

```bash
bash claude/install.sh
```

Idempotente — roda de novo qualquer hora, sincroniza mudanças do repo.

## Adicionar skill nova

1. Criar `claude/skills/<nome>/SKILL.md` (frontmatter `name`/`description`/`allowed-tools` + corpo).
2. Rodar `bash claude/install.sh`.

## Adicionar/editar rule

1. Editar/criar `claude/rules/common/<nome>.md` (regra geral) ou `claude/rules/<lang>/<nome>.md` (regra específica de linguagem, ex: `rust/`).
2. Rodar `bash claude/install.sh`.

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
| `common/` | code-review, coding-style, development-workflow, git-workflow, hooks, patterns, performance, security, testing |
| `rust/` | coding-style, hooks, patterns, security, testing |
