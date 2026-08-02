# sysup

Engine de update cross-distro/cross-OS em Go, self-updating. Binário único (`sysup update|mirrors|schedule|gitkraken|tidewave|polkit-setup [--dry-run]`), substitui scripts/aliases antigos de update. Standalone — não depende do repo continuar clonado (só dotfiles em si dependem).

## Layout

| Pacote | Faz |
|---|---|
| `cmd/sysup/main.go` | Entrypoint fino, chama `internal/cli.Run()` |
| `internal/cli` | Dispatch de subcomandos + orquestra `update` (self-update → worker polkit → pipeline → TUI/plain → resumo) |
| `internal/detect` | Detecta família do SO (`/etc/os-release` Linux, `runtime.GOOS` mac/Windows) + ferramentas opcionais instaladas (yay/paru, flatpak, brew, composer, npm, bun, fwupdmgr, choco/winget, pkexec) |
| `internal/style` | Helpers ANSI (`Ok`/`Fail`/`Warn`/`Dim`/`Header`/`Colorize`), pacote-folha sem dependência interna |
| `internal/pipeline` | Monta steps por família: paralelo (pacotes, flatpak, composer, npm, bun, firmware — goroutines, output prefixado `[nome]`) + serial (órfãos + cache, roda só depois do paralelo terminar) |
| `internal/polkit` + `cmd/sysup-worker` | Worker de elevação de privilégio de vida curta — ver seção Privilégio abaixo |
| `internal/download` | Baixa+extrai+confere checksum (compartilhado por `internal/polkit` pro `sysup-worker` e `internal/tools` pro GitKraken) |
| `internal/mirrors` | Rankeia mirrors (reflector/cachyos-rate-mirrors), guarda timestamp em `~/.local/state/sysup/last-mirror-check`, re-rankeia sozinho a cada 7 dias. Filtro geográfico opcional via `~/.config/sysup/mirror-country` |
| `internal/selfupdate` | Compara versão embutida (`-ldflags` setado pelo GoReleaser) com última release GitHub, baixa+substitui binário rodando via `syscall.Exec`; falha só loga aviso, nunca aborta o update. Também tenta `git pull --ff-only` nos dotfiles via `~/.config/sysup/repo-path` |
| `internal/tools` | Portas de `gitkraken-install-or-update.sh`/`tidewave.sh` pro Go (download, safe-swap, wrapper `UPDATE_ON_START`, `fix-codex-acp`). Fora do pipeline automático — sem checagem de versão, sempre rebaixam o asset inteiro, ficam como subcomandos explícitos |
| `internal/schedule` | Agendador nativo (systemd user timer / launchd / schtasks) pros mirrors, via `sysup schedule` |
| `internal/tui` | Dashboard full-screen (Bubble Tea + Lip Gloss + Bubbles) quando stdout é terminal real e não `--dry-run` — spinner, progresso `(N/TOTAL)` lido da saída do pacman, resumo final em caixa (`tui.RenderSummaryBox`). Fallback plain (`runUpdatePlain`) fora de terminal/CI/`NO_COLOR`/`--dry-run` |

## Release

`git tag vX.Y.Z && git push --tags` → `.github/workflows/release.yml` roda GoReleaser (`.goreleaser.yaml`), builda linux/darwin × amd64/arm64 + windows/amd64 (`sysup-worker` é linux-only), publica GitHub Release com `checksums.txt`.

Rebuild manual (dev): `cd sysup && go build -o ~/.local/bin/sysup ./cmd/sysup`.
Testar release local sem publicar: `go run github.com/goreleaser/goreleaser/v2@latest release --snapshot --clean --skip=publish`.

## Privilégio via polkit

### Problema original

`sysup update` roda num dashboard full-screen (Bubble Tea); depois de assumir o terminal (alt-screen) não dá pra mostrar prompt de senha no meio do run. Mecanismo antigo (`sudo -v` + ticker 60s) quebra silenciosamente quando a credencial expira no meio (suspend, `timestamp_timeout` curto no PAM, 2FA).

Tentativa anterior (só existiu como `git stash`) gerava `/etc/sudoers.d/sysup` com `NOPASSWD` escopado. Descartada: regra `NOPASSWD` **permanente** no sistema é brecha grande demais pra um utilitário pessoal.

### Arquitetura: worker de vida curta, autorizado uma vez via `pkexec`

