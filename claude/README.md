# Claude Skills

Skills do Claude Code criadas por mim, versionadas aqui e replicadas pra `~/.agents/skills/` (fonte compartilhada entre agentes) com symlink em `~/.claude/skills/` (onde o Claude Code descobre skills).

## Setup

```bash
bash claude/install.sh
```

Idempotente — roda de novo a qualquer momento pra sincronizar mudanças feitas no repo.

## Adicionar skill nova

1. Criar `claude/skills/<nome>/SKILL.md` (frontmatter `name`/`description`/`allowed-tools` + corpo).
2. Rodar `bash claude/install.sh`.

## Skills

| Skill | O que faz |
|---|---|
| `token-efficient-docs` | Otimiza CLAUDE.md/AGENTS.md/SKILL.md/docs pra consumo por IA — corta tokens sem perder substância técnica. |
