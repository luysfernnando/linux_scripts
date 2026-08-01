# sysup — privilégio via polkit

Este documento cobre só o mecanismo de elevação de privilégio do `sysup update`
(por que existe, como funciona, e onde ele deliberadamente não resolve tudo). Pro
resto do `sysup` (pipeline, TUI, self-update, mirrors), ver a seção `sysup` do
`CLAUDE.md` na raiz do repo.

## O problema original

`sysup update` roda dentro de um dashboard full-screen (Bubble Tea). Depois que ele
assume o terminal (alt-screen), não tem como mostrar um prompt de senha no meio do
run — `stdin`/`stdout` pertencem à TUI, e a saída de cada passo vai pra um buffer, não
pro terminal real. O mecanismo antigo (`sudo -v` antes de entrar na TUI + um ticker de
60s tentando manter a credencial viva) funciona na maioria das vezes, mas quebra
silenciosamente sempre que a credencial expira no meio do run (suspend da máquina,
`timestamp_timeout` curto no PAM, 2FA que nunca cacheia).

Uma tentativa anterior (não commitada, só existiu como `git stash`) resolvia isso
gerando um drop-in `/etc/sudoers.d/sysup` com regras `NOPASSWD` escopadas por comando
exato. Foi descartada: mesmo escopada, uma regra `NOPASSWD` **permanente** instalada no
sistema é uma brecha grande demais pra deixar aberta indefinidamente só pra um
utilitário pessoal de update.

## Arquitetura: worker de vida curta, autorizado uma vez via `pkexec`

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

Pontos centrais:

- **Uma única autorização por execução.** Só existe uma `<action>` de polkit
  (`io.github.luysfernnando.sysup.worker`), amarrada ao binário `sysup-worker`. Ela não
  varia por operação — o worker decide internamente o que é permitido, então só existe
  um evento de autorização por `sysup update`, não um por passo.
- **Toda validação acontece dentro do worker**, nunca confiando no que chega pelo
  socket sem checagem. A whitelist é um conjunto fixo de comandos exatos por família
  (`pacman -Syu --noconfirm`, `pacman -Rns --noconfirm <lista>`, `paccache -r`,
  `pacman -U --noconfirm <path dentro do cache do yay/paru>`, os equivalentes
  apt/dnf/zypper, e as duas invocações exatas do passo `npm -g`) — sem `sh -c`,
  sem argumento livre. O worker também **re-detecta**
  família/ferramentas sozinho; nunca confia em nada que o cliente diga sobre o sistema.
- **Não é um daemon.** `sysup-worker` não é instalado como serviço, não sobrevive entre
  execuções, não fica residente entre boots. Seu ciclo de vida está amarrado a um pipe
  de stdin que o `sysup` pai mantém aberto pela duração do run — ao fechar (fim normal
  ou crash), o worker recebe EOF e encerra. Um timeout de 15min existe só como defesa em
  profundidade, caso o pipe nunca feche por algum motivo. Isso é intencionalmente
  diferente da alternativa de daemon systemd cogitada como plano B (ver abaixo).
- **Socket local, permissão 0600** dentro de `$XDG_RUNTIME_DIR` (diretório 0700 por
  usuário, padrão do systemd-logind) — única barreira de acesso. Modelo de ameaça
  adequado pra uma máquina pessoal single-user, não pra ambiente multi-tenant.

## O caso yay/paru

yay e paru chamam `sudo` **por conta própria**, internamente, pra instalar o pacote AUR
que acabaram de compilar (paru também usa isso pra sincronizar pacotes oficiais dentro
do mesmo `-Syu`). Pesquisamos se dava pra redirecionar isso sem modificar os binários:

- **paru suporta** (`paru.conf`, seção `[bin]`): `Sudo = <binário>`. `sysup polkit-setup`
  configura isso automaticamente, apontando pra `sysup-authbridge` — um binário sem
  privilégio (symlink pro mesmo `sysup-worker`) que só repassa a chamada pro socket do
  worker já autorizado. Resultado: paru nunca chama `sudo` de verdade, então **uma
  autenticação cobre tudo**, paru incluso. O `SudoLoop` do próprio paru (keepalive de
  credencial) não é tocado nem precisa ser — o worker já cobre a sessão inteira.
