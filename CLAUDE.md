# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Purpose

Personal collection of Linux shell scripts for automating environment setup and tooling on Arch/Debian-based systems. Scripts are standalone — no build system, no test suite.

## Running Scripts

All scripts need execute permission before first use:

```bash
chmod +x <script>.sh
./<script>.sh
```

Read the top of each script for dependencies and options before running — most require `sudo` for system-level steps.

## Scripts Overview

| Script | What it does |
|---|---|
| `docker/install-docker.sh` | apt update → get.docker.com install → add user to docker group → reboot |
| `docker/postgres/docker-compose.yml` | Local Postgres tuned for heavy queries (2GB shared_buffers, NVMe settings) |
| `grub/silent-boot.sh` | Esconde menu do GRUB e mensagens de loading, silencia wall broadcast no shutdown, deixa o Plymouth assumir a tela. Precisa `sudo`. Re-rodar após updates do pacote `grub` (sobrescreve `/etc/grub.d/10_linux`). |

GitKraken e Tidewave foram portados pro Go — ver `sysup gitkraken` / `sysup tidewave` na seção `sysup` abaixo (os `.sh` antigos foram removidos).

## Conventions

- Bash scripts use `set -euo pipefail` and a `need()` guard for dependency checks.
- Postgres compose uses a named volume `pgdata` — data persists across `docker compose down`.

## Dotfiles

`dotfiles/` contém `.zshrc`, `.bashrc` (linkados via symlink pra `~/`) e `fish/config.fish` (aliases fish, `source`ado no fim do `~/.config/fish/config.fish` local — não symlinkado direto, porque `config.fish` local tem coisas específicas de máquina como brew/bun/cachyos). Os três shells têm o mesmo conjunto de aliases (nav, git, dev, lsd, serviços, tunnel, `tidewave`) e o mesmo `update`/`mirrors` (que só chamam o binário `sysup`).

`dotfiles/.gitconfig` existe no repo mas **não** é linkado pelo `install.sh` (não está em `FILES`) — está desatualizado (assinatura GPG antiga) em relação ao setup real (assinatura SSH). Não reative sem antes sincronizar o conteúdo com o `~/.gitconfig` de verdade.

**Setup em máquina nova:**
```bash
git clone <repo> ~/linux_scripts
cd ~/linux_scripts/dotfiles
bash install.sh
```

`install.sh` faz backup do arquivo original (`*.bak`) se não for symlink, depois cria o link. Linka `.zshrc`/`.bashrc`; se `fish` existir, garante a linha `source .../dotfiles/fish/config.fish` no `config.fish` local (idempotente). Instala `sysup` como **arquivo real** (não symlink) em `~/.local/bin/sysup`: primeiro tenta baixar o binário da última release do GitHub (funciona sem Go instalado — é o caminho pra quem só clona o repo numa máquina nova/de amigo); só compila do fonte (`go build`) como fallback, se o download falhar e `go` estiver disponível. Também grava `~/.config/sysup/repo-path` com o caminho do clone (usado pelo `sysup update` pra um `git pull` best-effort nos dotfiles). Adicionar novo dotfile pro bash/zsh: copiar pra `dotfiles/`, incluir o nome em `FILES` no `install.sh`. Pro fish: editar `dotfiles/fish/config.fish` direto.

## Ricing

`ricing/` guarda um snapshot do visual atual (KDE Plasma, kitty, oh-my-posh) pra restaurar depois numa reinstalação — não é symlinkado nem tocado pelo `dotfiles/install.sh` (diferente de `.zshrc`/`.bashrc`), é backup on-demand com passo a passo manual. Ver `ricing/README.md` pros comandos de restore (`kwriteconfig6`, `lookandfeeltool`, cópia de configs do kitty/oh-my-posh).

**Exceção:** `ricing/kitty/kitty.conf` **é** symlinkado — `~/.config/kitty/kitty.conf` aponta direto pra esse arquivo do repo (`ln -s`). Editar em qualquer um dos dois caminhos edita o mesmo arquivo; não precisa mais copiar manualmente pra atualizar o snapshot, só commitar. Setup em máquina nova: `ln -sf ~/linux_scripts/ricing/kitty/kitty.conf ~/.config/kitty/kitty.conf` (criar `~/.config/kitty/` antes se não existir).

