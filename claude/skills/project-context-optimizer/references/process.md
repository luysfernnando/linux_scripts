# Processo de auditoria

## 0. Entender o projeto (antes de tocar em qualquer arquivo)

Ler, nessa ordem, antes do inventário:

1. `CLAUDE.md` do projeto (se existir) — layout, convenções já documentadas.
2. `README.md` da raiz — do que o projeto trata, stack, se tem build/test suite.
3. Manifesto de linguagem presente (`package.json`, `mix.exs`, `Cargo.toml`, `go.mod`, `pyproject.toml`, etc.) — confirma linguagem, scripts de teste/lint reais (não assumir "tem lint" sem ver config).
4. `git log --oneline -20` — tipo de trabalho recente (feature/fix/docs/chore), sinaliza se o projeto é ativo em código ou majoritariamente scripts/config.

Objetivo: toda recomendação das seções seguintes tem que bater com o projeto real. Sinais de recomendação fora de contexto (rejeitar antes de sugerir):
- Sugerir hook/rule de "gate de teste 80%" pra repo sem test suite.
- Sugerir "sempre rodar linter pós-edit" pra linguagem sem linter configurado no repo.
- Copiar regra de outro projeto (ex: convenção de framework web) pra repo que é coleção de scripts standalone.

Se o projeto for pequeno/sem ambiguidade (README + CLAUDE.md já respondem tudo), esse passo é rápido — não precisa de agente dedicado, só leitura direta.

## 1. Inventário

```bash
find . -maxdepth 4 \( -path "*/.claude/*" -o -name "CLAUDE.md" -o -name "*.claude.md" \) -type f 2>/dev/null
```

Pra cada arquivo: caminho, tamanho (linhas/tokens), sempre-carregado (CLAUDE.md/rules) ou sob-demanda (skills/agents). Ler também hooks em `settings.json`/`.claude/settings.json` — não aparecem como arquivo solto.

## 2. Dados reais de uso (antes de recomendar por suposição)

Não adivinhe se uma skill/subagente/rule é usada — o Claude Code expõe isso nativamente:

- **`/usage`** — aba Attribution: % de uso recente por skill, subagente, plugin e MCP server individual. `d`/`w` alterna 24h vs 7 dias. Uso principal aqui: achar skill com 0% (candidata a "nunca dispara", ver árvore item 3) ou subagente/plugin consumindo % desproporcional pro valor que entrega.
- **`/insights`** — analisa até 200 sessões recentes desta máquina, gera relatório HTML (`~/.claude/usage-data/report.html`) com padrões de uso e fricção. Útil pra achar rule/skill que devia disparar e não dispara (frustração recorrente = sinal de descrição fraca ou item na camada errada).
- **OpenTelemetry** (`CLAUDE_CODE_ENABLE_TELEMETRY=1`, `OTEL_METRICS_EXPORTER`, `OTEL_LOGS_EXPORTER`) — só se o usuário já tiver backend (Prometheus/Datadog/etc). Métricas `claude_code.cost.usage`/`claude_code.token.usage` e evento `claude_code.tool_result` carregam `skill.name`/`agent.name`/`plugin.name` — dá contagem de invocação histórica, não só 24h/7d. Não sugerir setup de OTel só pra essa auditoria pontual, custo de infra não compensa — mencionar como opção se o projeto já tem observability.

Rodar `/usage` (e `/insights` se disponível) é passo padrão do inventário, não opcional — está no formato nativo, custo zero de tooling extra.

### 2b. Minerar `/insights` por padrão repetido SEM skill/hook (direção oposta)

O inventário de arquivos só pega o que já existe. `/insights` pega o oposto: instrução que o usuário repete manualmente sessão após sessão porque nunca virou skill/hook — tokens pagos toda vez reexplicando, quando uma skill resolveria de graça. Ler as seções `friction_analysis` e `suggestions.features_to_try`/`claude_md_additions` do relatório e cruzar com sinais de:

- **Categoria de sessão com contagem alta + regra repetida no prompt** (ex: "27 sessões de git, sempre pedindo 'sem co-author', 'commits incrementais'") → candidato a skill (`/commit`) ou CLAUDE.md, não a hook, é workflow não regra binária.
- **Friction "scope misjudgment"/"overreach" recorrente** (editou N arquivos em vez do componente compartilhado) → candidato a **hook PreToolUse** que bloqueia edit em >N arquivos sem plano aprovado — regra binária, checável por código.
- **Friction "unverified claims"** (afirmou fato técnico sem checar doc oficial) → não vira config nova, mas justifica linha em CLAUDE.md/rule: "verificar claim técnico contra fonte oficial antes de escrever como fato".
- **Módulo repetido ~N vezes com mesmo template** (ex: 6 migrações de LiveView) → candidato a subagente fork paralelo (isolamento real + paralelismo genuíno, bate item 4 da árvore), não a skill sequencial.

Diferença pro resto do processo: aqui a saída não é "mover X de camada", é "criar Y que não existe". Mesma tabela de saída (seção 6), linha nova com "Local atual: não existe (só instrução manual repetida)".

## 2c. Verificação de roteamento (toda rule/skill existente)

Pra cada rule/skill do inventário (item 1), checar 3 coisas, independente de rodar `/usage`:

1. **Descrição casa com gatilho real?** A frase-gatilho na `description` (skill) ou no corpo (rule) cobre como o usuário de fato pede a coisa (em PT-BR e EN, se o usuário mistura idioma)? Gatilho estreito demais = nunca dispara; gatilho genérico demais = dispara errado/sempre.
2. **Sobreposição com outra rule/skill/CLAUDE.md?** Duas fontes dizendo a mesma regra com palavras diferentes = tokens pagos 2x, e risco de uma ficar desatualizada sem a outra acompanhar.
3. **Referência morta?** Arquivo, comando ou path que a rule/skill cita ainda existe? (ex: rule falando de agente que nunca foi criado — ver achado real desta auditoria: `agents.md` da ecc descrevia 11 agentes em `~/.claude/agents/` que nunca existiram no disco.)

