#!/usr/bin/env bash
set -euo pipefail

CLAUDE_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=../ricing/shell/lib/log.sh
source "$CLAUDE_DIR/../ricing/shell/lib/log.sh"

if ! command -v rsync >/dev/null 2>&1; then
  log_warn "faltando: rsync"
  if command -v pacman >/dev/null 2>&1; then
    log_dim "instale com: sudo pacman -S rsync"
  elif command -v apt >/dev/null 2>&1; then
    log_dim "instale com: sudo apt install rsync"
  else
    log_dim "instale o pacote 'rsync' pelo gerenciador da sua distro."
  fi
  exit 1
fi

SKILLS_SRC_DIR="$CLAUDE_DIR/skills"
RULES_SRC_DIR="$CLAUDE_DIR/rules"
AGENTS_SKILLS_DIR="$HOME/.agents/skills"
CLAUDE_RULES_DIR="$HOME/.claude/rules"

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
    log_warn "backup: $dst -> $dst.bak"
    mv "$dst" "$dst.bak"
  fi

  ln -sfn "$src" "$dst"
  log_dim "$dst -> $src"
}

link_file() {
  local src="$1" dst="$2"

  if [[ -e "$dst" && ! -L "$dst" ]]; then
    log_warn "backup: $dst -> $dst.bak"
    mv "$dst" "$dst.bak"
  fi

  ln -sfn "$src" "$dst"
  log_dim "$dst -> $src"
}

mkdir -p "$AGENTS_SKILLS_DIR"

log_step "Sincronizando skills"
for skill_path in "$SKILLS_SRC_DIR"/*/; do
  name="$(basename "$skill_path")"

  rsync -a --delete "$skill_path" "$AGENTS_SKILLS_DIR/$name/"

  for agent_dir in "${AGENT_SKILL_DIRS[@]}"; do
    link_skill_into_agent "$name" "$agent_dir"
  done

  log_ok "$name"
done

log_step "Sincronizando rules"
rsync -a --delete "$RULES_SRC_DIR/" "$CLAUDE_RULES_DIR/"
log_ok "rules -> $CLAUDE_RULES_DIR"

log_step "Symlinkando CLAUDE.md e RTK.md"
link_file "$CLAUDE_DIR/CLAUDE.md" "$HOME/.claude/CLAUDE.md"
link_file "$CLAUDE_DIR/RTK.md" "$HOME/.claude/RTK.md"
link_file "$CLAUDE_DIR/settings.json" "$HOME/.claude/settings.json"
log_ok "CLAUDE.md + RTK.md + settings.json"

log_step "Symlinkando filtros do rtk"
mkdir -p "$HOME/.config/rtk"
link_file "$CLAUDE_DIR/rtk-filters.toml" "$HOME/.config/rtk/filters.toml"
log_ok "rtk-filters.toml"
if command -v rtk >/dev/null 2>&1; then
  rtk trust --yes >/dev/null 2>&1 && log_ok "rtk trust --yes" || log_warn "rode 'rtk trust --yes' manualmente pra ativar os filtros custom"
else
  log_dim "rtk não instalado — depois de instalar, rode 'rtk trust --yes' pra ativar os filtros custom"
fi
