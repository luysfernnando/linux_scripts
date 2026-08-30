# Guia de hooks (rule → hook)

Hooks = scripts que Claude Code roda em pontos fixos do ciclo de vida da sessão. Evento dispara, modelo não decide — por isso é a única camada de aplicação garantida (vs ~70% de uma instrução em texto).

## Eventos mais usados (auditoria)

- **PreToolUse** — antes da tool rodar. Bloqueia ação (`Repo` direto, `rm -rf`, escrita fora do repo). Bloqueio: exit code 2.
- **PostToolUse** — depois da tool rodar com sucesso. Automação determinística (lint/format pós-edit, teste pós-write).
- **SessionStart** — sessão inicia/retoma. Injeta contexto fixo sem depender do CLAUDE.md.
- **SubagentStop** — subagente termina. Valida resultado (rubric de qualidade) antes de aceitar de volta na conversa principal.
- **PreCompact** — antes de compactar contexto. Preserva info crítica antes do resumo.

## Lista completa de eventos

| Evento | Dispara quando |
|---|---|
| SessionStart | sessão inicia ou retoma |
| Setup | `claude --init-only`, ou `--init`/`--maintenance` em `-p` |
| UserPromptSubmit | prompt enviado, antes do Claude processar |
| UserPromptExpansion | comando digitado expande em prompt, antes de chegar ao Claude |
| PreToolUse | antes de tool call executar |
| PermissionRequest | tool call precisa de decisão de permissão |
| PermissionDenied | modo auto nega tool call |
| PostToolUse | tool call termina com sucesso |
| PostToolUseFailure | tool call falha |
| PostToolBatch | batch de tool calls paralelos resolve, antes da próxima chamada ao modelo |
| Notification | Claude Code envia notificação |
| MessageDisplay | texto da mensagem do assistant é exibido |
| SubagentStart | subagente é criado |
| SubagentStop | subagente termina |
| TaskCreated | task criada via `TaskCreate` |
| TaskCompleted | task marcada como concluída |
| Stop | Claude termina de responder |
| StopFailure | turno termina por erro de API |
| TeammateIdle | teammate de agent team prestes a ficar idle |
| InstructionsLoaded | CLAUDE.md ou `.claude/rules/*.md` carregado no contexto |
| ConfigChange | arquivo de config muda durante a sessão |
| CwdChanged | diretório de trabalho muda |
| DirectoryAdded | diretório adicionado mid-session (`/add-dir` ou SDK control request) |
| FileChanged | arquivo observado muda em disco |
| WorktreeCreate | worktree criado (`--worktree`, `isolation: "worktree"`, ou sessão background) |
| WorktreeRemove | worktree removido (fim de sessão, subagente termina, ou delete de sessão background) |
| PreCompact | antes de compactação de contexto |
| PostCompact | depois de compactação completar |
| Elicitation | MCP server pede input do usuário durante tool call |
| ElicitationResult | usuário responde elicitation MCP, antes de devolver ao server |
| SessionEnd | sessão termina |

Fonte: `docs.claude.com/en/docs/claude-code/hooks` (redireciona pra `code.claude.com/docs/en/hooks`).

## Formato

Payload JSON via stdin (tool, args, etc). Hook pode: devolver JSON no stdout (dado pro contexto), ou sair com exit code 2 (bloqueia). Formato antigo `decision`/`reason` está deprecado — sempre exit code.

## Exemplo ("nunca `Repo` direto, sempre Ash actions")

Hook PreToolUse inspeciona write/edit, procura `Repo.insert`/`Repo.update`/`Repo.delete` fora de contexto Ash aprovado, bloqueia (exit 2) com mensagem explicando o motivo. Config em `settings.json`/`.claude/settings.json`, apontando script + evento.

Teste sempre com 1 caso que deve bloquear e 1 que deve passar antes de considerar a migração concluída.
