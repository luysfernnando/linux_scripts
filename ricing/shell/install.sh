#!/usr/bin/env bash
set -euo pipefail

SHELL_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_DIR="$(cd "$SHELL_DIR/../.." && pwd)"

source "$SHELL_DIR/lib/log.sh"

need() {
  command -v "$1" >/dev/null 2>&1 || {
    log_warn "faltando: $1 — pulando passo que depende dele."
    return 1
  }
}

# backup_and_link src dst — faz backup pra dst.bak se dst existir e não for
# já um symlink (idempotente), depois symlinka src -> dst.
backup_and_link() {
  local src="$1" dst="$2"

  if [[ -e "$dst" && ! -L "$dst" ]]; then
    log_warn "backup: $dst -> $dst.bak"
    mv "$dst" "$dst.bak"
  fi

  mkdir -p "$(dirname "$dst")"
  ln -sf "$src" "$dst"
  log_ok "$dst -> $src"
}

backup_and_link "$SHELL_DIR/.gitconfig" "$HOME/.gitconfig"

# .gitconfig usa assinatura SSH (gpg.format=ssh) — cada máquina precisa da
# própria chave em ~/.ssh/id_luysfernnando_sign_commits (gerar com ssh-keygen
# se não existir) cadastrada no GitHub como "Signing Key". Ver ricing/README.md.

# ---------------------------
# Só linka o profile do shell que você realmente usa ($SHELL) — evita
# lixo de .bashrc/.zshrc linkado sem necessidade em máquina nova.
# ---------------------------
detected_shell="$(basename "${SHELL:-}")"
log_step "Shell detectado: $detected_shell"

case "$detected_shell" in
  zsh)  backup_and_link "$SHELL_DIR/zsh/.zshrc" "$HOME/.zshrc" ;;
  bash) backup_and_link "$SHELL_DIR/bash/.bashrc" "$HOME/.bashrc" ;;
  fish)
    FISH_CONFIG="$HOME/.config/fish/config.fish"
    SOURCE_LINE="source $SHELL_DIR/fish/config.fish"

    mkdir -p "$HOME/.config/fish"
    touch "$FISH_CONFIG"

    if ! grep -qF "$SOURCE_LINE" "$FISH_CONFIG"; then
      printf '\n# linux_scripts dotfiles (aliases/update)\n%s\n' "$SOURCE_LINE" >> "$FISH_CONFIG"
      log_ok "adicionado ao $FISH_CONFIG"
    else
      log_dim "fish já configurado: $FISH_CONFIG"
    fi
    ;;
  *)
    log_warn "shell '$detected_shell' não suportado (só zsh/bash/fish), pulando dotfiles de shell."
    ;;
esac

# ---------------------------
# Checklist de dependências externas (binários/frameworks usados pelos
# aliases e plugins) — primeiro só verifica e mostra o que falta, depois
# pergunta antes de instalar qualquer coisa (rede/sudo).
# ---------------------------
source "$SHELL_DIR/lib/install-cli-tools.sh"
source "$SHELL_DIR/lib/install-zsh-plugins.sh"

# Gerenciador de tema antigo (oh-my-posh, powerlevel10k) rodando junto com
# starship é lixo garantido — pergunta antes de checar/instalar o resto.
if [[ "$detected_shell" == zsh ]]; then
  detect_and_offer_uninstall_prompt_managers
fi

# item_desc/item_check/item_install id — tabela de despacho. "plugin:<nome>"
# cobre os plugins zsh (mesma lógica, url por nome via ZSH_PLUGIN_URL).
item_desc() {
  case "$1" in
    lsd)      echo "lsd (binário — aliases ls/l/la/lla/lstree)" ;;
    fzf)      echo "fzf (binário — dep do fzf-tab)" ;;
    imagemagick) echo "imagemagick (dep do fastfetch pra decodificar a imagem do logo)" ;;
    ohmyzsh)  echo "oh-my-zsh (framework zsh)" ;;
    plugin:*) echo "plugin zsh: ${1#plugin:}" ;;
    starship) echo "starship (prompt)" ;;
  esac
}
item_check() {
  case "$1" in
    lsd)      check_lsd ;;
    fzf)      check_fzf ;;
    imagemagick) check_imagemagick ;;
    ohmyzsh)  check_ohmyzsh ;;
    plugin:*) check_zsh_plugin "${1#plugin:}" ;;
    starship) check_starship ;;
  esac
}
item_install() {
  case "$1" in
    lsd)      install_lsd ;;
    fzf)      install_fzf ;;
    imagemagick) install_imagemagick ;;
    ohmyzsh)  install_ohmyzsh ;;
    plugin:*) install_zsh_plugin "${1#plugin:}" ;;
    starship) install_starship ;;
  esac
}

items=(lsd fzf imagemagick)
if [[ "$detected_shell" == zsh ]]; then
  items+=(ohmyzsh plugin:zsh-autosuggestions plugin:fast-syntax-highlighting plugin:zsh-completions plugin:fzf-tab starship)
fi

echo
log_step "Checando dependências..."
missing=()
for id in "${items[@]}"; do
  if item_check "$id"; then
    log_ok "$(item_desc "$id")"
  else
    log_warn "faltando: $(item_desc "$id")"
    missing+=("$id")
  fi
done

if [[ ${#missing[@]} -eq 0 ]]; then
  log_dim "nada pra instalar."
else
  echo
  log_step "Pendente (${#missing[@]}):"
  for id in "${missing[@]}"; do
    log_dim "  - $(item_desc "$id")"
  done
  echo
  if confirm "Instalar os ${#missing[@]} itens acima agora?"; then
    for id in "${missing[@]}"; do
      item_install "$id" || true
    done
  else
    log_dim "pulado — rode o script de novo quando quiser instalar."
  fi
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
case "$detected_shell" in
  zsh)  reload_cmd="source ~/.zshrc" ;;
  bash) reload_cmd="source ~/.bashrc" ;;
  fish) reload_cmd="exec fish" ;;
  *)    reload_cmd="(shell atual: $detected_shell, recarregue manualmente)" ;;
esac

log_box "Pronto! Reload shell: $reload_cmd" 42
log_dim "Pra manter os mirrors sempre bons em segundo plano, rode: sysup schedule"
