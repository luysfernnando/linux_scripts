#!/usr/bin/env bash
# Menu interativo (setas + gum) pra instalar/restaurar pedaços do ricing e o
# sysup numa máquina nova — automatiza o passo a passo manual documentado em
# ricing/README.md. Ferramenta separada do sysup em si.
set -euo pipefail

REPO_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

is_windows() { [[ "$(uname -s)" == MINGW* || "$(uname -s)" == MSYS* ]]; }

need() {
  command -v "$1" >/dev/null 2>&1 || {
    echo "faltando: $1" >&2
    if is_windows; then
      echo "Instale com: winget install $1" >&2
    elif [[ -f /etc/debian_version ]]; then
      echo "Instale com: sudo apt install $1" >&2
    else
      echo "Instale com: sudo pacman -S $1" >&2
    fi
    exit 1
  }
}

# add_winget_pkg_to_path bin winget_id — winget "cria" um alias de linha de
# comando em WinGet/Links via symlink NTFS, mas isso exige Developer Mode
# ligado ou terminal elevado — sem isso ele finge sucesso e o symlink nunca
# existe. O jeito confiável é achar o .exe direto dentro de
# WinGet/Packages/<winget_id>_* (estrutura interna varia por pacote, então
# busca recursiva) e somar o dir dele no PATH. Retorna 1 se não achou nada
# instalado (nem pacote, nem exe dentro dele).
add_winget_pkg_to_path() {
  local bin="$1" winget_id="$2"
  local appdata_unix pkg_glob pkg_dir exe_path
  appdata_unix="${LOCALAPPDATA//\\//}"
  appdata_unix="$(sed -E 's#^([A-Za-z]):#/\L\1#' <<<"$appdata_unix")"
  pkg_glob="$appdata_unix/Microsoft/WinGet/Packages/${winget_id}"_*
  pkg_dir="$(compgen -G "$pkg_glob" | head -1)"
  [[ -n "$pkg_dir" ]] || return 1
  exe_path="$(find "$pkg_dir" -iname "${bin}.exe" | head -1)"
  [[ -n "$exe_path" ]] || return 1
  export PATH="$PATH:$(dirname "$exe_path")"
  hash -r
}

# winget_install_if_missing bin winget_id — no Windows (git bash), winget é o
# caminho normal pra instalar sozinho em vez de só reclamar, já que não tem
# pacman/apt pra sugerir.
winget_install_if_missing() {
  local bin="$1" winget_id="$2"
  is_windows || return 0
  command -v "$bin" >/dev/null 2>&1 && return 0

  # Já instalado por uma sessão anterior, só que o PATH desta sessão nova
  # ainda não sabe disso (o fix de PATH abaixo só vale pro processo atual) —
  # acha e usa sem chamar o winget de novo.
  add_winget_pkg_to_path "$bin" "$winget_id" && return 0

  need winget
  echo "faltando: $bin — instalando via winget..." >&2
  winget install --id "$winget_id" -e --source winget --accept-source-agreements --accept-package-agreements
  add_winget_pkg_to_path "$bin" "$winget_id"
}

winget_install_if_missing gum charmbracelet.gum
need gum

source "$REPO_DIR/ricing/shell/lib/log.sh"

# backup_and_link src dst — mesmo padrão de ricing/shell/install.sh: só faz
# backup se dst existir e não for já um symlink (idempotente).
backup_and_link() {
  local src="$1" dst="$2"
  if [[ -e "$dst" && ! -L "$dst" ]]; then
    # No Windows, ln -s em diretório sem Developer Mode/terminal elevado cai
    # pra copiar o conteúdo em vez de linkar de verdade — dst nunca vira
    # symlink, então toda re-execução veria "conteúdo novo" aqui. Se for
    # idêntico ao src (ou seja, é essa cópia-fallback de uma execução
    # anterior, não dado do usuário), só substitui em vez de empilhar backup.
    if diff -rq "$src" "$dst" >/dev/null 2>&1; then
      rm -rf "$dst"
    else
      log_warn "backup: $dst -> $dst.bak"
      rm -rf "$dst.bak"
      mv "$dst" "$dst.bak"
    fi
  fi
  mkdir -p "$(dirname "$dst")"
  ln -sf "$src" "$dst"
  log_ok "$dst -> $src"
}

