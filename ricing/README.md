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
- `shell` usa fish se instalado, senão zsh (`sh -c 'exec "$(command -v fish || command -v zsh)"'`) — funciona em qualquer máquina independente do shell padrão.

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

## Terminal (Rio, Windows)

Config em `terminal/rio/config.toml`. Alternativa ao WezTerm testada na mesma máquina — mesma paleta Catppuccin Mocha (`[colors]` colado direto no config, mesmos hex do WezTerm), `shell.program` = pwsh, opacity 0.90.

Instalar: `winget install -e --id raphamorim.rio`

Restaurar (Windows, copiar):

```powershell
Copy-Item ricing\terminal\rio\config.toml "$HOME\AppData\Local\rio\config.toml"
```

## Shell (PowerShell + Starship, Windows)

Config em `shell/powershell/`. Prompt via Starship (não oh-my-posh — testado, ~900ms mais lento no boot pelo módulo de 55KB que oh-my-posh gera; Starship é ~10KB). Tema em `shell/starship/windows.toml` desliga módulos de versão de linguagem (php/node/python/etc) — cada um spawna processo verificando `composer.json`/`package.json` por pasta, lento em drive de rede. `Terminal-Icons` (módulo PowerShell Gallery) dá ícones no `ls`/`Get-ChildItem`.

Restaurar:

```powershell
Copy-Item ricing\shell\starship\windows.toml "$HOME\.config\starship.toml"
Copy-Item ricing\shell\powershell\Microsoft.PowerShell_profile.ps1 "$HOME\Documents\PowerShell\Microsoft.PowerShell_profile.ps1"
winget install Starship.Starship
Install-Module -Name Terminal-Icons -Repository PSGallery -Scope CurrentUser
```

## fastfetch

Config em `fastfetch/config.jsonc.tmpl`. Logo `images/165.png`, módulos agrupados em seções (`break` entre elas): sistema → shell/terminal → ambiente gráfico (DE/WM/tema) → hardware → rede/energia. `images/` (250 PNGs) e `presets/` (jsonc) vêm de https://github.com/Maheswara660/fastfetch.

Protocolo de imagem do logo (`logo.type`) varia por terminal — kitty/WezTerm usam `kitty`, Windows Terminal (WSL) usa `sixel` (precisa de `imagemagick` instalado pra decodificar; `fastfetch --show-errors` mostra o erro real se a imagem não aparecer). Por isso `config.jsonc` **não é symlink**: `config.jsonc.tmpl` fica no repo com o placeholder `@LOGO_TYPE@`, e `install-menu.sh` (ação "fastfetch") gera o arquivo real em `~/.config/fastfetch/config.jsonc` substituindo pelo valor certo pra cada máquina — symlinkar geraria diff de git toda vez que WSL e nativo divergissem.

Restaurar:

```bash
mkdir -p ~/.config/fastfetch
sed "s/@LOGO_TYPE@/kitty/" ricing/fastfetch/config.jsonc.tmpl > ~/.config/fastfetch/config.jsonc  # ou "sixel" no WSL
ln -s "$(pwd)/ricing/fastfetch/images" ~/.config/fastfetch/images
ln -s "$(pwd)/ricing/fastfetch/presets" ~/.config/fastfetch/presets
```

Ou simplesmente `./install-menu.sh` → "fastfetch (config gerado + symlink)", que já detecta WSL sozinho.

Restaurar (Windows, copiar — `config.jsonc` tem path do logo hardcoded pra máquina Linux, `/home/lulfex/...`; ajustar `logo.source` na cópia local depois, não editar o do repo):

```powershell
winget install Fastfetch-cli.Fastfetch
mkdir -p ~/.config/fastfetch
Copy-Item ricing\fastfetch\config.jsonc "$HOME\.config\fastfetch\config.jsonc"
Copy-Item -Recurse ricing\fastfetch\images "$HOME\.config\fastfetch\images"
Copy-Item -Recurse ricing\fastfetch\presets "$HOME\.config\fastfetch\presets"
# editar "logo.source" no config.jsonc copiado pro path Windows de images/165.png
```

