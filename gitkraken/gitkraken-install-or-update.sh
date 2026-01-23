#!/usr/bin/env bash
set -euo pipefail

URL="https://release.gitkraken.com/linux/gitkraken-amd64.tar.gz"
APP_DIR="/opt/gitkraken"
BIN_LINK="/usr/bin/gitkraken"

# Wrapper (pra abrir e/ou atualizar sob demanda)
WRAPPER_PATH="/usr/local/bin/gitkraken"
# caminho absoluto para este instalador (resolve links e caminhos relativos)
INSTALLER_PATH="$(readlink -f "${BASH_SOURCE[0]:-$0}")"
WRAPPER_UPDATE_SCRIPT="$INSTALLER_PATH"

need() { command -v "$1" >/dev/null 2>&1 || { echo "Falta: $1"; exit 1; }; }
need curl
need tar

tmpdir="$(mktemp -d)"
trap 'rm -rf "$tmpdir"' EXIT

echo "Baixando: $URL"
curl -fL "$URL" -o "$tmpdir/gitkraken.tar.gz"

echo "Extraindo..."
tar -xzf "$tmpdir/gitkraken.tar.gz" -C "$tmpdir"

src="$tmpdir/gitkraken"
[[ -d "$src" ]] || { echo "Formato inesperado: pasta 'gitkraken' não encontrada"; exit 1; }

echo "Instalando em $APP_DIR (requer sudo)..."
sudo rm -rf "${APP_DIR}.new" "${APP_DIR}.old"
sudo cp -a "$src" "${APP_DIR}.new"

# swap seguro
if sudo test -d "$APP_DIR"; then
  sudo mv "$APP_DIR" "${APP_DIR}.old"
fi
sudo mv "${APP_DIR}.new" "$APP_DIR"
sudo rm -rf "${APP_DIR}.old"

echo "Criando symlink $BIN_LINK -> $APP_DIR/gitkraken (requer sudo)..."
sudo ln -sf "$APP_DIR/gitkraken" "$BIN_LINK"

# --- Wrapper auto-create (se não existir) ---
if ! sudo test -x "$WRAPPER_PATH"; then
  echo "Criando wrapper em $WRAPPER_PATH (requer sudo)..."
  sudo install -d -m 0755 "$(dirname "$WRAPPER_PATH")"

  sudo tee "$WRAPPER_PATH" >/dev/null <<SH
#!/usr/bin/env bash
set -euo pipefail

# Se quiser atualizar sempre ao abrir, deixe UPDATE_ON_START=1 no ambiente.
UPDATE_ON_START="\${UPDATE_ON_START:-0}"
UPDATER="$WRAPPER_UPDATE_SCRIPT"
BIN="$APP_DIR/gitkraken"

if [[ "\$UPDATE_ON_START" == "1" && -x "\$UPDATER" ]]; then
  "\$UPDATER" >/dev/null 2>&1 || true
fi

exec "\$BIN" "\$@"
SH

  sudo chmod 0755 "$WRAPPER_PATH"
fi

echo "OK: GitKraken atualizado."
echo "Abrir: gitkraken"
echo "Atualizar ao abrir (opcional): UPDATE_ON_START=1 gitkraken"
