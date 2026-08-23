#!/usr/bin/env bash
# Sourced (not executed directly) por ricing/shell/install.sh — oh-my-zsh,
# plugins usados em zsh/.zshrc e starship. Cada item expõe check_<item>
# (só olha, sem efeito) e install_<item> (instala de fato) pra quem
# orquestra decidir o que rodar.

ZSH_CUSTOM="${ZSH_CUSTOM:-$HOME/.oh-my-zsh/custom}"

# Path do próprio lib (não do chamador) — pra achar o tema independente de
# quem deu source (install.sh usa SHELL_DIR, install-menu.sh usa REPO_DIR).
_ZSH_LIB_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
STARSHIP_THEME_SRC="$_ZSH_LIB_DIR/../starship/linux.toml"

check_ohmyzsh() { [[ -d "$HOME/.oh-my-zsh" ]]; }
install_ohmyzsh() {
  need curl || return 1
  local installer
  if ! installer="$(curl -fsSL https://raw.githubusercontent.com/ohmyzsh/ohmyzsh/master/tools/install.sh)"; then
    log_warn "falha baixando instalador do oh-my-zsh (sem rede?)."
    return 1
  fi
  if RUNZSH=no CHSH=no KEEP_ZSHRC=yes sh -c "$installer"; then
    log_ok "oh-my-zsh instalado"
  else
    log_warn "falha instalando oh-my-zsh."
    return 1
  fi
}

# URLs dos plugins usados em zsh/.zshrc (ver plugins=(...) lá).
declare -A ZSH_PLUGIN_URL=(
  [zsh-autosuggestions]=https://github.com/zsh-users/zsh-autosuggestions
  [fast-syntax-highlighting]=https://github.com/zdharma-continuum/fast-syntax-highlighting
  [zsh-completions]=https://github.com/zsh-users/zsh-completions
  [fzf-tab]=https://github.com/Aloxaf/fzf-tab
)

check_zsh_plugin() { [[ -d "$ZSH_CUSTOM/plugins/$1" ]]; }
install_zsh_plugin() {
  local name="$1"
  need git || return 1
  git clone --depth 1 "${ZSH_PLUGIN_URL[$name]}" "$ZSH_CUSTOM/plugins/$name" >/dev/null 2>&1 \
    && log_ok "plugin $name instalado" || log_warn "falha instalando plugin $name."
}

# check_starship olha binário E tema — sem o tema o prompt cai pro padrão
# do starship (preset genérico), não no nosso.
check_starship() {
  command -v starship >/dev/null 2>&1 && [[ -f "$HOME/.config/starship.toml" ]]
}
install_starship() {
  if ! command -v starship >/dev/null 2>&1; then
    if command -v pacman >/dev/null 2>&1; then
      pkg_install starship
    else
      need curl || return 1
      mkdir -p "$HOME/.local/bin"
      if ! curl -fsSL https://starship.rs/install.sh | sh -s -- -y -b "$HOME/.local/bin"; then
        log_warn "falha instalando starship — instale manualmente: https://starship.rs/guide/#step-1-install-starship"
        return 1
      fi
    fi
    command -v starship >/dev/null 2>&1 && log_ok "starship instalado"
  fi

  # tema sempre por último — só symlinka depois de confirmar o binário em pé.
  # Symlink (não copy): tema é igual em toda máquina, sem valor
  # machine-specific — diferente do config.jsonc do fastfetch.
  mkdir -p "$HOME/.config"
  ln -sf "$STARSHIP_THEME_SRC" "$HOME/.config/starship.toml"
  log_ok "~/.config/starship.toml -> $STARSHIP_THEME_SRC"
}

# ---------------------------
# Gerenciadores de tema de shell antigos (oh-my-posh, powerlevel10k) —
# detecta e oferece desinstalar antes de instalar starship, pra não ficar
# lixo (binário + init eval concorrente) na máquina. _pkg_owner descobre se
# o pacote veio de pacman/apt (pra desinstalar certo); se não veio de
# nenhum, assume instalação manual (binário solto / clone git) e remove o
# arquivo direto.
# ---------------------------
_pkg_owner() {
  local pkg="$1"
  if command -v pacman >/dev/null 2>&1 && pacman -Qq "$pkg" >/dev/null 2>&1; then
    echo pacman; return 0
  fi
  if command -v dpkg >/dev/null 2>&1 && dpkg -s "$pkg" >/dev/null 2>&1; then
    echo apt; return 0
  fi
  return 1
}

_pkg_uninstall() {
  local mgr="$1" pkg="$2"
  case "$mgr" in
    pacman) sudo pacman -Rns --noconfirm "$pkg" ;;
    apt)    sudo apt remove -y "$pkg" ;;
  esac
}

detect_and_offer_uninstall_prompt_managers() {
  local found=()

  if command -v oh-my-posh >/dev/null 2>&1 || [[ -e "$HOME/.local/bin/oh-my-posh" ]]; then
    found+=(oh-my-posh)
  fi
  if [[ -d "$ZSH_CUSTOM/themes/powerlevel10k" ]] || grep -q 'powerlevel10k' "$HOME/.zshrc" 2>/dev/null; then
    found+=(powerlevel10k)
  fi

  [[ ${#found[@]} -eq 0 ]] && return 0

  log_step "Gerenciador(es) de tema antigo detectado: ${found[*]}"
  if ! confirm "Desinstalar ${found[*]} (trocando por starship)?"; then
    log_dim "mantendo — só instalando starship por cima (pode conflitar no prompt)."
    return 0
  fi

  local mgr pm
  for mgr in "${found[@]}"; do
    case "$mgr" in
      oh-my-posh)
        if pm="$(_pkg_owner oh-my-posh)"; then
          _pkg_uninstall "$pm" oh-my-posh
        else
          rm -f "$HOME/.local/bin/oh-my-posh"
        fi
        rm -rf "$HOME/.poshthemes"
        log_ok "oh-my-posh removido"
        ;;
      powerlevel10k)
        if pm="$(_pkg_owner zsh-theme-powerlevel10k-git)"; then
          _pkg_uninstall "$pm" zsh-theme-powerlevel10k-git
        else
          rm -rf "$ZSH_CUSTOM/themes/powerlevel10k"
        fi
        log_ok "powerlevel10k removido (remova ZSH_THEME=\"powerlevel10k/powerlevel10k\" do .zshrc se sobrou)"
        ;;
    esac
  done
}
