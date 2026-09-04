#!/usr/bin/env bash
# Sourced por install-menu.sh, ricing/shell/install.sh e lib/install-shell-tools.sh.
# Fonte única de is_windows() e backup_and_link() — antes cada script tinha a
# própria cópia e só o do install-menu conhecia a armadilha do symlink no
# Windows.
#
# ARMADILHA (custou uma sessão inteira de debug): no Git Bash/MSYS2, `ln -s`
# NÃO cria symlink por padrão. Sem a variável MSYS=winsymlinks:*, o MSYS2
# COPIA o arquivo e sai com status 0 — parece que funcionou, o log diz
# "dst -> src", e o dst fica um snapshot congelado do repo. Toda edição
# posterior no repo não aparece na máquina, e nada no output denuncia isso.
#
# Fix em três camadas, nesta ordem:
#   1. MSYS=winsymlinks:nativestrict — faz `ln -s` criar symlink NTFS de
#      verdade, e FALHAR (status != 0) em vez de copiar quando não pode.
#   2. fallback via PowerShell New-Item -ItemType SymbolicLink.
#   3. verificação obrigatória com `[[ -L ]]` depois de criar — se o dst não
#      for symlink, é erro visível, nunca sucesso silencioso.
# Criar symlink no Windows exige Modo de Desenvolvedor ligado (Configurações →
# Sistema → Para desenvolvedores) ou terminal como administrador.

is_windows() { [[ "$(uname -s)" == MINGW* || "$(uname -s)" == MSYS* ]]; }

# nativestrict vale pro processo atual e pros filhos — precisa estar setado
# ANTES de qualquer `ln -s`.
if is_windows; then
  export MSYS="${MSYS:+$MSYS }winsymlinks:nativestrict"
fi

# _link_via_powershell src dst — fallback quando `ln -s` falha no Windows.
# Usa cygpath pra traduzir os paths POSIX do Git Bash pra path NTFS.
_link_via_powershell() {
  local src="$1" dst="$2" src_win dst_win ps_bin
  command -v cygpath >/dev/null 2>&1 || return 1
  ps_bin="$(command -v pwsh || command -v powershell || true)"
  [[ -n "$ps_bin" ]] || return 1

  src_win="$(cygpath -w "$src")"
  dst_win="$(cygpath -w "$dst")"
  "$ps_bin" -NoProfile -NonInteractive -Command \
    "New-Item -ItemType SymbolicLink -Path '$dst_win' -Target '$src_win' -Force | Out-Null" \
    >/dev/null 2>&1
}

# _symlink src dst — cria o symlink e garante que virou symlink de verdade.
# Retorna 1 (sem criar nada por baixo do pano) se não deu.
_symlink() {
  local src="$1" dst="$2"

  if ! ln -sfn "$src" "$dst" 2>/dev/null; then
    if ! is_windows || ! _link_via_powershell "$src" "$dst"; then
      log_warn "não consegui criar symlink: $dst"
      is_windows && log_dim "ligue o Modo de Desenvolvedor (Configurações → Sistema → Para desenvolvedores) ou rode como administrador"
      return 1
    fi
  fi

  # Sem esta checagem, um `ln` que copiou em vez de linkar passa como sucesso.
  if [[ ! -L "$dst" ]]; then
    log_warn "$dst foi CRIADO COMO CÓPIA, não symlink — mudanças no repo não vão chegar aqui"
    is_windows && log_dim "ligue o Modo de Desenvolvedor (Configurações → Sistema → Para desenvolvedores) ou rode como administrador"
    return 1
  fi
  return 0
}

# backup_and_link src dst — idempotente:
#   - dst já é symlink pro src         -> nada a fazer
#   - dst é cópia idêntica ao src      -> substitui (é a cópia-fallback de uma
#                                         execução anterior, não dado do usuário)
#   - dst é arquivo/dir com conteúdo   -> backup pra dst.bak
backup_and_link() {
  local src="$1" dst="$2"

  if [[ ! -e "$src" ]]; then
    log_warn "origem não existe, pulando: $src"
    return 1
  fi

  if [[ -L "$dst" ]] && [[ "$(readlink -f "$dst")" == "$(readlink -f "$src")" ]]; then
    log_ok "$dst -> $src (já linkado)"
    return 0
  fi

  if [[ -e "$dst" && ! -L "$dst" ]]; then
    if diff -rq "$src" "$dst" >/dev/null 2>&1; then
      rm -rf "$dst"
    else
      log_warn "backup: $dst -> $dst.bak"
      rm -rf "$dst.bak"
      mv "$dst" "$dst.bak"
    fi
  fi

  mkdir -p "$(dirname "$dst")"
  _symlink "$src" "$dst" || return 1
  log_ok "$dst -> $src"
}
