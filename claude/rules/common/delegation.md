# Delegation (Subagents)

Task shape (not tool name) decides the tool — check before Explore/Task/inline edit:

| Situation | Use | Instead of |
|---|---|---|
| Pure locate/enumerate, no discussion ("where is X defined", "what calls Y", "list uses of Z") | `cavecrew-investigator` | vanilla Explore |
| Confirmed ≤2-file mechanical edit (typo, rename, format-preserving) | `cavecrew-builder` | inline edit / generic Task agent |
| Diff/PR review, findings-only | `cavecrew-reviewer` | vanilla reviewer / manual read |
| Ambiguous, >2 files, needs judgment | Do directly | any cavecrew agent (refuses/underperforms) |

Full logic: `cavecrew` skill (caveman plugin).
