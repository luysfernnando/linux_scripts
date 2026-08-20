# Hierarquia de custo (barata → cara)

| Camada | Custo tokens | Quando usar | Limite |
|---|---|---|---|
| Hook | ~zero (roda fora do modelo) | regra binária/checável por código (padrão de texto, comando perigoso, formatação) | só aplica regra, não "conhecimento" |
| CLAUDE.md | fixo, pago em toda mensagem da sessão | fato universal: layout do repo, convenção inegociável, tabela de roteamento | maior = mais caro todo turno, qualquer tarefa |
| Skill/Rule referenciada | zero se não chamada; custo do corpo se chamada | "como fazer X" específico de contexto (progressive disclosure) | camada correta pra maioria do conhecimento de projeto |
| Subagente fork | herda conversa+cache (ago/2026), custo de acionar baixo | isolamento de contexto real ou paralelismo genuíno | não troca de modelo em execução |
| Subagente clássico (`model` explícito) | mais caro, recarrega contexto do zero | só quando modelo mais barato (Haiku/Flash) numa tarefa de triagem/alto-volume supera o custo de recarga | padrão tipo Orca (roteamento por custo), não divisão de papel fixo |

## Sinais de camada errada

- Rule violada com frequência → devia ser hook.
- CLAUDE.md com seção que só se aplica a um tipo de tarefa → mover pra skill/doc roteado.
- Subagente fixo por área (design/backend/QA) sem isolamento nem troca de modelo → rule disfarçada de agente.
- Subagentes aninhados pra tarefa simples → contra padrão atual (nesting off por default desde ago/2026), reavaliar.
- Skill nunca dispara → descrição fraca, ou conteúdo devia estar em CLAUDE.md (raro).
- CLAUDE.md e skill/rule dizendo a mesma coisa → consolidar, tokens pagos 2x.
- Múltiplos arquivos de rule dizendo a mesma coisa com palavras diferentes → duplicação, tokens pagos 2x.

## Nota (contexto ago/2026)

Teto prático de subagentes concorrentes por sessão existe. Empilhar nesting manual via env var = complexidade desnecessária — Claude Code removeu até a sugestão de "criar subagente pra cada coisa" do tour inicial.
