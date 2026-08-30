---
name: claude-config-optimizer
description: Audita a arquitetura de contexto do Claude Code em dois modos — projeto (CLAUDE.md, .claude/rules, .claude/agents, .claude/hooks, .claude/skills, MCPs de 1 repo) e global (~/.agents/rules/common, ~/.agents/skills, duplicação entre repos) — e recomenda pra qual camada/escopo cada instrução deveria ir, com custo de tokens como critério dominante. Use sempre que o usuário pedir para "otimizar o projeto", "revisar CLAUDE.md", "ver se compensa criar subagente", "auditar rules/hooks/agents", "reduzir tokens do setup", "reorganizar contexto do Claude", "subir regra pra global", "achar duplicação entre repos", ou rodar a evolução periódica (manual, sem cron). Também use proativamente quando o usuário estiver prestes a criar um novo subagente, hook ou rule sem antes ter comparado com as alternativas mais baratas.
---

# Claude Config Optimizer

Decide onde cada instrução deve morar — camada (`CLAUDE.md` / `.claude/rules/*.md` / `.claude/skills/*` / `.claude/hooks/*` / `.claude/agents/*`) E escopo (local a 1 repo vs global em `~/.agents/`). Critério dominante: custo de tokens, medido com dados reais de uso (`/usage`, `/insights`), não suposição. Só sobe de camada/escopo quando há ganho de qualidade real e explícito, nunca por organização/preferência estética.

**Antes de agir, leia `references/process.md` inteiro** — passo a passo completo: entender do que trata o projeto primeiro, inventário, verificação de roteamento, árvore de decisão de camada, detecção de duplicação local×global (modo global), sinais de over-engineering, formato de saída.

**Também disponível:**
- `references/decision-framework.md` — hierarquia de custo (hook < CLAUDE.md < skill < subagente fork < subagente clássico < Workflow) e notas sobre estado atual (ago/2026).
- `references/hooks-guide.md` — como implementar hook quando a auditoria recomendar migrar rule pra lá.

**Modo global (manual, sem agendamento):** varre `~/.agents/rules/common`, `~/.agents/skills` e repos conhecidos, procurando regra que deveria ser global (comportamento genérico do Claude, não domínio do projeto) e duplicação entre repo×global. Usuário decide quando rodar — sem cron (cloud não enxerga arquivo local; agendamento local do Claude Code é session-only, não dura). Estado da última rodada fica em memory (`reference`, projeto `linux_scripts`) — só pra avisar se já passou muito tempo, não pra disparar sozinho. Ver seção "Modo global" do `process.md`.

Nunca mexa em CLAUDE.md, crie/apague hook ou agente, ou mova rule entre repo e global, sem antes mostrar a tabela de recomendação ao usuário e ter aprovação.
