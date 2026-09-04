# Ricing Backup

Snapshot atual (KDE Plasma 6 + kitty + shell) pra restaurar depois.

**Atalho**: `../install-menu.sh` (raiz repo) automatiza passos abaixo, menu interativo (setas + `gum`). Marca o que quer, instala, sem copiar/colar comando por comando. Requer `gum` (`sudo pacman -S gum`). Passo a passo manual abaixo continua válido pra quem prefere mão ou quer entender cada restore.

## KDE Plasma

| Item | Valor |
|---|---|
| Look and Feel (global theme) | `com.github.vinceliuice.Layan` |
| Color scheme | Layan |
| Ícones | Papirus-Dark |
| Cursor | Layan-white-cursors |
| Decoração de janela (Aurorae) | Layan (`__aurorae__svg__Layan`) |
| Estilo de app (widget style) | Kvantum-Dark, engine Kvantum com tema `KvMojave` |
| Efeito blur do KWin | ativado |

Pacotes necessários (Arch):

```bash
yay -S layan-kde-git papirus-icon-theme-git layan-cursor-theme
sudo pacman -S kvantum   # inclui tema KvMojave
```

Restaurar via terminal (Plasma 6):

```bash
kwriteconfig6 --file kdeglobals --group KDE --key LookAndFeelPackage "com.github.vinceliuice.Layan"
kwriteconfig6 --file kdeglobals --group Icons --key Theme "Papirus-Dark"
kwriteconfig6 --file kcminputrc --group Mouse --key cursorTheme "Layan-white-cursors"
kwriteconfig6 --file kwinrc --group org.kde.kdecoration2 --key theme "__aurorae__svg__Layan"
kwriteconfig6 --file kwinrc --group org.kde.kdecoration2 --key library "org.kde.kwin.aurorae"
kwriteconfig6 --file kwinrc --group Plugins --key blurEnabled true
kwriteconfig6 --file kdeglobals --group KDE --key widgetStyle "kvantum-dark"
kvantummanager --set KvMojave
# relogar ou reiniciar plasmashell pra aplicar tudo
plasmashell --replace &
```

Mais simples: System Settings → Appearance → Global Theme → aplicar "Layan" (tema, ícones, cursor, decoração de uma vez, se instalado via `lookandfeeltool`).

```bash
lookandfeeltool -a com.github.vinceliuice.Layan
```

## Terminal (kitty)

Config em `terminal/kitty/`. Tema "Idle Toes", fundo transparente. Pasta `terminal/` pensada pra caber outros emuladores futuro (konsole, etc.), não só kitty.

- `background_opacity 0.8` + `dynamic_background_opacity yes` — transparência fundo.
- Blur visual vem do **compositor KWin** (blur effect ativado), não do kitty — kitty Linux/X11 não suporta `background_blur` nativo (exclusivo macOS).
- `shell` usa fish se instalado, senão bash (`sh -c 'exec "$(command -v fish || command -v bash)"'`) — funciona em qualquer máquina independente do shell padrão.

Restaurar (symlink, não copiar — repo fica fonte da verdade):

```bash
mkdir -p ~/.config/kitty
ln -s "$(pwd)/ricing/terminal/kitty/kitty.conf" ~/.config/kitty/kitty.conf
ln -s "$(pwd)/ricing/terminal/kitty/current-theme.conf" ~/.config/kitty/current-theme.conf
```

## Terminal (WezTerm, Windows)

Config em `terminal/wezterm/wezterm.lua`. Máquina Windows separada da KDE/kitty acima — WebGpu (`HighPerformance`), Catppuccin Mocha, tab bar embaixo (estilo kitty) integrada ao fundo (cores manuais iguais ao bg, sem faixa separada), `default_prog` = pwsh.

- `Ctrl+T` nova aba, `Ctrl+Q` fecha aba atual.
- `Ctrl+C` sempre copia (mesmo sem seleção); `Ctrl+Shift+C` manda SIGINT (`\x03`) pro processo.
- `Ctrl+V` cola.

Restaurar (Windows, copiar — sem symlink fácil sem modo dev/admin):

```powershell
Copy-Item ricing\terminal\wezterm\wezterm.lua "$HOME\.wezterm.lua"
```

## Terminal (Windows Terminal)

Config em `terminal/windows-terminal/settings.json`. Usado pra abas WSL (Arch), Nushell, pwsh e SSH. Fonte `JetBrainsMono NF` com `builtinGlyphs: false` — sem isso o Windows Terminal desenha os box-drawing com glifos próprios e os ícones Nerd Font do starship (separadores powerline, ícone de SO, relógio) saem errados.

