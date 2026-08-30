#!/usr/bin/env bash
# Sourced (not executed directly) por ricing/shell/install.sh — binários
# usados nos aliases dos 2 shells (bash/fish) e pelo fastfetch:
# lsd (alias ls/l/la/lla/lstree), fzf (fuzzy finder de uso geral), imagemagick
# (fastfetch usa libMagickWand pra decodificar a imagem do logo — sem ela
# cai calado pro ASCII genérico, erro real só aparece com --show-errors).
# Cada item expõe check_<item> (só olha, sem efeito) e install_<item>
# (instala de fato) pra quem orquestra decidir o que rodar.

# pkg_install nome — instala pacote via pacman/apt (o que existir), com sudo.
pkg_install() {
  local pkg="$1"
  if command -v pacman >/dev/null 2>&1; then
    sudo pacman -S --needed --noconfirm "$pkg"
  elif command -v apt >/dev/null 2>&1; then
    sudo apt update -qq && sudo apt install -y "$pkg"
  else
    log_warn "gerenciador de pacotes não suportado (só pacman/apt) — instale '$pkg' manualmente."
    return 1
  fi
}

check_lsd() { command -v lsd >/dev/null 2>&1; }
install_lsd() {
  pkg_install lsd && log_ok "lsd instalado" || log_warn "falha instalando lsd — aliases ls/l/la/lla/lstree vão quebrar até instalar."
}

check_fzf() { command -v fzf >/dev/null 2>&1; }
install_fzf() {
  pkg_install fzf && log_ok "fzf instalado" || log_warn "falha instalando fzf."
}

check_imagemagick() { command -v magick >/dev/null 2>&1 || command -v convert >/dev/null 2>&1; }
install_imagemagick() {
  pkg_install imagemagick && log_ok "imagemagick instalado" || log_warn "falha instalando imagemagick — fastfetch não consegue decodificar a imagem do logo sem ele (erro real: 'Image Magick library not found')."
}
