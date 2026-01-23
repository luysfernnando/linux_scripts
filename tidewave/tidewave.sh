#!/usr/bin/env bash
set -euo pipefail

# Tidewave - instalador / wrapper / utilitário
export PATH="/usr/local/bin:/usr/bin:/bin:/usr/sbin:$HOME/.local/bin:$PATH"
export XDG_CACHE_HOME="${XDG_CACHE_HOME:-$HOME/.cache}"

DEFAULT_PORT=8000
APP_BIN_DIR="$HOME/.local/bin"
CLI_BIN="$APP_BIN_DIR/tidewave-cli"

SELF_PATH="$(readlink -f "${BASH_SOURCE[0]:-$0}")"

need() { command -v "$1" >/dev/null 2>&1 || { echo "Falta: $1"; exit 1; } }

cmd_install() {
  mkdir -p "$APP_BIN_DIR"
  install -m 0755 "$SELF_PATH" "$APP_BIN_DIR/tidewave"
  echo "OK: wrapper instalado em $APP_BIN_DIR/tidewave"
}

cmd_update() {
  # se existir script auxiliar, delega
  if command -v tidewave-update >/dev/null 2>&1; then
    tidewave-update
    return
  fi

  need curl
  url="https://github.com/tidewave-ai/tidewave_app/releases/latest/download/tidewave-cli-x86_64-unknown-linux-gnu"
  tmp="$(mktemp)"
  echo "Baixando: $url"
  curl -fL "$url" -o "$tmp"
  mkdir -p "$APP_BIN_DIR"
  install -m 0755 "$tmp" "$CLI_BIN"
  rm -f "$tmp"
  echo "OK: $CLI_BIN atualizado."
}

cmd_fix_codex_acp() {
  if command -v tidewave-fix-codex-acp >/dev/null 2>&1; then
    tidewave-fix-codex-acp
    return
  fi

  acp="$(command -v codex-acp || true)"
  if [[ -z "$acp" ]]; then
    echo "codex-acp não encontrado no PATH"; return 1
  fi
  downloads="${XDG_CACHE_HOME:-$HOME/.cache}/tidewave/downloads"
  mkdir -p "$downloads"
  for f in "$downloads"/codex-acp-linux-x64-*; do
    [[ -e "$f" ]] || continue
    ln -sf "$acp" "$f"
  done
  ln -sf "$acp" "$downloads/codex-acp-linux-x64-0-8-2" 2>/dev/null || true
  echo "OK: codex-acp linkado em $downloads"
}

default_run() {
  # Auto-update (silencioso) e fix (silencioso)
  if command -v tidewave-update >/dev/null 2>&1; then
    tidewave-update >/dev/null 2>&1 || true
  else
    # tenta atualização local mínima sem barulho
    if command -v curl >/dev/null 2>&1; then
      cmd_update >/dev/null 2>&1 || true
    fi
  fi

  if command -v tidewave-fix-codex-acp >/dev/null 2>&1; then
    tidewave-fix-codex-acp >/dev/null 2>&1 || true
  else
    cmd_fix_codex_acp >/dev/null 2>&1 || true
  fi

  # garante CLI presente
  if [[ ! -x "$CLI_BIN" ]]; then
    echo "tidewave-cli não encontrado. Executando update..."
    cmd_update
  fi

  # injeta porta padrão se não passada
  ARGS=("$@")
  HAS_PORT=false
  for arg in "${ARGS[@]}"; do
    [[ "$arg" == "-p" || "$arg" == "--port" ]] && HAS_PORT=true && break
  done
  if [[ "$HAS_PORT" == false ]]; then
    ARGS=(-p "$DEFAULT_PORT" "${ARGS[@]}")
  fi

  exec "$CLI_BIN" "${ARGS[@]}"
}

usage() {
  cat <<EOF
Uso: tidewave [comando] [args]

Comandos:
  install         Instala o wrapper em $APP_BIN_DIR/tidewave
  update          Atualiza o binário tidewave-cli
  fix-codex-acp   Linka/arruma o codex-acp no cache
  help            Mostra esta ajuda

Sem comando, executa o Tidewave (com update/fix silenciosos)
EOF
}

case "${1:-}" in
  install)
    cmd_install
    exit 0
    ;;
  update)
    cmd_update
    exit 0
    ;;
  fix-codex-acp)
    cmd_fix_codex_acp
    exit 0
    ;;
  help|-h|--help)
    usage
    exit 0
    ;;
  "")
    default_run "${@:1}"
    ;;
  *)
    # Se primeiro argumento não for comando conhecido, assume execução do CLI
    default_run "${@:1}"
    ;;
esac
