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

## Ricing

`ricing/` é o guarda-chuva único pra tudo que deixa a máquina com a cara certa — shell,
terminal, boot, KDE, prompt — tanto o que é symlinkado de verdade quanto o que é backup
manual restaurado sob demanda. Ver `ricing/README.md` pros comandos de restore (KDE via
`kwriteconfig6`/`lookandfeeltool`, fastfetch, oh-my-posh).

**`ricing/shell/`** — `.zshrc` (`shell/zsh/`), `.bashrc` (`shell/bash/`) e
`config.fish` (`shell/fish/`, `source`ado no fim do `~/.config/fish/config.fish` local
— não symlinkado direto, porque o `config.fish` local tem coisas específicas de máquina
como brew/bun/cachyos), um subdiretório por shell. Os três têm o mesmo conjunto de
aliases (nav, git, dev, lsd, serviços, tunnel, `tidewave`) e o mesmo `update`/`mirrors`
(que só chamam o binário `sysup`). `shell/.gitconfig` existe no repo mas **não** é
linkado pelo `install.sh` — está desatualizado (assinatura GPG antiga) em relação ao
setup real (assinatura SSH); não reative sem sincronizar o conteúdo primeiro. Tema de
prompt (oh-my-posh, `p10k.omp.json`) mora em `shell/zsh/tema/` — fica dentro de
`shell/zsh/` porque é especificamente o prompt do zsh, não um item visual solto.

**Setup em máquina nova:**
```bash
git clone <repo> ~/linux_scripts
cd ~/linux_scripts/ricing/shell
bash install.sh
```

`install.sh` faz backup do arquivo original (`*.bak`) se não for symlink, depois cria o
link (`.zshrc`, `.bashrc`); se `fish` existir, garante a linha
`source .../ricing/shell/fish/config.fish` no `config.fish` local (idempotente). Instala
`sysup` como **arquivo real** (não symlink) em `~/.local/bin/sysup` via
`ricing/shell/lib/install-sysup.sh` (sourceado, compartilhado com `install-menu.sh`):
primeiro tenta baixar o binário da última release do GitHub (funciona sem Go instalado —
é o caminho pra quem só clona o repo numa máquina nova/de amigo); só compila do fonte
(`go build ./cmd/sysup`) como fallback, se o download falhar e `go` estiver disponível.
Também grava `~/.config/sysup/repo-path` com o caminho do clone (usado pelo
`sysup update` pra um `git pull` best-effort nos dotfiles) e imprime uma dica sugerindo
`sysup polkit-setup` (não roda automático). Adicionar novo dotfile: colocar no
subdiretório do shell certo (`shell/zsh/`, `shell/bash/`, `shell/fish/`) e incluir a
chamada de `backup_and_link` correspondente no `install.sh`.

**`ricing/terminal/kitty/`** — config do kitty (tema "Idle Toes", fundo transparente).
**É** symlinkado de verdade — `~/.config/kitty/kitty.conf` e `current-theme.conf`
apontam direto pra cá (`ln -s`). Editar em qualquer um dos dois caminhos edita o mesmo
arquivo; não precisa copiar manualmente pra atualizar o snapshot, só commitar. Setup em
máquina nova:
```bash
ln -sf ~/linux_scripts/ricing/terminal/kitty/kitty.conf ~/.config/kitty/kitty.conf
ln -sf ~/linux_scripts/ricing/terminal/kitty/current-theme.conf ~/.config/kitty/current-theme.conf
```
(criar `~/.config/kitty/` antes se não existir). Subpasta `terminal/` pensada pra caber
outros emuladores de terminal no futuro (konsole, etc.), não só kitty.

**`ricing/grub/`** (`silent-boot.sh` + `95-grub-silent.hook`) — esconde o menu do GRUB e
as mensagens de loading, silencia o wall broadcast no shutdown, deixa o Plymouth assumir
a tela. Precisa `sudo`. Re-rodar `silent-boot.sh` após updates do pacote `grub`
(sobrescreve `/etc/grub.d/10_linux`).

**`ricing/fastfetch/`** — `config.jsonc` symlinkado (não backup on-demand). `images/`
(250 PNGs) e `presets/` (jsonc) vêm de https://github.com/Maheswara660/fastfetch
(imagens são LFS no repo original — clone raso baixa só ponteiros, precisa
`git lfs pull`) e também são symlinkados como pasta inteira pro
`~/.config/fastfetch/`. Logo atual é `images/165.png`, referenciado em
`config.jsonc`. `install-menu.sh`'s `action_fastfetch` faz os 3 symlinks
(config + images + presets). Adicionar novo item ricado: criar subpasta em
`ricing/`, documentar restore no `ricing/README.md`.