- **yay não suporta** isso — não existe flag, config ou env var documentada pra trocar o
  binário de escalação. A única forma seria sombrear o `sudo` do sistema globalmente, o
  que reabriria exatamente o tipo de brecha ampla que motivou descartar o approach de
  sudoers. **Deliberadamente não fizemos isso.**

Consequência prática: em máquinas com **yay**, `sysup update` ainda dispara um segundo
prompt — o `sudo` clássico, primado no mesmo instante em que o worker é autorizado
(ambos antes do alt-screen, nunca no meio do dashboard). Não é "uma senha só" no sentido
literal nesse caso específico, mas resolve o bug real (travar/falhar no meio do update).
Quem quer a experiência de senha única de verdade numa máquina Arch: **use paru em vez
de yay** (nota, não exigência — yay continua funcionando, só com um prompt a mais).

### `sysup-authbridge` precisa funcionar fora do `sysup update` também

Depois que `polkit-setup` configura `paru.conf`, `sysup-authbridge` vira a **única**
forma de paru escalar privilégio — inclusive quando alguém roda `paru -Syu` na mão, sem
o `sysup` no meio. Por isso ele nunca falha direto: se `$SYSUP_WORKER_SOCKET` não está
definido (uso manual) ou o worker não responde (socket morto, `sysup update` não está
rodando), ele cai pro `sudo` real do sistema, prompt normal, comportamento idêntico ao
que paru sempre teve. Só quando o worker **executa e a própria pacman/etc falha** é que
o authbridge propaga o erro sem tentar de novo — repetir via sudo nesse caso rodaria o
comando duas vezes.

Máquinas sem yay/paru (pacman puro, apt, dnf, zypper): **exatamente um prompt**,
sempre.

## Limitações conhecidas, sem esconder

- **Headless sem agente gráfico de polkit.** Numa sessão só-TTY (SSH sem X/Wayland),
  `pkexec` cai pro `pkttyagent`, que prompta contra o terminal controlador — herdando a
  mesma restrição de "precisa acontecer antes do alt-screen" que o `sudo` tem hoje. Este
  design resolve o problema pra sessões gráficas (o uso real do usuário, que roda KDE
  Plasma), não pra máquinas headless.
- **`npm -g` está na whitelist** como as duas invocações exatas que o pipeline sempre
  emite (`npm install -g npm@latest`, `npm update -g`) — não é "npm pode fazer qualquer
  coisa como root", é exatamente o que o passo já fazia antes, só que sem sudo repetido.
- **`sysup polkit-setup` é a primeira vez que o setup edita um arquivo de config de
  terceiros** (`paru.conf`), não só cria arquivos novos — por isso faz backup `.bak`
  antes de sobrescrever e mostra o conteúdo final proposto antes de aplicar.

## Comparação com a alternativa: daemon systemd + socket

Combinado com o usuário: se essa abordagem via polkit não se comportar como esperado na
prática, o próximo passo é um daemon root persistente (`systemd` service/socket) que o
`sysup` cliente fala por um socket Unix — sem `pkexec`, sem prompt algum depois do setup
inicial.

| | polkit (atual) | daemon systemd (plano B) |
|---|---|---|
| Prompts após setup | 1 por run (0 com paru; 2 com yay) | 0 |
| Processo root residente | Não — só durante o run | Sim, sempre (ou socket-activated) |
| Superfície de ataque permanente | Nenhuma além dos arquivos instalados | Um listener root sempre vivo |
| Peças móveis | policy XML + 1 binário + symlink | unit files + socket activation + protocolo IPC |
| Portável entre distros | Sim (polkit existe em praticamente todo Linux desktop) | Sim, mas mais peças pra instalar/habilitar |

O polkit ganha em superfície de ataque (nada fica residente); o daemon ganharia em UX
pura (zero prompts, sempre). Se o polkit não se provar confiável em uso real (ex:
diferenças entre desktops, comportamento inconsistente do agente gráfico em alguma
distro), essa tabela é o ponto de partida pra reavaliar.