WezTerm (build 2024-02) não renderiza direito o placeholder unicode do protocolo `kitty` (aparece `⸮` no lugar da imagem). Trocar `"logo.type"` de `"kitty"` pra `"iterm"` na cópia local resolve — WezTerm suporta protocolo iTerm2 nativamente, sem placeholder unicode. Também remover módulo `disk` na cópia local se quiser esconder discos (múltiplos drives montados no Windows poluem a saída).

## Prompt (oh-my-posh)

## Prompt (Starship)

Tema em `shell/starship/linux.toml` — pasta própria porque o prompt não é exclusivo de um shell (zsh, bash e fish todos carregam o mesmo tema; só o `windows.toml`, usado pelo PowerShell, é diferente — minimalista, módulos de linguagem desligados por causa do drive de rede, ver seção acima). Preset oficial `gruvbox-rainbow` (`starship preset gruvbox-rainbow -o starship.toml`) — segmentos powerline com fundo colorido, ícone de OS/usuário, git, versões de linguagem, hora. `.zshrc`/`config.fish` carregam via starship:

```bash
eval "$(starship init zsh)"   # .zshrc
starship init fish | source   # config.fish
```

Restaurar (symlink, não copiar — repo fica fonte da verdade):

```bash
mkdir -p ~/.config
ln -s "$(pwd)/ricing/shell/starship/linux.toml" ~/.config/starship.toml
```

Se a máquina tiver oh-my-posh ou powerlevel10k instalado de antes, `ricing/shell/install.sh` (ou `install-menu.sh` → "starship (symlink tema)") detecta e pergunta antes de desinstalar (via `pacman`/`apt` se veio de pacote, ou removendo o binário/pasta se foi instalação manual). Esse passo hoje só roda quando o shell detectado é zsh — fish ganha o `starship init fish` no `config.fish` mas depende do symlink já ter sido feito numa passada zsh, ou de rodar o `ln -s` manual acima.

zsh também usa oh-my-zsh (`~/.oh-my-zsh`) — ver `plugins=(...)` e `ZSH_THEME` em `shell/zsh/.zshrc` (já versionado neste repo).

## Shell

`.zshrc`, `.bashrc`, `config.fish` — cada um sua subpasta (`shell/zsh/`, `shell/bash/`, `shell/fish/`), mesmos aliases nos três. Tema do prompt fica fora dessas pastas, em `shell/starship/` (ver seção acima), por não ser exclusivo de nenhuma. Restaurar via `shell/install.sh` (symlink pra `~/`, backup `.bak` se já tem algo lá) ou `../install-menu.sh`. Detalhe completo no `CLAUDE.md` raiz repo (seção Ricing).

## Assinatura de commits (SSH)

`shell/git/.gitconfig` usa `gpg.format = ssh` — commits assinados com chave SSH dedicada, não a de autenticação. `signingKey` aponta pro mesmo path (`~/.ssh/luysfernnando_sign_commits.pub`) em toda máquina — só o *conteúdo* do keypair muda por máquina, o path fica fixo no `.gitconfig` compartilhado, sem override local. Setup por máquina nova via `install-menu.sh` → "Git (assinatura SSH + ssh-agent)" (ou manual, ver `ricing/shell/lib/install-git-ssh.sh`):

```bash
ssh-keygen -t ed25519 -f ~/.ssh/luysfernnando_sign_commits -C "$(git config --global user.email)"
```

Depois cadastra a pubkey nova no GitHub (github.com/settings/keys → "New SSH key" → tipo **Signing Key**, não Authentication).

`allowed_signers` é versionado em `shell/git/allowed_signers` e symlinkado pra `~/.ssh/allowed_signers` (mesmo padrão do `.gitconfig`) — cada máquina *acrescenta* sua pubkey nesse arquivo (nunca sobrescreve) e dá commit+push, assim qualquer máquina roda `git log --show-signature` e reconhece commit assinado em qualquer outra. Serve só pra validação local — GitHub verifica sozinho pelas chaves cadastradas na conta.

## GRUB silencioso

`grub/silent-boot.sh` esconde menu GRUB e mensagens de loading, silencia wall broadcast no shutdown, deixa Plymouth assumir tela — parte do "boot bonito", por isso mora aqui. Precisa `sudo`; re-rodar depois updates do pacote `grub`.