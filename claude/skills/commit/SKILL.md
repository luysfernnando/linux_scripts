---
name: commit
description: Stage and commit work in clean, logically separated commits with a caveman-terse Conventional Commits message and zero AI-attribution trailer. Use whenever the user asks to commit, "crie um commit", "commita isso", "separa em commits", or after finishing a chunk of work that's ready to save. Always delegates message wording to /caveman:caveman-commit.
allowed-tools: Bash, Read, Grep, Skill
---

# Commit

Regra fixa pra esse projeto (linux_scripts) e qualquer outro onde essa skill for instalada: nunca `Co-Authored-By: Claude` ou trailer de atribuição de IA — mesmo se `includeCoAuthoredBy` global não estiver desligado. Se aparecer, remover antes de criar o commit.

## Passo a passo

1. `git status` (nunca `-uall`) + `git diff` (staged e unstaged) — entender o que mudou. Se houver staged E unstaged não relacionado, tratar como concerns separados.
2. Rodar o quality gate do projeto se existir (`./check.sh`, `mix compile --warnings-as-errors && mix test`, `npm test`, lint configurado, etc — checar o que o projeto já usa antes de assumir). Se falhar, parar e reportar — não commitar quebrado.
3. Agrupar mudanças por concern (1 commit = 1 assunto). Nunca empacotar mudanças não relacionadas no mesmo commit.
4. Pra cada concern: `git add` só os arquivos daquele concern.
5. Gerar a mensagem chamando a skill `caveman:caveman-commit` (Conventional Commits: `<type>: <description>`, subject ≤50 chars, imperativo, sem ponto final, corpo só se o "porquê" não for óbvio) — nunca escrever a mensagem à mão, sempre delegar.
6. Commitar via heredoc, sem trailer de co-author, sem `--no-verify`/`--no-gpg-sign` a menos que pedido explicitamente.
7. Repetir 4-6 pra cada concern restante.
8. Mostrar `git log --oneline` do resultado. Não dar push a menos que pedido.

## Nunca

- Commitar com quality gate falhando.
- Amend em commit já existente sem pedido explícito (criar commit novo).
- Adicionar arquivo que pode ter secret sem antes checar o conteúdo.
- Push automático.