action_dotfiles() {
  log_step "Shell (.bashrc/fish)"
  bash "$REPO_DIR/ricing/shell/install.sh"
}

action_kitty() {
  log_step "kitty"
  for f in kitty.conf current-theme.conf; do
    backup_and_link "$REPO_DIR/ricing/terminal/kitty/$f" "$HOME/.config/kitty/$f"
  done
}

action_kde_theme() {
  log_step "Tema KDE (Layan)"
  if ! command -v kwriteconfig6 >/dev/null 2>&1; then
    log_warn "kwriteconfig6 não encontrado — isso não parece uma sessão KDE Plasma 6, pulando."
    return 0
  fi
  gum confirm "Isso assume que layan-kde-git, papirus-icon-theme-git e layan-cursor-theme já estão instalados (AUR). Continuar?" || {
    log_warn "cancelado (tema KDE)"
    return 0
  }
  kwriteconfig6 --file kdeglobals --group KDE --key LookAndFeelPackage "com.github.vinceliuice.Layan"
  kwriteconfig6 --file kdeglobals --group Icons --key Theme "Papirus-Dark"
  kwriteconfig6 --file kcminputrc --group Mouse --key cursorTheme "Layan-white-cursors"
  kwriteconfig6 --file kwinrc --group org.kde.kdecoration2 --key theme "__aurorae__svg__Layan"
  kwriteconfig6 --file kwinrc --group org.kde.kdecoration2 --key library "org.kde.kwin.aurorae"
  kwriteconfig6 --file kwinrc --group Plugins --key blurEnabled true
  plasmashell --replace >/dev/null 2>&1 &
  log_ok "tema aplicado (plasmashell reiniciando em segundo plano)"
}

# WSL (Windows Terminal) e kitty nativo pedem protocolo de imagem diferente
# no fastfetch — "auto" do fastfetch não acerta isso de forma confiável (viu
# na prática: escolheu kitty dentro do WSL e caiu pra bytes crus na tela).
is_wsl() { grep -qi microsoft /proc/version 2>/dev/null || [[ -n "${WSL_DISTRO_NAME:-}" ]]; }

# render_fastfetch_config — gera (não symlinka) ~/.config/fastfetch/config.jsonc
# a partir do template do repo, substituindo @LOGO_TYPE@ pelo protocolo certo
# pra essa máquina. É um COPY de propósito, não symlink: o valor varia por
# máquina, então symlinkar geraria diff no git toda vez que WSL != nativo.
render_fastfetch_config() {
  local tmpl="$REPO_DIR/ricing/fastfetch/config.jsonc.tmpl" dst="$HOME/.config/fastfetch/config.jsonc"
  local logo_type="kitty"
  is_wsl && logo_type="sixel"

  mkdir -p "$(dirname "$dst")"
  if [[ -e "$dst" && ! -L "$dst" ]]; then
    log_warn "backup: $dst -> $dst.bak"
    mv "$dst" "$dst.bak"
  fi
  rm -f "$dst" # se ainda for symlink de uma instalação antiga, troca por arquivo real

  sed "s/@LOGO_TYPE@/$logo_type/" "$tmpl" > "$dst"
  log_ok "$dst (logo.type=$logo_type$(is_wsl && echo " — WSL detectado"))"
}

action_fastfetch() {
  log_step "fastfetch"
  winget_install_if_missing fastfetch Fastfetch-cli.Fastfetch
  need fastfetch
  render_fastfetch_config
  backup_and_link "$REPO_DIR/ricing/fastfetch/images" "$HOME/.config/fastfetch/images"
  backup_and_link "$REPO_DIR/ricing/fastfetch/presets" "$HOME/.config/fastfetch/presets"
}

