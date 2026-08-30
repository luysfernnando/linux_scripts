# CLAUDE.md

Guidance for Claude Code neste repo.

## Purpose

Scripts Linux pessoais (Arch/Debian) — setup de ambiente e ricing. Standalone, sem build/test suite (exceto `sysup/`, ver seção própria).

```bash
chmod +x <script>.sh && ./<script>.sh
```

## Scripts

| Script | Faz |
|---|---|
| `docker/install-docker.sh` | apt update → get.docker.com → add user ao grupo docker → reboot |
| `docker/postgres/docker-compose.yml` | Postgres local (2GB shared_buffers, NVMe) |
| `install-menu.sh` | Menu interativo (`gum`) que instala os pedaços abaixo numa máquina nova |

GitKraken/Tidewave: portados pro sysup (Rust), ver `sysup gitkraken`/`sysup tidewave`.

**Convenções:** bash com `set -euo pipefail` + `need()` guard; Postgres compose usa volume nomeado `pgdata` (persiste no `docker compose down`).

## Ricing — tema atual

KDE Plasma 6 + kitty + fish/starship. `ricing/README.md` tem os comandos de restore completos.

| Item | Valor/Tema | Path no repo | Symlink real? |
|---|---|---|---|
| KDE Look and Feel | Layan (`com.github.vinceliuice.Layan`) | — (via `kwriteconfig6`) | Não, aplicado por comando |
| KDE ícones | Papirus-Dark | — | Não |
| KDE cursor | Layan-white-cursors | — | Não |
| KDE decoração | Aurorae Layan | — | Não |
| KDE estilo de app | Kvantum-Dark (engine Kvantum, tema `KvMojave`) | — (`~/.config/Kvantum/kvantum.kvconfig`) | Não |
| kitty | tema "Idle Toes", fundo transparente 0.8 | `ricing/terminal/kitty/` | Sim |
| shell | bash/fish, mesmos aliases | `ricing/shell/{bash,fish}/` | Sim (bash); fish é `source`ado |
| lsd (`ls`) | mesma paleta do starship (degradê roxo→azul), pastas primeiro alfabético | `ricing/shell/lsd/{config.yaml,colors.yaml}` (metadado) + `LS_COLORS` (nome/ext) duplicado em `{bash,fish}/` — export difere por shell | Sim |
| prompt | Starship, preset `gruvbox-rainbow` (bash+fish, tema não exclusivo de shell) | `ricing/shell/starship/linux.toml` | Sim (`~/.config/starship.toml`) |
| fastfetch | logo `images/165.png`, presets de terceiros | `ricing/fastfetch/` (`config.jsonc.tmpl`, `images/`, `presets/`) | `images/`/`presets/` sim; `config.jsonc` não — gerado do `.tmpl` (placeholder `@LOGO_TYPE@` → `sixel` se WSL, `kitty` se nativo; ver `install-menu.sh`) |
| GRUB | boot silencioso (sem menu/mensagens) | `ricing/grub/silent-boot.sh` | Não, script `sudo` |
| git | assinatura SSH (`gpg.format=ssh`) | `ricing/shell/git/.gitconfig` | Sim |
| WezTerm (Windows) | Catppuccin Mocha, WebGpu, tab bar embaixo | `ricing/terminal/wezterm/wezterm.lua` | Não, copiado |
| PowerShell/Starship (Windows) | prompt Starship + `Terminal-Icons` | `ricing/shell/powershell/` (profile) + `ricing/shell/starship/windows.toml` (tema minimalista, drive de rede) | Não, copiado |

`shell/git/.gitconfig` symlinkado pro `~/.gitconfig` via `install.sh`. Assinatura de commits via SSH (`gpg.format=ssh`, chave `~/.ssh/luysfernnando_sign_commits` — path fixo em toda máquina, keypair distinto por máquina). `allowed_signers` versionado (`shell/git/allowed_signers`, symlink igual) acumula a pubkey de cada máquina — setup em `ricing/README.md`.

