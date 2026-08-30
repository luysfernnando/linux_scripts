# Hierarquia de custo (barata → cara)

| Camada | Custo tokens | Quando usar | Limite |
|---|---|---|---|
| Hook | ~zero (roda fora do modelo) | regra binária/checável por código (padrão de texto, comando perigoso, formatação) | só aplica regra, não "conhecimento" |
| CLAUDE.md | fixo, pago em toda mensagem da sessão | fato universal: layout do repo, convenção inegociável, tabela de roteamento | maior = mais caro todo turno, qualquer tarefa |
| Skill/Rule referenciada | zero se não chamada; custo do corpo se chamada | "como fazer X" específico de contexto (progressive disclosure) | camada correta pra maioria do conhecimento de projeto |
| Subagente fork | herda conversa+cache (ago/2026), custo de acionar baixo | isolamento de contexto real ou paralelismo genuíno | não troca de modelo em execução |
| Subagente clássico (`model` explícito) | mais caro, recarrega contexto do zero | só quando modelo mais barato (Haiku/Flash) numa tarefa de triagem/alto-volume supera o custo de recarga | padrão tipo Orca (roteamento por custo), não divisão de papel fixo |
| Workflow (Dynamic Workflows) | mais caro ainda, dezenas/centenas de subagentes num script só | paralelismo genuíno em escala, e só quando usuário pediu explicitamente ("use workflow", "ultracode") | nunca decisão unilateral do Claude — opt-in explícito é regra dura, não preferência |

## Sinais de camada errada

- Rule violada com frequência → devia ser hook.
- CLAUDE.md com seção que só se aplica a um tipo de tarefa → mover pra skill/doc roteado.
- Subagente fixo por área (design/backend/QA) sem isolamento nem troca de modelo → rule disfarçada de agente.
- Subagentes aninhados/paralelos pra tarefa simples sem usuário ter pedido escala → over-engineering (Dynamic Workflows existe pra escala genuína, mas exige opt-in explícito, não é default).
- Regra é só "permite/nega esse comando" (allow/deny) → não precisa hook, cabe como frase em `settings.json` (auto mode, ago/2026). Hook continua certo quando a regra precisa *rodar código* (format-check, bloqueio condicional, etc).
- Skill nunca dispara → descrição fraca, ou conteúdo devia estar em CLAUDE.md (raro).
- CLAUDE.md e skill/rule dizendo a mesma coisa → consolidar, tokens pagos 2x.
- Múltiplos arquivos de rule dizendo a mesma coisa com palavras diferentes → duplicação, tokens pagos 2x.

## Nota (contexto ago/2026)

Teto prático de subagentes concorrentes por sessão existe fora de Workflow. Empilhar nesting manual via env var pra tarefa comum = complexidade desnecessária. Dynamic Workflows é a via oficial pra escala real, mas só dispara com pedido explícito do usuário — nunca de forma proativa.
