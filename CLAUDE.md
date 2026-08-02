# CLAUDE.md

Guidance for Claude Code neste repo.

## Purpose

Scripts Linux pessoais (Arch/Debian) — setup de ambiente e ricing. Standalone, sem build/test suite.

```bash
chmod +x <script>.sh && ./<script>.sh
```

## Scripts

| Script | Faz |
|---|---|
| `docker/install-docker.sh` | apt update → get.docker.com → add user ao grupo docker → reboot |
| `docker/postgres/docker-compose.yml` | Postgres local (2GB shared_buffers, NVMe) |
| `install-menu.sh` | Menu interativo (`gum`) que instala os pedaços abaixo numa máquina nova |

GitKraken/Tidewave: portados pro Go, ver `sysup gitkraken`/`sysup tidewave`.

**Convenções:** bash com `set -euo pipefail` + `need()` guard; Postgres compose usa volume nomeado `pgdata` (persiste no `docker compose down`).

## Ricing — tema atual

KDE Plasma 6 + kitty + zsh/oh-my-posh. `ricing/README.md` tem os comandos de restore completos.

| Item | Valor/Tema | Path no repo | Symlink real? |
|---|---|---|---|
| KDE Look and Feel | Layan (`com.github.vinceliuice.Layan`) | — (via `kwriteconfig6`) | Não, aplicado por comando |
| KDE ícones | Papirus-Dark | — | Não |
| KDE cursor | Layan-white-cursors | — | Não |
| KDE decoração | Aurorae Layan | — | Não |
| KDE estilo de app | Kvantum-Dark (engine Kvantum, tema `KvMojave`) | — (`~/.config/Kvantum/kvantum.kvconfig`) | Não |
| kitty | tema "Idle Toes", fundo transparente 0.8 | `ricing/terminal/kitty/` | Sim |
| shell | zsh/bash/fish, mesmos aliases | `ricing/shell/{zsh,bash,fish}/` | Sim (zsh/bash); fish é `source`ado |
| prompt | oh-my-posh, `p10k.omp.json` | `ricing/shell/zsh/tema/` | Não (copiado pra `~/.poshthemes/`) |
| fastfetch | logo `images/165.png`, presets de terceiros | `ricing/fastfetch/` (`config.jsonc`, `images/`, `presets/`) | Sim (pasta inteira) |
| GRUB | boot silencioso (sem menu/mensagens) | `ricing/grub/silent-boot.sh` | Não, script `sudo` |
| git | assinatura SSH (`gpg.format=ssh`) | `ricing/shell/.gitconfig` | Sim |

`shell/.gitconfig` symlinkado pro `~/.gitconfig` via `install.sh`. Assinatura de commits via SSH (`gpg.format=ssh`, chave dedicada `~/.ssh/id_luysfernnando_sign_commits`) — setup por máquina em `ricing/README.md`.

`install-menu.sh` automatiza tudo acima (shell, kitty, tema KDE, fastfetch, oh-my-posh, sysup); requer `gum`. `ricing/shell/install.sh` faz o setup de shell isolado + instala `sysup` (baixa release do GitHub, ou `go build` como fallback).

## sysup — engine de update (Go, cross-distro)

Binário único: `sysup update|mirrors|schedule|gitkraken|tidewave|polkit-setup [--dry-run]`. Standalone (não depende do repo clonado).

| Subcomando | Faz |
|---|---|
| `update` | Detecta distro/SO, roda pipeline paralelo (pacotes, flatpak, composer, npm, bun, firmware) + serial (órfãos, cache) numa TUI (Bubble Tea) |
| `mirrors` | Reranqueia mirrors (reflector/cachyos-rate-mirrors), re-roda sozinho a cada 7 dias |
| `schedule` | Instala agendador nativo (systemd timer/launchd/schtasks) pros mirrors |
| `gitkraken` / `tidewave` | Install/update desses apps (sem checagem de versão, subcomando explícito) |
| `polkit-setup` | Configura elevação via polkit — 1 prompt por run em vez de repetir `sudo` |

**Privilégio:** com `polkit-setup` rodado, `update` autoriza um único `pkexec sysup-worker` no início (antes da TUI); worker vive só durante o run, valida comandos por whitelist exata (sem `sh -c`). paru é redirecionado pro worker via `paru.conf`; yay não suporta isso e continua chamando `sudo` (2º prompt). Sem `polkit-setup`, cai pro `sudo -v` clássico. Detalhe/trade-offs completos em `sysup/README.md`.

Layout por pacote (`internal/*`), release e detalhe completo do worker de privilégio: `sysup/README.md`.

## Claude Skills

`claude/skills/` → `claude/install.sh` replica pra `~/.agents/skills/<nome>/` + symlink em `~/.claude/skills/<nome>`. Nova skill: criar `claude/skills/<nome>/SKILL.md`, rodar o install. Ver `claude/README.md`.

## Gotchas conhecidos

| Problema | Causa | Fix |
|---|---|---|
| Ctrl+V não cola imagem no Claude Code (kitty+Wayland+KDE) | `wl-clipboard` ausente, ou kitty remapeando Ctrl+V, ou Spectacle copiando path em vez de imagem | `sudo pacman -S wl-clipboard`; comentar `map ctrl+v paste_from_clipboard` no `kitty.conf`; atalho global KDE rodando `spectacle -b -r -n -c` direto (ignora config do GUI). Diagnóstico: `wl-paste --list-types` deve listar `image/png` |

**kitty.conf bindings custom:** `ctrl+shift+t` = nova aba no cwd; `ctrl+v` (paste) comentado — não reativar sem ler gotcha acima; Ctrl+C/Ctrl+Shift+C invertidos (copiar = Ctrl+C, SIGINT = Ctrl+Shift+C).

## Adding New Scripts

Subdiretório por categoria. `need()` guard + `set -euo pipefail` no topo.