action_starship() {
  log_step "starship"
  # shellcheck source=ricing/shell/lib/install-cli-tools.sh
  source "$REPO_DIR/ricing/shell/lib/install-cli-tools.sh"
  # shellcheck source=ricing/shell/lib/install-shell-tools.sh
  source "$REPO_DIR/ricing/shell/lib/install-shell-tools.sh"
  detect_and_offer_uninstall_prompt_managers
  install_starship
}

action_git() {
  log_step "Git (assinatura SSH + ssh-agent)"
  # shellcheck source=ricing/shell/lib/install-git-ssh.sh
  source "$REPO_DIR/ricing/shell/lib/install-git-ssh.sh"

  if check_sign_key; then
    log_ok "chave de assinatura ($SIGN_KEY.pub)"
  else
    log_warn "faltando: chave de assinatura de commit"
    gum confirm "Gerar chave de assinatura agora?" && install_sign_key
  fi

  if check_allowed_signers; then
    log_ok "allowed_signers"
  else
    log_warn "faltando: allowed_signers"
    install_allowed_signers
  fi

  if check_ssh_agent_socket; then
    log_ok "ssh-agent.socket (systemd)"
  else
    log_warn "faltando: ssh-agent.socket habilitado"
    install_ssh_agent_socket
  fi

  if check_keys_loaded; then
    log_ok "chaves carregadas no ssh-agent"
  else
    log_warn "faltando: chaves no ssh-agent"
    install_keys_loaded
  fi
}

action_sysup() {
  log_step "sysup (primeira instalação)"
  # shellcheck source=ricing/shell/lib/install-sysup.sh
  source "$REPO_DIR/ricing/shell/lib/install-sysup.sh"
  install_sysup
}

action_claude() {
  log_step "Claude Code (skills/rules/RTK.md/settings.json/filtros rtk)"
  bash "$REPO_DIR/claude/install.sh"
}

gum style --bold --foreground 212 --border rounded --padding "0 2" --margin "1 0" \
  "install-menu.sh — setup de ambiente"

choice="$(gum choose \
  --header "O que instalar/configurar? (setas + enter escolhe 1; rode de novo pra outra opção)" \
  --cursor.foreground 212 --selected.foreground 42 \
  "Shell (.bashrc/fish)" \
  "kitty (symlink)" \
  "Tema KDE (Layan)" \
  "fastfetch (config gerado + symlink)" \
  "starship (symlink tema)" \
  "Git (assinatura SSH + ssh-agent)" \
  "sysup (primeira instalação)" \
  "Claude Code (skills/rules/configs)")"

if [[ -z "$choice" ]]; then
  log_dim "nada selecionado, saindo."
  exit 0
fi

gum confirm "Aplicar \"$choice\" agora?" || {
  log_dim "cancelado."
  exit 0
}

echo
case "$choice" in
  "Shell (.bashrc/fish)") action_dotfiles ;;
  "kitty (symlink)") action_kitty ;;
  "Tema KDE (Layan)") action_kde_theme ;;
  "fastfetch (config gerado + symlink)") action_fastfetch ;;
  "starship (symlink tema)") action_starship ;;
  "Git (assinatura SSH + ssh-agent)") action_git ;;
  "sysup (primeira instalação)") action_sysup ;;
  "Claude Code (skills/rules/configs)") action_claude ;;
esac

echo
log_box "Concluído: $choice" 42

# Recarrega o shell sozinho: como este script roda como processo filho, não
# dá pra dar "source ~/.bashrc" no shell que te chamou daqui de dentro — o
# jeito é substituir este processo por um shell novo (exec), que já nasce
# lendo o .bashrc/config.fish atualizado. No Windows isso não faz sentido: o
# script já roda dentro do Git Bash chamado pelo install-menu.ps1 a partir do
# PowerShell, e um exec aqui troca o processo por outro Git Bash em vez de
# voltar pro PowerShell de quem chamou.
if ! is_windows; then
  log_dim "recarregando shell..."
  exec "${SHELL:-/bin/bash}" -l
fi