`NF` e não `NFM` de propósito: a variante Mono comprime todo glifo em uma célula, o que deixa os ícones uniformes mas pequenos; a não-Mono respeita a largura de design e os ícones ficam maiores. O preço é ficarem desiguais entre si, porque cada um vem de um set patchado diferente (Material Design, Font Awesome, Octicons, Seti) com grid próprio. Trocar entre as duas é uma linha em `profiles.defaults.font.face`, mas exige fechar o Windows Terminal inteiro — abrir aba nova não recarrega a fonte. Opacity 80, acrylic na tab row, `Ctrl+C`/`Ctrl+V` copiar/colar, `alt+shift+d` duplica pane.

Restaurar (Windows, symlink — precisa modo desenvolvedor ligado, ou terminal como admin):

```powershell
$real = "$env:LOCALAPPDATA\Packages\Microsoft.WindowsTerminal_8wekyb3d8bbwe\LocalState\settings.json"
Copy-Item $real "$real.bak" -ErrorAction SilentlyContinue
Remove-Item $real -Force -ErrorAction SilentlyContinue
New-Item -ItemType SymbolicLink -Path $real -Target "$(Get-Location)\ricing\terminal\windows-terminal\settings.json"
```

Os `guid` dos profiles são gerados por máquina (source WSL/nu/pwsh); numa máquina nova, deixar o Windows Terminal gerar o `settings.json` dele primeiro e trocar só os `guid` da lista, ou aceitar que perfis não resolvidos ficam ocultos.

## Terminal (Rio, Windows)

Config em `terminal/rio/config.toml`. Alternativa ao WezTerm testada na mesma máquina — mesma paleta Catppuccin Mocha (`[colors]` colado direto no config, mesmos hex do WezTerm), `shell.program` = pwsh, opacity 0.90.

Instalar: `winget install -e --id raphamorim.rio`

Restaurar (Windows, copiar):

```powershell
Copy-Item ricing\terminal\rio\config.toml "$HOME\AppData\Local\rio\config.toml"
```

## Shell (PowerShell + Starship, Windows)

Config em `shell/powershell/`. Prompt via Starship (não oh-my-posh — testado, ~900ms mais lento no boot pelo módulo de 55KB que oh-my-posh gera; Starship é ~10KB). Tema em `shell/starship/windows.toml` desliga módulos de versão de linguagem (php/node/python/etc) — cada um spawna processo verificando `composer.json`/`package.json` por pasta, lento em drive de rede. `Terminal-Icons` (módulo PowerShell Gallery) dá ícones no `ls`/`Get-ChildItem`.