Atualizar o snapshot: sobrescrever os arquivos em `ricing/` com o config atual da máquina e commitar. Adicionar novo item ricado (ex: outro terminal, outro tema): criar subpasta em `ricing/`, documentar restore no `ricing/README.md`.

## Claude Skills

`claude/skills/` versiona skills do Claude Code criadas por mim (ex: `token-efficient-docs`). `claude/install.sh` replica cada skill pra `~/.agents/skills/<nome>/` (fonte compartilhada entre agentes) e cria symlink em `~/.claude/skills/<nome>` (onde o Claude Code descobre skills) — mesmo padrão fonte-no-repo → symlink usado em `dotfiles/`, mas idempotente e sem depender do repo continuar clonado (a cópia em `~/.agents/skills` sobrevive independente do symlink). Adicionar skill nova: criar `claude/skills/<nome>/SKILL.md`, rodar `bash claude/install.sh`. Ver `claude/README.md`.

## Paste de imagem (Claude Code + kitty + KDE Wayland)

Ctrl+V colando imagem (print) direto no Claude Code precisou de 3 correções na máquina:

1. **`wl-clipboard` ausente** — Wayland não tem `xclip`/`xsel`; sem `wl-paste`/`wl-copy` instalado, kitty não acessa a área de transferência. `sudo pacman -S wl-clipboard`.
2. **kitty remapeava Ctrl+V** — `~/.config/kitty/kitty.conf` tinha `map ctrl+v paste_from_clipboard`, que faz o kitty interceptar a tecla e colar como texto puro antes do Claude Code ver a imagem binária. Comentado.
3. **Spectacle inconsistente entre GUI e atalho global** — a config gráfica ("copiar imagem" vs "copiar localização") nem sempre reflete no atalho do teclado. Fix: atalho global do KDE (Configurações do Sistema → Atalhos → Atalhos Personalizados) roda direto:
   ```bash
   spectacle -b -r -n -c
   ```
   (`-b` background, `-r` região, `-n` sem notificação, `-c` força copiar **imagem** pro clipboard, ignora a config salva do GUI).

Teste rápido de diagnóstico: `wl-paste --list-types` depois do print — precisa listar `image/png`. Se só listar `text/plain`/`STRING`, o Spectacle copiou path/texto, não a imagem.

## kitty.conf — bindings customizados

- `map ctrl+shift+t new_tab_with_cwd` — nova aba abre no path atual (default do kitty abre na home).
- `map ctrl+v paste_from_clipboard` fica **comentado** (ver seção de paste de imagem acima) — não reativar sem entender o motivo.
- `map ctrl+c copy_to_clipboard` / `map ctrl+shift+c send_text all \x03` — Ctrl+C e Ctrl+Shift+C invertidos (copiar vira Ctrl+C, cancelar/SIGINT vira Ctrl+Shift+C).

## sysup (engine de update cross-distro/cross-OS, self-updating)

`sysup/` é um módulo Go — binário único (`sysup update|mirrors|schedule|gitkraken|tidewave [--dry-run]`) que substitui os antigos scripts/aliases de update espalhados pelo repo. Distribuído como binário standalone (não depende do repo continuar clonado no disco — só os dotfiles em si dependem disso).