Qualquer um dos 3 falhando é motivo de recomendação (corrigir gatilho, consolidar, ou remover), mesmo sem dado de `/usage`.

## 3. Árvore de decisão (pare na 1ª que bater)

1. Regra precisa valer 100% das vezes sem depender do modelo lembrar? → **Hook**. Zero custo, ~100% aplicação vs ~70% em texto. Ex: bloquear padrão proibido, rodar linter pós-edit, impedir comando perigoso.
2. Fato necessário em toda mensagem do projeto? → **CLAUDE.md**, só isso. "Só às vezes"/"só nesse módulo" → não é isso, desce.
3. Conhecimento de procedimento, útil só quando a tarefa bate um contexto específico? → **Skill/Rule referenciada** (`.claude/rules/*.md` ou skill formal). Sob demanda, custo zero se irrelevante. Aqui entra a maioria do que hoje vai errado pra CLAUDE.md ou subagente. Se `/usage` mostrar 0% de uso em 7 dias pra essa skill, questionar: descrição fraca (não dispara quando devia) ou conteúdo obsoleto (remover)?
4. Tarefa se beneficia de isolamento real, paralelismo genuíno, ou modelo diferente por custo? → **Subagente**:
   - Fork (padrão atual): herda conversa+cache, custo baixo. Pra exploração pesada, revisão de módulo inteiro, output intermediário irrelevante ao contexto principal.
   - Clássico com modelo diferente: só compensa quando modelo mais barato (Haiku/Flash) numa tarefa de triagem/alto-volume supera o custo de recarga sem cache. Padrão tipo Orca (roteamento por custo), não divisão de papel fixo (não criar "agente front-end" + "agente back-end" fixos — rule resolve de graça).
5. Não bateu em nada com convicção? → Provavelmente não precisa existir como config formal. Instrução pontual na conversa, não fixar.

## 4. Sinais de over-engineering

Ver `references/decision-framework.md` → "Sinais de camada errada".

## 5. Exceção de qualidade

Custo de token é critério dominante, não absoluto. Mover pra camada mais cara vale a pena SE resolve problema real e recorrente de qualidade — sinalizar explicitamente com justificativa escrita (nunca silenciosamente).

- Válida: revisão de segurança/compliance onde subagente com tools restritas + outro modelo pega genuinamente mais bugs que a regra em texto.
- Inválida: "fica mais organizado ter um agente por área" — preferência estética, não justifica custo.

## 6. Saída esperada

Tabela antes de mudar qualquer arquivo, com coluna de uso real quando `/usage`/`/insights` rodou:

| Item encontrado | Local atual | Uso real (7d) | Recomendação | Motivo | Impacto em tokens | Ação |
|---|---|---|---|---|---|---|
| "Nunca Repo direto, sempre Ash actions" | CLAUDE.md | — (regra, não skill) | Hook (PreToolUse) | Regra binária, hoje seguida ~70% por ser só texto | Reduz a zero custo nessa checagem | Criar hook |
| docs/design-system.md checklist | Rule enxuta | — | Manter | Conhecimento contextual, sob demanda, já correto | Neutro | Nenhuma |
| Agente "front-end-builder" proposto | Não existe | — | Não criar | Sem isolamento/paralelismo real, rule já cobre | Evita custo de spawn à toa | Não implementar |
| Skill "legacy-deploy-helper" | `.claude/skills/` | 0% em 7d (`/usage`) | Remover ou revisar descrição | Nunca disparou — ou ninguém precisou, ou trigger não casa com pedido real | Corpo da skill não é lido nunca, mas nome+descrição ocupam listing sempre | Perguntar ao usuário antes de apagar |
| "Sem co-author, commits incrementais, rodar check.sh antes" | Não existe (instrução manual repetida em ~27 sessões, via `/insights`) | Criar skill `/commit` | Regra de workflow reexplicada toda sessão de git | Corta tokens de reexplicar + reduz erro de esquecer a regra | Criar skill |
| Bloquear edit em >5 arquivos sem plano aprovado | Não existe (friction "scope misjudgment" recorrente, via `/insights`) | Criar hook PreToolUse | Regra binária/checável por código, hoje só é lembrada quando o usuário pega a tempo | Zero custo de token, 100% aplicação | Criar hook |

Tabela sempre termina com uma seção **"Sugestões novas"** (mesmo sem `/insights` disponível) — o que hoje não existe como config mas devia, baseado no item 0 (realidade do projeto) + 2c (roteamento quebrado). Se nada novo se justifica, escrever "Nenhuma sugestão nova — cobertura atual adequada" em vez de omitir a seção.

Só após validação do usuário, implementar (hooks, mover conteúdo, criar/remover arquivos). Não mexer em CLAUDE.md nem criar/apagar agente sem essa confirmação — custo de errar aqui > custo de perguntar.

## 7. Hooks

Se recomendação for rule → hook, ver `references/hooks-guide.md` (eventos, payload JSON, exit code 2 pra bloqueio, formato `decision`/`reason` deprecado).

## 8. Registro de decisão

Ao final, se o projeto não tiver, sugerir `docs/context-architecture.md` curto: o que foi pra hook/rule/subagente e por quê. Evita retrabalho em sessão futura sem inchar CLAUDE.md — é só um doc que a skill relê quando chamada de novo.