```
sysup update
  │
  ├─ StartPrivilegedWorker() ──── pkexec /usr/lib/sysup/sysup-worker --socket <path>
  │                                  │
  │                          [ agente gráfico de polkit pede a senha — UMA vez ]
  │                                  │
  │                          sysup-worker (root) abre um socket Unix local
  │                          e fica vivo só durante este run
  │
  ├─ pipeline (TUI ou plain) ──── passos privilegiados mandam pedido pro socket
  │                                em vez de re-autenticar
  │
  └─ worker.Close() ──── fecha o pipe de stdin do worker → ele recebe EOF e encerra
```

| Ponto | Detalhe |
|---|---|
| Autorização única | Uma `<action>` polkit (`io.github.luysfernnando.sysup.worker`) amarrada ao binário — um evento de autorização por `update`, não por passo |
| Validação | Toda dentro do worker, nunca confia no que chega pelo socket. Whitelist fixa por família (`pacman -Syu --noconfirm`, `pacman -Rns --noconfirm <lista>`, `paccache -r`, `pacman -U --noconfirm <cache yay/paru>`, equivalentes apt/dnf/zypper, as 2 invocações exatas de `npm -g`) — sem `sh -c`, sem argumento livre. Worker re-detecta família/ferramentas sozinho |
| Não é daemon | Não instalado como serviço, não sobrevive entre execuções. Ciclo de vida = pipe stdin que o `sysup` pai mantém aberto; fecha → EOF → worker encerra. Timeout 15min só como defesa em profundidade |
| Socket | Local, permissão 0600, dentro `$XDG_RUNTIME_DIR` — modelo de ameaça pra máquina pessoal single-user, não multi-tenant |

### yay vs paru

yay e paru chamam `sudo` por conta própria pra instalar pacote AUR compilado (paru também sincroniza oficiais no mesmo `-Syu`).

| | Suporta redirecionar `sudo`? | Resultado |
|---|---|---|
| paru | Sim — `paru.conf [bin] Sudo = sysup-authbridge` (configurado por `polkit-setup`, symlink pro `sysup-worker`) | Uma autenticação cobre tudo, paru incluso |
| yay | Não — sem flag/config/env documentada | Update dispara 2º prompt (`sudo` clássico), primado junto com a autorização polkit, nunca no meio do dashboard. Sombrear `sudo` globalmente reabriria a brecha descartada acima — **deliberadamente não fizemos isso** |

`sysup-authbridge` também precisa funcionar fora do `sysup update` (ex: `paru -Syu` na mão): se `$SYSUP_WORKER_SOCKET` não definido ou worker não responde, cai pro `sudo` real, comportamento idêntico ao paru padrão. Só propaga erro sem retry quando o worker executa e o comando em si falha (repetir via sudo rodaria duas vezes).

Máquinas sem yay/paru (pacman puro, apt, dnf, zypper): exatamente um prompt, sempre.

### Limitações conhecidas

| Limitação | Detalhe |
|---|---|
| Headless sem agente gráfico | Sessão só-TTY (SSH sem X/Wayland): `pkexec` cai pro `pkttyagent`, mesma restrição do `sudo` antigo. Design resolve pra sessões gráficas (uso real, KDE Plasma), não headless |
| `npm -g` na whitelist | São as 2 invocações exatas que o pipeline sempre emite (`npm install -g npm@latest`, `npm update -g`) — não é acesso livre |
| `polkit-setup` edita `paru.conf` | Primeira vez que o setup mexe em config de terceiros — faz backup `.bak` antes, mostra conteúdo proposto antes de aplicar |

### Alternativa cogitada: daemon systemd + socket

Se polkit não se provar confiável em uso real, plano B é daemon root persistente (`systemd` service/socket) — sem `pkexec`, zero prompts após setup.

| | polkit (atual) | daemon systemd (plano B) |
|---|---|---|
| Prompts após setup | 1 por run (0 com paru; 2 com yay) | 0 |
| Processo root residente | Não — só durante run | Sim, sempre |
| Superfície de ataque permanente | Nenhuma além dos arquivos instalados | Listener root sempre vivo |
| Peças móveis | policy XML + 1 binário + symlink | unit files + socket activation + protocolo IPC |
| Portável entre distros | Sim (polkit em quase todo Linux desktop) | Sim, mais peças pra instalar/habilitar |

Polkit ganha em superfície de ataque; daemon ganharia em UX pura (zero prompts sempre).