- **`detect.go`/`which.go`**: detecta a família do sistema (`/etc/os-release` no Linux, `runtime.GOOS` pra mac/Windows: arch/debian/fedora/suse/darwin/windows) e quais ferramentas opcionais existem (yay/paru, flatpak, brew, composer, npm, bun, fwupdmgr, choco/winget) — só roda uma etapa se a ferramenta estiver instalada.
- **`pipeline.go`**: monta os passos pra família detectada, divididos em paralelo (sistema de pacotes, flatpak, composer, npm, bun, firmware — todos independentes entre si, rodam em goroutines com output prefixado `[nome]` e um `sudo -v` + keep-alive pra não colidir pedido de senha) e serial (órfãos + cache, que só rodam depois que TODO o paralelo termina, pois dependem do estado final do gerenciador de pacotes).
- **`mirrors.go`**: ranqueia mirrors (reflector/cachyos-rate-mirrors no Arch, sempre `--protocol https` pro reflector pra não sujar a lista com `rsync://`, que pacman não consegue usar) e guarda o timestamp em `~/.local/state/sysup/last-mirror-check`; todo `sysup update` reranqueia sozinho se passou de 7 dias. `sysup schedule` instala um agendador nativo (systemd user timer / launchd / schtasks) pra manter isso em dia mesmo sem rodar update toda semana. Filtro geográfico pro reflector é opcional e por máquina, nunca hardcoded no repo (o `sysup` roda em qualquer lugar): grave países em `~/.config/sysup/mirror-country` (formato reflector, ex: `Brazil,Argentina,Chile`) pra restringir a busca à região; sem o arquivo, busca no mundo todo. `cachyos-rate-mirrors` já detecta a região sozinho, esse filtro só afeta o caminho do reflector.
- **`selfupdate.go`**: no início de todo `sysup update` (pula com `--no-self-update`), usa `github.com/rhysd/go-github-selfupdate` pra comparar a versão embutida no binário (`var version`, via `-ldflags -X main.version=vX.Y.Z` setado pelo GoReleaser a partir da tag) com a última release do GitHub; se houver uma nova, baixa o asset certo pro SO/arch, confere checksum e substitui o binário rodando, depois faz `syscall.Exec` pra continuar já na versão nova. Qualquer falha (sem internet, GitHub fora do ar) só loga aviso e segue o `update` normalmente — nunca aborta por causa disso. Também tenta (best-effort, via `~/.config/sysup/repo-path`) um `git pull --ff-only` no clone do repo, pra manter os dotfiles frescos.
- **`tool_gitkraken.go`/`tool_tidewave.go`**: portas dos antigos `gitkraken-install-or-update.sh` e `tidewave.sh` pro Go (mesma lógica: download, safe-swap, wrapper com `UPDATE_ON_START`, `fix-codex-acp`, porta padrão 8000). **Não** entram no pipeline automático de `update` — nenhum dos dois tem checagem de versão, sempre rebaixam o asset inteiro, então ficam como subcomandos explícitos (`sysup gitkraken`, `sysup tidewave [install|update|fix-codex-acp]`).
- **`color.go`/`tui.go`**: `sysup update` roda num dashboard full-screen (Bubble Tea + Lip Gloss + Bubbles) quando stdout é um terminal de verdade e não é `--dry-run` (`tuiAvailable`) — lista de steps com spinner, cor rotativa por step, ✔/✘/− (pulado) e duração ao vivo. Sudo é primado (`primeSudo`, extraído de `pipeline.go`, reusado por `RunParallel` e pela TUI) **antes** de entrar em alt-screen, porque o Bubble Tea assume o terminal e um prompt de senha no meio quebraria tudo. Cada step captura sua própria saída num buffer (`syncBuffer`) em vez de streamar ao vivo — só é exibida depois, no `── saída de "step" ──` impresso após a TUI fechar, se aquele step falhar. Fora de terminal (pipe, log, CI) ou com `NO_COLOR`/`--dry-run`, cai pro fallback em `runUpdatePlain` (log colorido linha a linha, sem alt-screen) — mesmo path usado antes da TUI existir.

**Release**: `.goreleaser.yaml` builda `linux/darwin` × `amd64/arm64` + `windows/amd64`, gera `checksums.txt`; `.github/workflows/release.yml` dispara em push de tag `v*.*.*` e publica tudo como GitHub Release via `goreleaser-action`. Processo: `git tag vX.Y.Z && git push --tags` — o resto é automático.

Rebuild manual (dev): `cd sysup && go build -o ~/.local/bin/sysup .`. Testar build de release local sem publicar: `go run github.com/goreleaser/goreleaser/v2@latest release --snapshot --clean --skip=publish`.

## Adding New Scripts

Place scripts in a subdirectory named after the tool/category. Follow the `need()` pattern for dependency checks and `set -euo pipefail` at the top.
