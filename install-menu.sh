#!/usr/bin/env bash
# Menu interativo (setas + espaço + gum) pra instalar/restaurar pedaços do
# ricing e o sysup numa máquina nova — automatiza o passo a passo manual
# documentado em ricing/README.md. Ferramenta separada do sysup em si.
set -euo pipefail

REPO_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

need() {
  command -v "$1" >/dev/null 2>&1 || {
    echo "faltando: $1" >&2
    echo "Instale com: sudo pacman -S gum   (ou: go install github.com/charmbracelet/gum@latest)" >&2
    exit 1
  }
}

need gum

# backup_and_link src dst — mesmo padrão de ricing/shell/install.sh: só faz
# backup se dst existir e não for já um symlink (idempotente).
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

action_dotfiles() {
  echo "==> Shell (.zshrc/.bashrc/fish)"
  bash "$REPO_DIR/ricing/shell/install.sh"
}

action_kitty() {
  echo "==> kitty"
  for f in kitty.conf current-theme.conf; do
    backup_and_link "$REPO_DIR/ricing/terminal/kitty/$f" "$HOME/.config/kitty/$f"
  done
}

action_kde_theme() {
  echo "==> Tema KDE (Layan)"
  if ! command -v kwriteconfig6 >/dev/null 2>&1; then
    echo "kwriteconfig6 não encontrado — isso não parece uma sessão KDE Plasma 6, pulando."
    return 0
  fi
  gum confirm "Isso assume que layan-kde-git, papirus-icon-theme-git e layan-cursor-theme já estão instalados (AUR). Continuar?" || {
    echo "cancelado (tema KDE)"
    return 0
  }
  kwriteconfig6 --file kdeglobals --group KDE --key LookAndFeelPackage "com.github.vinceliuice.Layan"
  kwriteconfig6 --file kdeglobals --group Icons --key Theme "Papirus-Dark"
  kwriteconfig6 --file kcminputrc --group Mouse --key cursorTheme "Layan-white-cursors"
  kwriteconfig6 --file kwinrc --group org.kde.kdecoration2 --key theme "__aurorae__svg__Layan"
  kwriteconfig6 --file kwinrc --group org.kde.kdecoration2 --key library "org.kde.kwin.aurorae"
  kwriteconfig6 --file kwinrc --group Plugins --key blurEnabled true
  plasmashell --replace >/dev/null 2>&1 &
  echo "Tema aplicado (plasmashell reiniciando em segundo plano)."
}

action_fastfetch() {
  echo "==> fastfetch"
  backup_and_link "$REPO_DIR/ricing/fastfetch/config.jsonc" "$HOME/.config/fastfetch/config.jsonc"
  backup_and_link "$REPO_DIR/ricing/fastfetch/images" "$HOME/.config/fastfetch/images"
  backup_and_link "$REPO_DIR/ricing/fastfetch/presets" "$HOME/.config/fastfetch/presets"
}

action_ohmyposh() {
  echo "==> oh-my-posh"
  mkdir -p "$HOME/.poshthemes"
  cp "$REPO_DIR/ricing/shell/zsh/tema/p10k.omp.json" "$HOME/.poshthemes/"
  echo "Copiado: ~/.poshthemes/p10k.omp.json"
}

action_sysup() {
  echo "==> sysup (primeira instalação)"
  # shellcheck source=ricing/shell/lib/install-sysup.sh
  source "$REPO_DIR/ricing/shell/lib/install-sysup.sh"
  install_sysup
}

mapfile -t choices < <(gum choose --no-limit \
  --header "O que instalar/configurar? (espaço marca, enter confirma a seleção)" \
  "Shell (.zshrc/.bashrc/fish)" \
  "kitty (symlink)" \
  "Tema KDE (Layan)" \
  "fastfetch (symlink)" \
  "oh-my-posh (copia tema)" \
  "sysup (primeira instalação)")

if [[ ${#choices[@]} -eq 0 ]]; then
  echo "Nada selecionado, saindo."
  exit 0
fi

echo "Selecionado:"
printf '  - %s\n' "${choices[@]}"

gum confirm "Aplicar essas ações agora?" || {
  echo "cancelado"
  exit 0
}

for choice in "${choices[@]}"; do
  case "$choice" in
    "Shell (.zshrc/.bashrc/fish)") action_dotfiles ;;
    "kitty (symlink)") action_kitty ;;
    "Tema KDE (Layan)") action_kde_theme ;;
    "fastfetch (symlink)") action_fastfetch ;;
    "oh-my-posh (copia tema)") action_ohmyposh ;;
    "sysup (primeira instalação)") action_sysup ;;
  esac
done

echo "Concluído."
