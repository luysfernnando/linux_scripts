# Claude Skills

Skills Claude Code criadas por mim. Versionadas aqui, replicadas pra `~/.agents/skills/` (fonte compartilhada entre agentes), symlink em `~/.claude/skills/` (onde Claude Code descobre skills).

## Setup

```bash
bash claude/install.sh
```

Idempotente — roda de novo qualquer hora, sincroniza mudanças do repo.

## Adicionar skill nova

1. Criar `claude/skills/<nome>/SKILL.md` (frontmatter `name`/`description`/`allowed-tools` + corpo).
2. Rodar `bash claude/install.sh`.

## Skills

| Skill | O que faz |
|---|---|
| `token-efficient-docs` | Otimiza CLAUDE.md/AGENTS.md/SKILL.md/docs pra IA — corta tokens, mantém substância técnica. |