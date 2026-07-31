#!/usr/bin/env bash
set -euo pipefail

CLAUDE_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SKILLS_SRC_DIR="$CLAUDE_DIR/skills"
AGENTS_SKILLS_DIR="$HOME/.agents/skills"

# Diretórios de agente que descobrem skills via ~/.agents/skills — symlinkados
# por skill. Hoje só o Claude Code tem esse conceito; adicionar outro agente
# no futuro é só incluir o path aqui.
AGENT_SKILL_DIRS=("$HOME/.claude/skills")

link_skill_into_agent() {
  local name="$1" agent_dir="$2"
  local src="$AGENTS_SKILLS_DIR/$name"
  local dst="$agent_dir/$name"

  mkdir -p "$agent_dir"

  if [[ -e "$dst" && ! -L "$dst" ]]; then
    echo "Backup: $dst -> $dst.bak"
    mv "$dst" "$dst.bak"
  fi

  ln -sfn "$src" "$dst"
  echo "Linked: $dst -> $src"
}

mkdir -p "$AGENTS_SKILLS_DIR"

for skill_path in "$SKILLS_SRC_DIR"/*/; do
  name="$(basename "$skill_path")"

  rsync -a --delete "$skill_path" "$AGENTS_SKILLS_DIR/$name/"
  echo "Copiado: $name -> $AGENTS_SKILLS_DIR/$name"

  for agent_dir in "${AGENT_SKILL_DIRS[@]}"; do
    link_skill_into_agent "$name" "$agent_dir"
  done
done

echo "Done. Skills instaladas: $(ls -1 "$SKILLS_SRC_DIR")"
