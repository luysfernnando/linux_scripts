#!/usr/bin/env bash
set -euo pipefail

DOTFILES_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

FILES=(.zshrc .bashrc .gitconfig)

backup_and_link() {
  local src="$DOTFILES_DIR/$1"
  local dst="$HOME/$1"

  if [[ -e "$dst" && ! -L "$dst" ]]; then
    echo "Backup: $dst -> $dst.bak"
    mv "$dst" "$dst.bak"
  fi

  ln -sf "$src" "$dst"
  echo "Linked: $dst -> $src"
}

for f in "${FILES[@]}"; do
  backup_and_link "$f"
done

echo "Done. Reload shell: source ~/.zshrc"
