#!/usr/bin/env bash
# Sourced (não executado direto) — setup de assinatura de commit via SSH
# (gpg.format=ssh, ver ricing/shell/git/.gitconfig) e do ssh-agent persistente
# que o GitKraken/git CLI usam pra autenticação. Idempotente: cada passo
# checa antes de agir.
#
# SIGN_KEY tem o MESMO nome em toda máquina (keypair distinto por máquina,
# path idêntico) — é o que deixa signingKey fixo no .gitconfig compartilhado
# sem precisar de override local por máquina.

SIGN_KEY="$HOME/.ssh/luysfernnando_sign_commits"
ALLOWED_SIGNERS="$HOME/.ssh/allowed_signers"

check_sign_key() { [[ -f "$SIGN_KEY.pub" ]]; }

# gera chave de assinatura (não autenticação) se não existir. Precisa
# cadastrar a pubkey no GitHub depois (Settings → SSH keys → tipo "Signing
# Key", não "Authentication Key") — isso é manual, não dá pra automatizar.
install_sign_key() {
  local email
  email="$(git config --global user.email 2>/dev/null || true)"
  [[ -z "$email" ]] && email="$(gum input --placeholder "email pra chave de assinatura" 2>/dev/null || true)"
  [[ -z "$email" ]] && { log_warn "sem email, pulando geração da chave."; return 1; }

  ssh-keygen -t ed25519 -f "$SIGN_KEY" -C "$email" -N "" >/dev/null
  log_ok "chave gerada: $SIGN_KEY.pub"
  echo "$email $(cat "$SIGN_KEY.pub")" > "$ALLOWED_SIGNERS"
  log_ok "$ALLOWED_SIGNERS atualizado"

  log_box "Cadastra a pubkey no GitHub agora" 214
  log_dim "github.com/settings/keys → New SSH key → tipo Signing Key (não Authentication)"
  log_dim "$(cat "$SIGN_KEY.pub")"
}

check_allowed_signers() { [[ -f "$ALLOWED_SIGNERS" ]] && grep -qF "$(cat "$SIGN_KEY.pub" 2>/dev/null)" "$ALLOWED_SIGNERS" 2>/dev/null; }

# ALLOWED_SIGNERS é symlink pro arquivo versionado no repo (ricing/shell/git/
# allowed_signers) — acumula (append), nunca sobrescreve, senão perde a
# chave de outras máquinas. Depois de rodar isso, commitar + dar push no
# arquivo pra outras máquinas confiarem na chave nova.
install_allowed_signers() {
  local email
  email="$(git config --global user.email 2>/dev/null || echo "$(whoami)@localhost")"
  mkdir -p "$(dirname "$ALLOWED_SIGNERS")"
  touch "$ALLOWED_SIGNERS"
  echo "$email $(cat "$SIGN_KEY.pub")" >> "$ALLOWED_SIGNERS"
  log_ok "$ALLOWED_SIGNERS atualizado — commit + push nesse arquivo pra outras máquinas confiarem"
}

# ssh-agent via socket-activation do systemd (sobrevive ao terminal fechar,
# ao contrário de "eval \$(ssh-agent)" solto no .zshrc) + export global via
# environment.d pra apps gráficos (GitKraken) herdarem SSH_AUTH_SOCK mesmo
# não sendo filhos do shell.
check_ssh_agent_socket() { systemctl --user is-enabled --quiet ssh-agent.socket 2>/dev/null; }

install_ssh_agent_socket() {
  systemctl --user enable --now ssh-agent.socket
  mkdir -p "$HOME/.config/environment.d"
  printf 'SSH_AUTH_SOCK=%s/ssh-agent.socket\n' "${XDG_RUNTIME_DIR:-/run/user/$(id -u)}" > "$HOME/.config/environment.d/ssh-agent.conf"
  log_ok "ssh-agent.socket habilitado + SSH_AUTH_SOCK exportado (relogar pra apps gráficos pegarem)"
}

# adiciona no agent toda chave privada em ~/.ssh que tenha uma .pub e ainda
# não esteja carregada — cobre luysfernnando_sign_commits e outras
# (ex: gitkraken_rsa) sem precisar listar nome por nome.
check_keys_loaded() {
  export SSH_AUTH_SOCK="${SSH_AUTH_SOCK:-${XDG_RUNTIME_DIR:-/run/user/$(id -u)}/ssh-agent.socket}"
  local loaded
  loaded="$(ssh-add -l 2>/dev/null || true)"
  [[ -z "$loaded" ]] && return 1
  local key
  for key in "$HOME"/.ssh/*.pub; do
    [[ -f "$key" ]] || continue
    local fp
    fp="$(ssh-keygen -lf "$key" 2>/dev/null | awk '{print $2}')"
    [[ -n "$fp" ]] && ! grep -qF "$fp" <<<"$loaded" && return 1
  done
  return 0
}

install_keys_loaded() {
  export SSH_AUTH_SOCK="${SSH_AUTH_SOCK:-${XDG_RUNTIME_DIR:-/run/user/$(id -u)}/ssh-agent.socket}"
  local pub key
  for pub in "$HOME"/.ssh/*.pub; do
    [[ -f "$pub" ]] || continue
    key="${pub%.pub}"
    [[ -f "$key" ]] || continue
    ssh-add "$key" 2>/dev/null && log_ok "adicionado ao agent: $key"
  done
}
