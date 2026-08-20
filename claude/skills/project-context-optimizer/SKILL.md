---
name: project-context-optimizer
description: Audita a arquitetura de contexto de um projeto Claude Code (CLAUDE.md, .claude/rules, .claude/agents, .claude/hooks, .claude/skills, MCPs configurados) e recomenda para qual camada cada instrução deveria ir, com custo de tokens como critério dominante. Use sempre que o usuário pedir para "otimizar o projeto", "revisar CLAUDE.md", "ver se compensa criar subagente", "auditar rules/hooks/agents", "reduzir tokens do setup", "reorganizar contexto do Claude", ou qualquer pedido de reestruturação da configuração do Claude Code para eficiência. Também use proativamente quando o usuário estiver prestes a criar um novo subagente, hook ou rule sem antes ter comparado com as alternativas mais baratas.
---

# Project Context Optimizer

Decide onde cada instrução do projeto deve morar: `CLAUDE.md`, `.claude/rules/*.md`, `.claude/skills/*`, `.claude/hooks/*` ou `.claude/agents/*` (subagente). Critério dominante: custo de tokens, medido com dados reais de uso (`/usage`, `/insights`), não suposição. Só sobe de camada (mais caro) quando há ganho de qualidade real e explícito, nunca por organização/preferência estética.

**Antes de agir, leia `references/process.md` inteiro** — ele tem o passo a passo completo: entender do que trata o projeto primeiro (README/CLAUDE.md/manifesto, pra recomendação não sair descolada da realidade), inventário, verificação de roteamento de cada rule/skill existente (gatilho bate? sobrepõe outra fonte? referencia algo que não existe mais?), árvore de decisão, sinais de over-engineering, e formato de saída (sempre com seção de sugestões novas). Este arquivo é só o resumo para saber se vale a pena acionar.

**Também disponível:**
- `references/decision-framework.md` — hierarquia de custo (hook < CLAUDE.md < skill < subagente fork < subagente clássico) e notas sobre o estado atual de subagentes (ago/2026).
- `references/hooks-guide.md` — como implementar um hook quando a auditoria recomendar migrar uma rule pra lá.

Nunca mexa em CLAUDE.md, crie/apague hook ou agente sem antes mostrar a tabela de recomendação ao usuário e ter aprovação.
