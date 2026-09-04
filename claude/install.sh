#!/usr/bin/env bash
set -euo pipefail

CLAUDE_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=../ricing/shell/lib/log.sh
source "$CLAUDE_DIR/../ricing/shell/lib/log.sh"
# is_windows() + backup_and_link() — este script também roda em Git Bash, onde
# `ln -s` cru copia em vez de linkar (armadilha do MSYS documentada em
# lib/link.sh).
# shellcheck source=../ricing/shell/lib/link.sh
source "$CLAUDE_DIR/../ricing/shell/lib/link.sh"

# mirror_dir src dst — replica src/ em dst/, removendo o que não existe mais
# em src (equivalente ao uso de `rsync -a --delete` abaixo). Usa rsync quando
# disponível; no Windows (Git Bash) não tem pacote rsync real no winget, então
# cai pro fallback rm+cp (mesmo resultado pros dois usos aqui, que são sempre
# mirror completo de diretório).
if command -v rsync >/dev/null 2>&1; then
  mirror_dir() { rsync -a --delete "$1" "$2"; }
else
  log_dim "rsync não encontrado — usando fallback rm+cp (mesmo resultado pra mirror de diretório)"
  mirror_dir() {
    local src="${1%/}" dst="${2%/}"
    rm -rf "$dst"
    mkdir -p "$dst"
    cp -r "$src/." "$dst/"
  }
fi

SKILLS_SRC_DIR="$CLAUDE_DIR/skills"
RULES_SRC_DIR="$CLAUDE_DIR/rules"
AGENTS_SKILLS_DIR="$HOME/.agents/skills"
AGENTS_RULES_DIR="$HOME/.agents/rules"

# Diretórios de agente que descobrem skills e rules via ~/.agents — symlinkados
# (Claude Code e Gemini / Antigravity CLI).
AGENT_SKILL_DIRS=(
  "$HOME/.claude/skills"
  "$HOME/.gemini/config/skills"
)

AGENT_RULE_DIRS=(
  "$HOME/.claude/rules"
  "$HOME/.gemini/config/rules"
)

link_skill_into_agent() {
  local name="$1" agent_dir="$2"
  mkdir -p "$agent_dir"
  backup_and_link "$AGENTS_SKILLS_DIR/$name" "$agent_dir/$name"
}

link_file() { backup_and_link "$1" "$2"; }

mkdir -p "$AGENTS_SKILLS_DIR"

log_step "Sincronizando skills"
for skill_path in "$SKILLS_SRC_DIR"/*/; do
  name="$(basename "$skill_path")"

  mirror_dir "$skill_path" "$AGENTS_SKILLS_DIR/$name/"

  for agent_dir in "${AGENT_SKILL_DIRS[@]}"; do
    link_skill_into_agent "$name" "$agent_dir"
  done

  log_ok "$name"
done

log_step "Sincronizando rules"
mkdir -p "$AGENTS_RULES_DIR"
mirror_dir "$RULES_SRC_DIR/" "$AGENTS_RULES_DIR/"

for agent_rules_dir in "${AGENT_RULE_DIRS[@]}"; do
  mkdir -p "$(dirname "$agent_rules_dir")"
  link_file "$AGENTS_RULES_DIR" "$agent_rules_dir"
done
log_ok "rules -> $AGENTS_RULES_DIR (espelhado em: ${AGENT_RULE_DIRS[*]})"

log_step "Symlinkando CLAUDE.md e RTK.md"
link_file "$CLAUDE_DIR/CLAUDE.md" "$HOME/.claude/CLAUDE.md"
link_file "$CLAUDE_DIR/RTK.md" "$HOME/.claude/RTK.md"
link_file "$CLAUDE_DIR/settings.json" "$HOME/.claude/settings.json"
log_ok "CLAUDE.md + RTK.md + settings.json"

log_step "Symlinkando settings.json do Antigravity CLI"
mkdir -p "$HOME/.gemini/antigravity-cli"
link_file "$CLAUDE_DIR/../gemini/antigravity-cli/settings.json" "$HOME/.gemini/antigravity-cli/settings.json"
log_ok "antigravity-cli settings.json"

log_step "Symlinkando filtros do rtk"
mkdir -p "$HOME/.config/rtk"
link_file "$CLAUDE_DIR/rtk-filters.toml" "$HOME/.config/rtk/filters.toml"
log_ok "rtk-filters.toml"
if command -v rtk >/dev/null 2>&1; then
  rtk trust --yes >/dev/null 2>&1 && log_ok "rtk trust --yes" || log_warn "rode 'rtk trust --yes' manualmente pra ativar os filtros custom"
else
  log_dim "rtk não instalado — depois de instalar, rode 'rtk trust --yes' pra ativar os filtros custom"
fi
