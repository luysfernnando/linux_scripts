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

## sysup (engine de update cross-distro/cross-OS, self-updating)

`sysup/` é um módulo Go — binário único (`sysup update|mirrors|schedule|gitkraken|tidewave [--dry-run]`) que substitui os antigos scripts/aliases de update espalhados pelo repo. Distribuído como binário standalone (não depende do repo continuar clonado no disco — só os dotfiles em si dependem disso).

- **`detect.go`/`which.go`**: detecta a família do sistema (`/etc/os-release` no Linux, `runtime.GOOS` pra mac/Windows: arch/debian/fedora/suse/darwin/windows) e quais ferramentas opcionais existem (yay/paru, flatpak, brew, composer, npm, bun, fwupdmgr, choco/winget) — só roda uma etapa se a ferramenta estiver instalada.
- **`pipeline.go`**: monta os passos pra família detectada, divididos em paralelo (sistema de pacotes, flatpak, composer, npm, bun, firmware — todos independentes entre si, rodam em goroutines com output prefixado `[nome]` e um `sudo -v` + keep-alive pra não colidir pedido de senha) e serial (órfãos + cache, que só rodam depois que TODO o paralelo termina, pois dependem do estado final do gerenciador de pacotes).
- **`mirrors.go`**: ranqueia mirrors (reflector/cachyos-rate-mirrors no Arch) e guarda o timestamp em `~/.local/state/sysup/last-mirror-check`; todo `sysup update` reranqueia sozinho se passou de 7 dias. `sysup schedule` instala um agendador nativo (systemd user timer / launchd / schtasks) pra manter isso em dia mesmo sem rodar update toda semana.
- **`selfupdate.go`**: no início de todo `sysup update` (pula com `--no-self-update`), usa `github.com/rhysd/go-github-selfupdate` pra comparar a versão embutida no binário (`var version`, via `-ldflags -X main.version=vX.Y.Z` setado pelo GoReleaser a partir da tag) com a última release do GitHub; se houver uma nova, baixa o asset certo pro SO/arch, confere checksum e substitui o binário rodando, depois faz `syscall.Exec` pra continuar já na versão nova. Qualquer falha (sem internet, GitHub fora do ar) só loga aviso e segue o `update` normalmente — nunca aborta por causa disso. Também tenta (best-effort, via `~/.config/sysup/repo-path`) um `git pull --ff-only` no clone do repo, pra manter os dotfiles frescos.
- **`tool_gitkraken.go`/`tool_tidewave.go`**: portas dos antigos `gitkraken-install-or-update.sh` e `tidewave.sh` pro Go (mesma lógica: download, safe-swap, wrapper com `UPDATE_ON_START`, `fix-codex-acp`, porta padrão 8000). **Não** entram no pipeline automático de `update` — nenhum dos dois tem checagem de versão, sempre rebaixam o asset inteiro, então ficam como subcomandos explícitos (`sysup gitkraken`, `sysup tidewave [install|update|fix-codex-acp]`).

**Release**: `.goreleaser.yaml` builda `linux/darwin` × `amd64/arm64` + `windows/amd64`, gera `checksums.txt`; `.github/workflows/release.yml` dispara em push de tag `v*.*.*` e publica tudo como GitHub Release via `goreleaser-action`. Processo: `git tag vX.Y.Z && git push --tags` — o resto é automático.

Rebuild manual (dev): `cd sysup && go build -o ~/.local/bin/sysup .`. Testar build de release local sem publicar: `go run github.com/goreleaser/goreleaser/v2@latest release --snapshot --clean --skip=publish`.

## Adding New Scripts

Place scripts in a subdirectory named after the tool/category. Follow the `need()` pattern for dependency checks and `set -euo pipefail` at the top.