Automatizado via `install-menu.sh` → "starship (symlink tema)" (roda em Git Bash — `install_starship`/`install_powershell_profile` em `ricing/shell/lib/install-shell-tools.sh`): instala starship, symlinka `windows.toml`, symlinka o profile pra `$HOME\Documents\PowerShell\`, instala `Terminal-Icons` se faltar. Restaurar manual (equivalente, se preferir sem o Git Bash):

```powershell
Copy-Item ricing\shell\starship\windows.toml "$HOME\.config\starship.toml"
Copy-Item ricing\shell\powershell\Microsoft.PowerShell_profile.ps1 "$HOME\Documents\PowerShell\Microsoft.PowerShell_profile.ps1"
winget install Starship.Starship
Install-Module -Name Terminal-Icons -Repository PSGallery -Scope CurrentUser
```

## fastfetch

Dois perfis, ambos com o mesmo logo (`images/165.png`) e as mesmas seções (`break` entre elas): sistema → shell/terminal/ambiente gráfico → hardware → rede/energia. `images/` (250 PNGs) e `presets/` (jsonc) vêm de https://github.com/Maheswara660/fastfetch.

| Comando | Template | Conteúdo |
|---|---|---|
| `fastfetch` | `fastfetch/config.jsonc.tmpl` | Enxuto — 16 linhas, mesma altura do logo |
| `fastfetch --full` | `fastfetch/full.jsonc.tmpl` | Tudo: separador, host, board, BIOS, fonte do terminal, ícones, cursor, display, disco, som, IP local, locale, usuários |

`--full` não é flag do fastfetch: é atalho de uma função de shell (`bash/.bashrc`, `fish/config.fish`, `powershell/Microsoft.PowerShell_profile.ps1`) que troca por `--config full`. Qualquer outro argumento passa direto pro binário.

Os ícones das chaves são fixos no template e todos do set Material Design (`nf-md-*`), pra ficarem do mesmo tamanho visual — ver a seção do Windows Terminal sobre `NF` vs `NFM`. O único que varia por plataforma é o de SO, via placeholder `@OS_ICON@`. Por isso `display.key.type` é `"string"` e não `"both"`: com `"both"` o fastfetch prefixaria o ícone dele.

### Por que `config.jsonc` não é symlink

Cinco valores variam por máquina — protocolo de imagem, path absoluto do logo, largura e altura em células, ícone de SO. Então o repo guarda só os `.tmpl`, com os placeholders `@LOGO_TYPE@`, `@LOGO_PATH@`, `@LOGO_W@`, `@LOGO_H@` e `@OS_ICON@`, e `install-menu.sh` (ação "fastfetch") gera os arquivos reais em `~/.config/fastfetch/{config,full}.jsonc`. Symlinkar geraria diff de git a cada troca de máquina.

A altura do logo é **medida, não configurada**: o install gera os configs, roda `fastfetch --logo none | wc -l` pra saber quantas linhas os módulos imprimem nesta máquina, e regenera o sixel nessa altura. Sem isso sobra vão em branco entre a imagem e o prompt, porque módulo que não encontra nada (`DE` no Windows, `battery`/`poweradapter` num desktop) não imprime linha alguma — e quanto disso acontece só a máquina sabe. `FASTFETCH_LOGO_ROWS` é só o palpite inicial e o fallback se a medição falhar.

Trocar a imagem: `FASTFETCH_LOGO_IMG`, com o nome de um arquivo de `images/`. A largura acompanha pela proporção.

| Plataforma | `logo.type` | `logo.source` |
|---|---|---|
| kitty nativo | `kitty` | `images/165.png` |
| WSL | `sixel` | `images/165.png` |
| Windows (Terminal e WezTerm) | `raw` | `165.sixel`, pré-convertido pelo `chafa` no install |

`fastfetch --show-errors` mostra o erro real quando a imagem não aparece.

### Windows: por que `raw` e não `sixel`

O `--logo-type sixel` do fastfetch não funciona no Windows nativo: ele carrega o ImageMagick pedindo `libMagickCore-7.Q16HDRI-10.dll` (naming do MSYS2, ABI do 7.1.1) e o instalador oficial entrega `CORE_RL_MagickCore_.dll` do 7.1.2. Renomear tira o `Image Magick library not found`, mas cai em `Failed to load / convert the image source`, porque a ABI difere. Montar as DLLs mingw da versão certa também não resolve (as libs atuais não são contemporâneas do 7.1.1 — dá `WinError 127`).

A saída é converter fora do fastfetch: `render_sixel_logo` no `install-menu.sh` gera `~/.config/fastfetch/165.sixel`, e o `logo.type` fica `raw`, que despeja o arquivo byte a byte. Como o WezTerm também renderiza sixel, `raw` cobre os dois terminais.

Três armadilhas:

- **`file-raw` não serve** — trata o arquivo como linhas de texto e injeta escapes no meio do blob DCS, embaralhando a tela em listras. Tem que ser `raw`.
- **`width`/`height` são só reserva de espaço** — `raw` não sabe o tamanho do blob. Se não casarem com o sixel, o texto sobrepõe a imagem ou sobra vão em branco até o prompt. A reserva sai das dimensões em px que o conversor grava nos raster attributes do header, divididas pela célula do terminal (`FASTFETCH_CELL_PX_W`/`_H`).
- **A célula é medida, não estimada** — chutar esse valor foi a causa do vão em branco. Num terminal novo (fonte ou tamanho diferente muda), medir com `ESC[16t`, que responde `ESC[6;<altura>;<largura>t`. No Windows Terminal com JetBrainsMono NF no tamanho padrão: 20x10.

#### O conversor é o chafa, não o ImageMagick

Sixel não tem canal alpha, e o ImageMagick achata os pixels transparentes numa cor: branco por padrão (borda serrilhada em volta do logo), e achatar na cor do terminal só troca isso por um retângulo opaco, que aparece porque o perfil usa `opacity: 80`.

O chafa aproveita o `P2=1` do header DCS, que faz pixel não pintado ficar transparente de verdade. Verificável decodificando o blob: o chafa pinta ~60% da área, o ImageMagick pinta 100%.

Uma pegadinha no chafa: **`--colors 256` não quer dizer "256 cores", quer dizer "a paleta fixa de 256 do xterm"** — cubo 6x6x6 de tons genéricos, sem nenhuma cor da imagem, o que enche tudo de chuvisco por mais dither que se aplique. `--colors full` faz ele montar a paleta a partir da imagem (ainda cabe nos 256 registradores do sixel), e com paleta boa o dither deixa de ser necessário (`--dither none`).

### Restaurar

```bash
./install-menu.sh   # "fastfetch (config gerado + symlink)" — detecta a plataforma sozinho
```

No Windows o install instala o `chafa` sozinho via winget (`hpjansson.Chafa`).

WezTerm (build 2024-02) não renderiza o placeholder unicode do protocolo `kitty` (aparece `⸮` no lugar da imagem) — daí o Linux usar `kitty` só em kitty nativo. Remover o módulo `disk` se quiser esconder discos (vários drives montados no Windows poluem a saída).

## Prompt (oh-my-posh)

## Prompt (Starship)

Tema em `shell/starship/linux.toml` — pasta própria porque o prompt não é exclusivo de um shell (bash e fish carregam o mesmo tema; só o `windows.toml`, usado pelo PowerShell, é diferente — minimalista, módulos de linguagem desligados por causa do drive de rede, ver seção acima). Preset oficial `gruvbox-rainbow` (`starship preset gruvbox-rainbow -o starship.toml`) — segmentos powerline com fundo colorido, ícone de OS/usuário, git, versões de linguagem, hora. `.bashrc`/`config.fish` carregam via starship:

```bash
eval "$(starship init bash)"  # .bashrc
starship init fish | source   # config.fish
```

Restaurar (symlink, não copiar — repo fica fonte da verdade):

```bash
mkdir -p ~/.config
ln -s "$(pwd)/ricing/shell/starship/linux.toml" ~/.config/starship.toml
```

Se a máquina tiver oh-my-posh instalado de antes, `ricing/shell/install.sh` (ou `install-menu.sh` → "starship (symlink tema)") detecta e pergunta antes de desinstalar (via `pacman`/`apt` se veio de pacote, ou removendo o binário se foi instalação manual) — roda pra qualquer shell detectado, não só um.

## Shell

`.bashrc`, `config.fish` — cada um sua subpasta (`shell/bash/`, `shell/fish/`), mesmos aliases nos dois. zsh saiu do repo (pesado — oh-my-zsh + 4 plugins reparseando cada tecla digitada; fish tem highlighting/completions nativos). Tema do prompt fica fora dessas pastas, em `shell/starship/` (ver seção acima), por não ser exclusivo de nenhuma. Restaurar via `shell/install.sh` (symlink pra `~/`, backup `.bak` se já tem algo lá) ou `../install-menu.sh`. Detalhe completo no `CLAUDE.md` raiz repo (seção Ricing).

## Assinatura de commits (SSH)

`shell/git/.gitconfig` usa `gpg.format = ssh` — commits assinados com chave SSH dedicada, não a de autenticação. `signingKey` aponta pro mesmo path (`~/.ssh/luysfernnando_sign_commits.pub`) em toda máquina — só o *conteúdo* do keypair muda por máquina, o path fica fixo no `.gitconfig` compartilhado, sem override local. Setup por máquina nova via `install-menu.sh` → "Git (assinatura SSH + ssh-agent)" (ou manual, ver `ricing/shell/lib/install-git-ssh.sh`):

```bash
ssh-keygen -t ed25519 -f ~/.ssh/luysfernnando_sign_commits -C "$(git config --global user.email)"
```

Depois cadastra a pubkey nova no GitHub (github.com/settings/keys → "New SSH key" → tipo **Signing Key**, não Authentication).

`allowed_signers` é versionado em `shell/git/allowed_signers` e symlinkado pra `~/.ssh/allowed_signers` (mesmo padrão do `.gitconfig`) — cada máquina *acrescenta* sua pubkey nesse arquivo (nunca sobrescreve) e dá commit+push, assim qualquer máquina roda `git log --show-signature` e reconhece commit assinado em qualquer outra. Serve só pra validação local — GitHub verifica sozinho pelas chaves cadastradas na conta.

## GRUB silencioso

`grub/silent-boot.sh` esconde menu GRUB e mensagens de loading, silencia wall broadcast no shutdown, deixa Plymouth assumir tela — parte do "boot bonito", por isso mora aqui. Precisa `sudo`; re-rodar depois updates do pacote `grub`.