`install-menu.sh` automatiza tudo acima (shell, kitty, tema KDE, fastfetch, starship, sysup); requer `gum`. `ricing/shell/install.sh` faz o setup de shell isolado (só bash/fish — zsh saiu do repo) + instala `sysup` (baixa release do GitHub, ou `cargo build` como fallback) + symlinka `lsd/{config.yaml,colors.yaml}` (independe do shell detectado). Garante binários dos aliases e `starship` (`lsd`, `fzf`, `starship` — via `pacman`/`apt`, `ricing/shell/lib/install-cli-tools.sh` + `install-shell-tools.sh`); antes de instalar starship, detecta oh-my-posh e pergunta se quer desinstalar. `mise` (elixir/erlang/etc) ativado em `.bashrc`/`config.fish` se o binário já existir — instalação não é feita por este script. Tudo idempotente, checa antes de instalar.

## sysup — engine de update (Rust, cross-distro)

Binário único: `sysup update|mirrors|schedule|gitkraken|tidewave|polkit-setup [--dry-run]`. Standalone (não depende do repo clonado).

| Subcomando | Faz |
|---|---|
| `update` | Detecta distro/SO, roda pipeline paralelo (pacotes, flatpak, composer, npm, bun, firmware) + serial (órfãos, cache) numa TUI (Bubble Tea) |
| `mirrors` | Reranqueia mirrors (reflector/cachyos-rate-mirrors), re-roda sozinho a cada 7 dias |
| `schedule` | Instala agendador nativo (systemd timer/launchd/schtasks) pros mirrors |
| `gitkraken` / `tidewave` | Install/update desses apps (sem checagem de versão, subcomando explícito) |
| `polkit-setup` | Configura elevação via polkit — 1 prompt por run em vez de repetir `sudo` |

**Privilégio:** com `polkit-setup` rodado, `update` autoriza um único `pkexec sysup-worker` no início (antes da TUI); worker vive só durante o run, valida comandos por whitelist exata (sem `sh -c`). paru é redirecionado pro worker via `paru.conf`; yay não suporta isso e continua chamando `sudo` (2º prompt). Sem `polkit-setup`, cai pro `sudo -v` clássico. Detalhe/trade-offs completos em `sysup/README.md`.

Layout por módulo (workspace Cargo: `sysup/cli`/`sysup/worker`/`sysup/ipc`), release e detalhe completo do worker de privilégio: `sysup/README.md`.

**Testes:** `./sysup/check.sh` antes de commit em `sysup/` (fmt+clippy+test, silencioso no sucesso; self-update testado contra mock HTTP local, nunca rede real). Detalhe: `sysup/README.md`.

## Claude Skills & Rules

`claude/skills/` e `claude/rules/` (`common/`+`rust/`) sincronizados via `claude/install.sh` pra `~/.agents/skills/`/`~/.claude/skills/` e `~/.claude/rules/`. Nova skill/rule + sync: ver `claude/README.md`.

## Gotchas conhecidos

| Problema | Causa | Fix |
|---|---|---|
| Ctrl+V não cola imagem no Claude Code (kitty+Wayland+KDE) | `wl-clipboard` ausente, ou kitty remapeando Ctrl+V, ou Spectacle copiando path em vez de imagem | `sudo pacman -S wl-clipboard`; comentar `map ctrl+v paste_from_clipboard` no `kitty.conf`; atalho global KDE rodando `spectacle -b -r -n -c` direto (ignora config do GUI). Diagnóstico: `wl-paste --list-types` deve listar `image/png` |

**kitty.conf bindings custom:** `ctrl+shift+t` = nova aba no cwd; `ctrl+v` (paste) comentado — não reativar sem ler gotcha acima; Ctrl+C/Ctrl+Shift+C invertidos (copiar = Ctrl+C, SIGINT = Ctrl+Shift+C).

## Adding New Scripts

Subdiretório por categoria. `need()` guard + `set -euo pipefail` no topo.
