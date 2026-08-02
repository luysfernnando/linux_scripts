#!/usr/bin/env bash
# Sourced (not executed directly) by ricing/shell/install.sh and install-menu.sh
# — instala/atualiza o binário `sysup` em ~/.local/bin, com fallback pra
# build local se não houver release pronta. Espera REPO_DIR já definido pelo
# script que dá source aqui (raiz do clone do repo).

SYSUP_REPO_SLUG="luysfernnando/linux_scripts"

install_sysup_from_release() {
  local os arch asset url tmpdir
  case "$(uname -s)" in
    Linux)  os=linux ;;
    Darwin) os=darwin ;;
    *) echo "SO não suportado pra release pronta: $(uname -s)"; return 1 ;;
  esac
  case "$(uname -m)" in
    x86_64|amd64) arch=amd64 ;;
    aarch64|arm64) arch=arm64 ;;
    *) echo "Arquitetura não suportada pra release pronta: $(uname -m)"; return 1 ;;
  esac

  asset="sysup_${os}_${arch}.tar.gz"
  url="https://github.com/$SYSUP_REPO_SLUG/releases/latest/download/$asset"
  tmpdir="$(mktemp -d)"

  echo "Baixando $url..."
  if ! curl -fsSL "$url" -o "$tmpdir/$asset"; then
    echo "Sem release disponível ainda (ou download falhou)."
    rm -rf "$tmpdir"
    return 1
  fi

  tar -xzf "$tmpdir/$asset" -C "$tmpdir" sysup
  command install -m 0755 "$tmpdir/sysup" "$HOME/.local/bin/sysup"
  rm -rf "$tmpdir"
  echo "sysup instalado (release) em $HOME/.local/bin/sysup"
}

install_sysup_from_source() {
  if ! command -v go >/dev/null 2>&1; then
    echo "AVISO: sem release disponível e go não encontrado — 'update'/'sysup' não vai funcionar até instalar Go e rodar:"
    echo "  cd $REPO_DIR/sysup && go build -o ~/.local/bin/sysup ./cmd/sysup"
    return 1
  fi
  echo "Compilando sysup a partir do fonte..."
  local tmpbin
  tmpbin="$(mktemp)"
  (cd "$REPO_DIR/sysup" && go build -o "$tmpbin" ./cmd/sysup)
  command install -m 0755 "$tmpbin" "$HOME/.local/bin/sysup"
  rm -f "$tmpbin"
  echo "sysup instalado (build local) em $HOME/.local/bin/sysup"
}

# install_sysup instala um binário real (não symlink) em ~/.local/bin/sysup,
# pra sobreviver mesmo se a pasta do repo for apagada depois. Prioridade:
# baixar o release pronto do GitHub (funciona sem precisar de Go instalado —
# é o caminho pros amigos); só builda do fonte se o download falhar e go
# estiver disponível.
install_sysup() {
  mkdir -p "$HOME/.local/bin"
  install_sysup_from_release || install_sysup_from_source || true

  # marca o clone do repo, pra `sysup update` conseguir dar um `git pull`
  # best-effort nos dotfiles (opcional, nunca bloqueia o update se falhar)
  mkdir -p "$HOME/.config/sysup"
  echo "$REPO_DIR" > "$HOME/.config/sysup/repo-path"

  if command -v sysup >/dev/null 2>&1 || [[ -x "$HOME/.local/bin/sysup" ]]; then
    echo "Dica: rode \`sysup polkit-setup\` pra evitar prompts de sudo repetidos no \`sysup update\` (Linux com polkit)."
  fi
}
