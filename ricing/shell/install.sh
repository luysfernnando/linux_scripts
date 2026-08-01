#!/usr/bin/env bash
set -euo pipefail

SHELL_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_DIR="$(cd "$SHELL_DIR/../.." && pwd)"

# backup_and_link src dst — faz backup pra dst.bak se dst existir e não for
# já um symlink (idempotente), depois symlinka src -> dst.
backup_and_link() {
  local src="$1" dst="$2"

  if [[ -e "$dst" && ! -L "$dst" ]]; then
    echo "Backup: $dst -> $dst.bak"
    mv "$dst" "$dst.bak"
  fi

  mkdir -p "$(dirname "$dst")"
  ln -sf "$src" "$dst"
  echo "Linked: $dst -> $src"
}

backup_and_link "$SHELL_DIR/zsh/.zshrc" "$HOME/.zshrc"
backup_and_link "$SHELL_DIR/bash/.bashrc" "$HOME/.bashrc"

# .gitconfig fica de fora: a cópia no repo está desatualizada (assinatura
# GPG antiga) em relação ao que está em uso de verdade (assinatura SSH) —
# symlinkar ela por cima quebraria commits assinados. Reative aqui só depois
# de atualizar ricing/shell/.gitconfig pra bater com o ~/.gitconfig real.

# ---------------------------
# Fish (só se instalado)
# ---------------------------
if command -v fish >/dev/null 2>&1; then
  FISH_CONFIG="$HOME/.config/fish/config.fish"
  SOURCE_LINE="source $SHELL_DIR/fish/config.fish"

  mkdir -p "$HOME/.config/fish"
  touch "$FISH_CONFIG"

  if ! grep -qF "$SOURCE_LINE" "$FISH_CONFIG"; then
    printf '\n# linux_scripts dotfiles (aliases/update)\n%s\n' "$SOURCE_LINE" >> "$FISH_CONFIG"
    echo "Adicionado ao $FISH_CONFIG: $SOURCE_LINE"
  else
    echo "Fish já configurado: $FISH_CONFIG"
  fi
else
  echo "fish não encontrado, pulando config de fish"
fi

# ---------------------------
# sysup (engine de update cross-distro/OS, em Go)
# ---------------------------
# Lógica de instalação em ricing/shell/lib/install-sysup.sh (compartilhada
# com install-menu.sh, que também precisa poder instalar o sysup sozinho).
source "$SHELL_DIR/lib/install-sysup.sh"
install_sysup

# ---------------------------
# Reload certo pro shell atual
# ---------------------------
current_shell="$(basename "${SHELL:-}")"
case "$current_shell" in
  zsh)  echo "Done. Reload shell: source ~/.zshrc" ;;
  bash) echo "Done. Reload shell: source ~/.bashrc" ;;
  fish) echo "Done. Reload shell: exec fish" ;;
  *)    echo "Done. Reload shell manualmente (shell atual: $current_shell)" ;;
esac

echo "Pra manter os mirrors sempre bons em segundo plano, rode: sysup schedule"
