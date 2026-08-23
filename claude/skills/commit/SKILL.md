---
name: commit
description: Stage and commit in clean, per-concern commits — Conventional Commits message (language/format from project memory), zero AI-attribution trailer. Use whenever the user asks to commit, "crie um commit", "commita isso", "separa em commits", or after finishing a chunk of work that's ready to save.
allowed-tools: Bash, Read, Grep
---
# Commit

Nunca use `Co-Authored-By: Claude` ou trailer de atribuição de IA — mesmo se `includeCoAuthoredBy` global não desligado. Aparecer → remover antes de commit.

## Passo a passo

1. `git status` (nunca `-uall`) + `git diff` (staged e unstaged). Staged e unstaged não relacionados = concerns separados.
2. Quality gate do projeto se existir (`./check.sh`, `mix compile --warnings-as-errors && mix test`, `npm test`, lint configurado — checar o que projeto usa). Falhou → parar, reportar, não commitar.
3. Agrupar por concern: 1 commit = 1 assunto. Nunca misturar mudanças não relacionadas.
4. Por concern: `git add` só arquivos dele.
5. Memória `commit-convention` (idioma + padrão); ausente → derivar de `git log --oneline -20` e salvar. Mensagem: ≤50 chars (limite 72), sem ponto final, corpo só se "porquê" não óbvio pelo diff, nunca 1ª pessoa, nunca atribuição de IA.
6. Commit via heredoc: sem trailer co-author, sem `--no-verify`/`--no-gpg-sign` salvo pedido explícito.
7. Repetir 4-6 pros concerns restantes.
8. `git log --oneline` do resultado.

## Nunca

- Commitar com quality gate falhando.
- Amend em commit já existente sem pedido explícito (criar commit novo).
- Adicionar arquivo com secret possível sem checar conteúdo antes.
- Push automático.