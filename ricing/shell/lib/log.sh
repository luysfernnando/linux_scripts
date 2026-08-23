#!/usr/bin/env bash
# Sourced (not executed directly) — helpers de output colorido/consistente
# pro install-menu.sh e ricing/shell/install.sh. Cai pra echo puro se gum
# não estiver disponível (ex: rodando install.sh direto sem menu).

_have_gum() { command -v gum >/dev/null 2>&1; }

log_step() { # título de seção/ação em destaque
  if _have_gum; then gum style --bold --foreground 212 "▶ $*"; else echo "==> $*"; fi
}

log_ok() { # ação concluída (link criado, instalação ok)
  if _have_gum; then gum style --foreground 42 "  ✓ $*"; else echo "  ok: $*"; fi
}

log_warn() { # aviso não-fatal (backup feito, pulando algo)
  if _have_gum; then gum style --foreground 214 "  ⚠ $*"; else echo "  aviso: $*"; fi
}

log_dim() { # info secundária (dicas, comandos)
  if _have_gum; then gum style --foreground 245 "  $*"; else echo "  $*"; fi
}

log_box() { # destaque final/importante, com borda
  if _have_gum; then
    gum style --border rounded --border-foreground "${2:-212}" --padding "0 2" "$1"
  else
    echo "== $1 =="
  fi
}

confirm() { # pergunta sim/não; gum confirm se tiver, senão read puro (y/N)
  if _have_gum; then
    gum confirm "$*"
  else
    local ans
    read -r -p "$* [y/N] " ans
    [[ "$ans" =~ ^[Yy]$ ]]
  fi
}
