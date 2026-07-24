#!/usr/bin/env bash
set -euo pipefail

DOTFILES_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_DIR="$(cd "$DOTFILES_DIR/.." && pwd)"

# .gitconfig fica de fora: a cópia no repo está desatualizada (assinatura
# GPG antiga) em relação ao que está em uso de verdade (assinatura SSH) —
# symlinkar ela por cima quebraria commits assinados. Reative aqui só depois
# de atualizar dotfiles/.gitconfig pra bater com o ~/.gitconfig real.
FILES=(.zshrc .bashrc)

# backup_and_link src_relative_to_dotfiles dst_absolute
backup_and_link() {
  local src="$DOTFILES_DIR/$1"
  local dst="$2"

  if [[ -e "$dst" && ! -L "$dst" ]]; then
    echo "Backup: $dst -> $dst.bak"
    mv "$dst" "$dst.bak"
  fi

  mkdir -p "$(dirname "$dst")"
  ln -sf "$src" "$dst"
  echo "Linked: $dst -> $src"
}

for f in "${FILES[@]}"; do
  backup_and_link "$f" "$HOME/$f"
done

# ---------------------------
# Fish (só se instalado)
# ---------------------------
if command -v fish >/dev/null 2>&1; then
  FISH_CONFIG="$HOME/.config/fish/config.fish"
  SOURCE_LINE="source $DOTFILES_DIR/fish/config.fish"

  mkdir -p "$HOME/.config/fish"
  touch "$FISH_CONFIG"

  if ! grep -qF "$SOURCE_LINE" "$FISH_CONFIG"; then
    printf '\n# linux_scripts dotfiles (aliases/update)\n%s\n' "$SOURCE_LINE" >> "$FISH_CONFIG"
    echo "Adicionado ao $FISH_CONFIG: $SOURCE_LINE"
  else
    echo "Fish já configurado: $FISH_CONFIG"
  fi
else
  echo "fish não encontrado, pulando config de fish"
fi

# ---------------------------
# sysup (engine de update cross-distro/OS, em Go)
# ---------------------------
# Instala um binário real (não symlink) em ~/.local/bin/sysup, pra sobreviver
# mesmo se a pasta do repo for apagada depois. Prioridade: baixar o release
# pronto do GitHub (funciona sem precisar de Go instalado — é o caminho pros
# amigos); só builda do fonte se o download falhar e go estiver disponível.
SYSUP_REPO_SLUG="luysfernnando/linux_scripts"
mkdir -p "$HOME/.local/bin"

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
  trap 'rm -rf "$tmpdir"' RETURN

  echo "Baixando $url..."
  if ! curl -fsSL "$url" -o "$tmpdir/$asset"; then
    echo "Sem release disponível ainda (ou download falhou)."
    return 1
  fi

  tar -xzf "$tmpdir/$asset" -C "$tmpdir" sysup
  install -m 0755 "$tmpdir/sysup" "$HOME/.local/bin/sysup"
  echo "sysup instalado (release) em $HOME/.local/bin/sysup"
}

install_sysup_from_source() {
  if ! command -v go >/dev/null 2>&1; then
    echo "AVISO: sem release disponível e go não encontrado — 'update'/'sysup' não vai funcionar até instalar Go e rodar:"
    echo "  cd $REPO_DIR/sysup && go build -o ~/.local/bin/sysup ."
    return 1
  fi
  echo "Compilando sysup a partir do fonte..."
  local tmpbin
  tmpbin="$(mktemp)"
  (cd "$REPO_DIR/sysup" && go build -o "$tmpbin" .)
  install -m 0755 "$tmpbin" "$HOME/.local/bin/sysup"
  rm -f "$tmpbin"
  echo "sysup instalado (build local) em $HOME/.local/bin/sysup"
}

install_sysup_from_release || install_sysup_from_source || true

# marca o clone do repo, pra `sysup update` conseguir dar um `git pull`
# best-effort nos dotfiles (opcional, nunca bloqueia o update se falhar)
mkdir -p "$HOME/.config/sysup"
echo "$REPO_DIR" > "$HOME/.config/sysup/repo-path"

# ---------------------------
# Reload certo pro shell atual
# ---------------------------
current_shell="$(basename "${SHELL:-}")"
case "$current_shell" in
  zsh)  echo "Done. Reload shell: source ~/.zshrc" ;;
  bash) echo "Done. Reload shell: source ~/.bashrc" ;;
  fish) echo "Done. Reload shell: exec fish" ;;
  *)    echo "Done. Reload shell manualmente (shell atual: $current_shell)" ;;
esac

echo "Pra manter os mirrors sempre bons em segundo plano, rode: sysup schedule"
