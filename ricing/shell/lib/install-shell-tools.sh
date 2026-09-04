#!/usr/bin/env bash
# Sourced (não executado direto) por ricing/shell/install.sh e install-menu.sh —
# starship (prompt, usado por bash e fish) + detecção de gerenciador de tema
# antigo (oh-my-posh) que precisa sair antes do starship entrar.

_SHELL_TOOLS_LIB_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# is_windows() + backup_and_link() — sourced (não redefinidas) porque este
# arquivo é sourced tanto por install-menu.sh quanto por
# ricing/shell/install.sh. Guard de idempotência: os dois já podem ter
# sourcado link.sh antes.
if ! declare -F backup_and_link >/dev/null 2>&1; then
  source "$_SHELL_TOOLS_LIB_DIR/link.sh"
fi

# check_starship olha binário E tema, e exige que o tema seja SYMLINK: um
# ~/.config/starship.toml comum é uma cópia congelada (ver a armadilha do
# MSYS em lib/link.sh) — edição no repo nunca chega no prompt. Tratar cópia
# como "instalado" foi exatamente o que esconde esse bug, então aqui ela
# conta como não-instalado e a re-execução conserta.
check_starship() {
  command -v starship >/dev/null 2>&1 && [[ -L "$HOME/.config/starship.toml" ]]
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
  # machine-specific — diferente do config.jsonc do fastfetch. windows.toml é
  # separado do linux.toml (módulos de versão de linguagem desligados — drive
  # de rede lento), então a escolha do arquivo depende do SO.
  local theme_src="$_SHELL_TOOLS_LIB_DIR/../starship/linux.toml"
  is_windows && theme_src="$_SHELL_TOOLS_LIB_DIR/../starship/windows.toml"
  backup_and_link "$theme_src" "$HOME/.config/starship.toml"

  is_windows && install_powershell_profile
}

# install_powershell_profile — README documentava isso como passo manual
# (Copy-Item); aqui vira parte do fluxo automático do starship no Windows.
# Symlink em vez de copy: perfil não tem nada machine-specific.
install_powershell_profile() {
  local ps_profile_src="$_SHELL_TOOLS_LIB_DIR/../powershell/Microsoft.PowerShell_profile.ps1"
  local ps_profile_dst="$HOME/Documents/PowerShell/Microsoft.PowerShell_profile.ps1"
  local pwsh_bin

  backup_and_link "$ps_profile_src" "$ps_profile_dst"

  pwsh_bin="$(command -v pwsh || true)"
  if [[ -z "$pwsh_bin" ]]; then
    log_warn "pwsh não encontrado — instale o módulo Terminal-Icons manualmente: Install-Module Terminal-Icons -Scope CurrentUser"
    return 0
  fi
  if "$pwsh_bin" -NoProfile -Command "if (Get-Module -ListAvailable Terminal-Icons) { exit 0 } else { exit 1 }" >/dev/null 2>&1; then
    log_ok "módulo Terminal-Icons (PowerShell)"
  else
    log_step "instalando módulo Terminal-Icons (PowerShell)..."
    if "$pwsh_bin" -NoProfile -Command "Install-Module Terminal-Icons -Scope CurrentUser -Force -Repository PSGallery"; then
      log_ok "Terminal-Icons instalado"
    else
      log_warn "falha instalando Terminal-Icons — instale manualmente: Install-Module Terminal-Icons -Scope CurrentUser"
    fi
  fi
}

# ---------------------------
# Gerenciador de tema de shell antigo (oh-my-posh) — detecta e oferece
# desinstalar antes de instalar starship, pra não ficar lixo (binário + init
# eval concorrente) na máquina. _pkg_owner descobre se o pacote veio de
# pacman/apt (pra desinstalar certo); se não veio de nenhum, assume
# instalação manual (binário solto) e remove o arquivo direto.
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
    esac
  done
}