**`install-menu.sh`** (raiz do repo, shell + [`gum`](https://github.com/charmbracelet/gum)):
menu interativo de setas pra escolher quais pedaços instalar — shell, kitty, tema KDE,
fastfetch, oh-my-posh, ou o `sysup` pela primeira vez. Automatiza o passo a passo do
`ricing/README.md`, não substitui a documentação manual (continua valendo pra quem
prefere copiar/colar comando por comando). Ferramenta separada do `sysup` (shell, não Go
— não precisa compilar nada; usa a mesma família de ferramentas do Charm que o `sysup`
já usa via Bubble Tea, só que sem build). Requer `gum` instalado
(`sudo pacman -S gum`); a lógica de instalar o `sysup` é compartilhada com
`ricing/shell/install.sh` via `ricing/shell/lib/install-sysup.sh`, pra não duplicar.

## Claude Skills

`claude/skills/` versiona skills do Claude Code criadas por mim (ex: `token-efficient-docs`). `claude/install.sh` replica cada skill pra `~/.agents/skills/<nome>/` (fonte compartilhada entre agentes) e cria symlink em `~/.claude/skills/<nome>` (onde o Claude Code descobre skills) — mesmo padrão fonte-no-repo → symlink usado em `ricing/shell/`, mas idempotente e sem depender do repo continuar clonado (a cópia em `~/.agents/skills` sobrevive independente do symlink). Adicionar skill nova: criar `claude/skills/<nome>/SKILL.md`, rodar `bash claude/install.sh`. Ver `claude/README.md`.

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

`sysup/` é um módulo Go — binário único (`sysup update|mirrors|schedule|gitkraken|tidewave|polkit-setup [--dry-run]`) que substitui os antigos scripts/aliases de update espalhados pelo repo. Distribuído como binário standalone (não depende do repo continuar clonado no disco — só os dotfiles em si dependem disso).

Layout `cmd/` + `internal/*` (padrão Go, não é tudo `package main` na raiz):

- **`cmd/sysup/main.go`**: entrypoint fino, só chama `internal/cli.Run()`.
- **`internal/cli`**: dispatch de subcomandos e a orquestração de `update` (self-update → worker do polkit → pipeline → TUI/plain → resumo/notificação). Era o antigo `main.go` da raiz.
- **`internal/detect`**: detecta a família do sistema (`/etc/os-release` no Linux, `runtime.GOOS` pra mac/Windows: arch/debian/fedora/suse/darwin/windows) e quais ferramentas opcionais existem (yay/paru, flatpak, brew, composer, npm, bun, fwupdmgr, choco/winget, pkexec) — só roda uma etapa se a ferramenta estiver instalada.
- **`internal/style`**: helpers ANSI (`Ok`/`Fail`/`Warn`/`Dim`/`Header`/`Colorize`) — pacote-folha, sem dependência interna, usado por quase todo o resto.
- **`internal/pipeline`**: monta os passos pra família detectada, divididos em paralelo (sistema de pacotes, flatpak, composer, npm, bun, firmware — todos independentes entre si, rodam em goroutines com output prefixado `[nome]`) e serial (órfãos + cache, que só rodam depois que TODO o paralelo termina, pois dependem do estado final do gerenciador de pacotes). Passos privilegiados usam o worker de polkit quando disponível (ver bullet abaixo); sem ele, caem pro `sudo -v` + keep-alive de sempre (`PrimeSudo`) pra não colidir pedido de senha.
- **`internal/polkit` + `cmd/sysup-worker/`**: quando `sysup polkit-setup` já rodou, `sysup update` autoriza **um único** `pkexec /usr/lib/sysup/sysup-worker` no início do run (antes da TUI, mesmo timing do `PrimeSudo`) — o agente gráfico de polkit (KDE/GNOME/etc., detectado, nunca hardcoded a um DE) pede a senha uma vez, e o worker fica vivo só durante aquele run, servindo os passos de pacman/apt/dnf/zypper, cleanup e npm -g por um socket Unix local (`$XDG_RUNTIME_DIR/sysup-worker.sock`) em vez de reautenticar a cada passo. Toda validação de comando é feita dentro do worker (whitelist exata por família, sem `sh -c`, re-detecção própria — **`cmd/sysup-worker` deliberadamente não importa `internal/detect` nem nenhum outro pacote da árvore principal**, mantendo a superfície do binário privilegiado mínima e nunca confiando no que chega pelo socket). Não é um daemon: o ciclo de vida está amarrado a um pipe de stdin que o `sysup` pai mantém aberto pela duração do run. paru é redirecionado pro worker via `[bin] Sudo` no `paru.conf` (editado por `polkit-setup`, com backup `.bak`); yay não tem esse gancho de config, então continua chamando `sudo` de verdade — nessas máquinas o update dispara um segundo prompt (sudo clássico), primado junto com a autorização do polkit, nunca no meio do dashboard. Sem `polkit-setup` rodado, ou sem `pkexec` disponível, `BuildPipeline` cai de volta pro `sudo` de sempre. Detalhe completo (arquitetura, trade-offs, comparação com a alternativa de daemon systemd) em `sysup/README.md`.
- **`internal/download`**: lógica compartilhada de baixar+extrair+conferir checksum (usada por `internal/polkit` pra buscar o `sysup-worker` de uma release, e por `internal/tools` pro GitKraken) — antes eram duas implementações hand-rolled separadas.
- **`internal/mirrors`**: ranqueia mirrors (reflector/cachyos-rate-mirrors no Arch, sempre `--protocol https` pro reflector pra não sujar a lista com `rsync://`, que pacman não consegue usar) e guarda o timestamp em `~/.local/state/sysup/last-mirror-check`; todo `sysup update` reranqueia sozinho se passou de 7 dias. `sysup schedule` (`internal/schedule`) instala um agendador nativo (systemd user timer / launchd / schtasks) pra manter isso em dia mesmo sem rodar update toda semana. Filtro geográfico pro reflector é opcional e por máquina, nunca hardcoded no repo (o `sysup` roda em qualquer lugar): grave países em `~/.config/sysup/mirror-country` (formato reflector, ex: `Brazil,Argentina,Chile`) pra restringir a busca à região; sem o arquivo, busca no mundo todo. `cachyos-rate-mirrors` já detecta a região sozinho, esse filtro só afeta o caminho do reflector.
- **`internal/selfupdate`**: no início de todo `sysup update` (pula com `--no-self-update`), usa `github.com/rhysd/go-github-selfupdate` pra comparar a versão embutida no binário (`selfupdate.Version`, via `-ldflags -X sysup/internal/selfupdate.Version=vX.Y.Z` setado pelo GoReleaser a partir da tag) com a última release do GitHub; se houver uma nova, baixa o asset certo pro SO/arch, confere checksum e substitui o binário rodando, depois faz `syscall.Exec` pra continuar já na versão nova. Qualquer falha (sem internet, GitHub fora do ar) só loga aviso e segue o `update` normalmente — nunca aborta por causa disso. Também tenta (best-effort, via `~/.config/sysup/repo-path`) um `git pull --ff-only` no clone do repo, pra manter os dotfiles frescos.
- **`internal/tools`**: portas dos antigos `gitkraken-install-or-update.sh` e `tidewave.sh` pro Go (mesma lógica: download, safe-swap, wrapper com `UPDATE_ON_START`, `fix-codex-acp`, porta padrão 8000). **Não** entram no pipeline automático de `update` — nenhum dos dois tem checagem de versão, sempre rebaixam o asset inteiro, então ficam como subcomandos explícitos (`sysup gitkraken`, `sysup tidewave [install|update|fix-codex-acp]`).
- **`internal/tui`**: `sysup update` roda num dashboard full-screen (Bubble Tea + Lip Gloss + Bubbles) quando stdout é um terminal de verdade e não é `--dry-run` (`tui.Available`) — lista de steps com spinner, cor rotativa por step, ✔/✘/− (pulado), progresso `(N/TOTAL)` ao vivo (lido direto da própria saída do pacman, sem inventar contagem) e duração. A elevação de privilégio (worker de polkit se `polkit-setup` rodou, senão `PrimeSudo` clássico — em `internal/pipeline`/`internal/polkit`) acontece **antes** de entrar em alt-screen, porque o Bubble Tea assume o terminal e um prompt de senha no meio quebraria tudo. Cada step captura sua própria saída num buffer (`syncBuffer`) em vez de streamar ao vivo — só é exibida depois, no `── saída de "step" ──` impresso após a TUI fechar, se aquele step falhar. O resumo final (`tui.RenderSummaryBox`) é uma caixa com borda igual à dashboard, com uma linha declarativa por passo (`42 pacote(s) atualizado(s)`, `Já estava tudo atualizado`) — não um `✔ ok` genérico. Fora de terminal (pipe, log, CI) ou com `NO_COLOR`/`--dry-run`, cai pro fallback em `runUpdatePlain` (`internal/cli`, log colorido linha a linha, sem alt-screen).

**Release**: `.goreleaser.yaml` builda `linux/darwin` × `amd64/arm64` + `windows/amd64` (o `sysup-worker` é linux-only), gera `checksums.txt`; `.github/workflows/release.yml` dispara em push de tag `v*.*.*` e publica tudo como GitHub Release via `goreleaser-action` — o workflow chama `goreleaser release` genericamente, sem nomes de binário hardcoded, então pega qualquer novo `builds`/`archives` automaticamente. Processo: `git tag vX.Y.Z && git push --tags` — o resto é automático.

Rebuild manual (dev): `cd sysup && go build -o ~/.local/bin/sysup ./cmd/sysup`. Testar build de release local sem publicar: `go run github.com/goreleaser/goreleaser/v2@latest release --snapshot --clean --skip=publish`.

**Instalação numa máquina nova**: `ricing/shell/install.sh` (ver seção Ricing acima) instala o `sysup` via `ricing/shell/lib/install-sysup.sh` (baixa a release do GitHub; builda do fonte só se não achar release e tiver Go). Depois disso, opcionalmente, `sysup polkit-setup` configura a autenticação única do polkit (ver bullet acima) — não é automático, o `install.sh` só imprime a dica.

## Adding New Scripts

Place scripts in a subdirectory named after the tool/category. Follow the `need()` pattern for dependency checks and `set -euo pipefail` at the top.
