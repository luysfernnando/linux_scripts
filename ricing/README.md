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

## Shell (PowerShell + Starship, Windows)

Config em `shell/powershell/`. Prompt via Starship (não oh-my-posh — testado, ~900ms mais lento no boot pelo módulo de 55KB que oh-my-posh gera; Starship é ~10KB). `starship.toml` desliga módulos de versão de linguagem (php/node/python/etc) — cada um spawna processo verificando `composer.json`/`package.json` por pasta, lento em drive de rede. `Terminal-Icons` (módulo PowerShell Gallery) dá ícones no `ls`/`Get-ChildItem`.

Restaurar:

```powershell
Copy-Item ricing\shell\powershell\starship.toml "$HOME\.config\starship.toml"
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

## Prompt (Starship)

Tema em `shell/zsh/tema/` (mora dentro `shell/zsh/` — específico prompt zsh, não item solto). Preset oficial `gruvbox-rainbow` (`starship preset gruvbox-rainbow -o starship.toml`) — segmentos powerline com fundo colorido, ícone de OS/usuário, git, versões de linguagem, hora. Visual diferente do `starship.toml` do PowerShell/Windows (esse é minimalista, módulos de linguagem desligados por causa do drive de rede — ver seção acima). `.zshrc` carrega via starship:

```bash
eval "$(starship init zsh)"
```

Restaurar (symlink, não copiar — repo fica fonte da verdade):

```bash
mkdir -p ~/.config
ln -s "$(pwd)/ricing/shell/zsh/tema/starship.toml" ~/.config/starship.toml
```

Se a máquina tiver oh-my-posh ou powerlevel10k instalado de antes, `ricing/shell/install.sh` (ou `install-menu.sh` → "starship (symlink tema)") detecta e pergunta antes de desinstalar (via `pacman`/`apt` se veio de pacote, ou removendo o binário/pasta se foi instalação manual).

zsh também usa oh-my-zsh (`~/.oh-my-zsh`) — ver `plugins=(...)` e `ZSH_THEME` em `shell/zsh/.zshrc` (já versionado neste repo).

## Shell

`.zshrc`, `.bashrc`, `config.fish` — cada um sua subpasta (`shell/zsh/`, `shell/bash/`, `shell/fish/`), mesmos aliases nos três. Restaurar via `shell/install.sh` (symlink pra `~/`, backup `.bak` se já tem algo lá) ou `../install-menu.sh`. Detalhe completo no `CLAUDE.md` raiz repo (seção Ricing).

## Assinatura de commits (SSH)

`shell/.gitconfig` usa `gpg.format = ssh` — commits assinados com chave SSH dedicada, não a de autenticação. Setup por máquina nova:

```bash
ssh-keygen -t ed25519 -f ~/.ssh/id_luysfernnando_sign_commits -C "email signing"
echo "luysfernnando@gmail.com $(cat ~/.ssh/id_luysfernnando_sign_commits.pub)" > ~/.ssh/allowed_signers
```

Depois cadastra a pubkey nova no GitHub (github.com/settings/keys → "New SSH key" → tipo **Signing Key**, não Authentication). `allowed_signers` é só pra `git log --show-signature` validar localmente — GitHub verifica sozinho pelas chaves cadastradas na conta.

## GRUB silencioso

`grub/silent-boot.sh` esconde menu GRUB e mensagens de loading, silencia wall broadcast no shutdown, deixa Plymouth assumir tela — parte do "boot bonito", por isso mora aqui. Precisa `sudo`; re-rodar depois updates do pacote `grub`.