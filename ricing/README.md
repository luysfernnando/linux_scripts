# Ricing Backup

Snapshot do visual atual (KDE Plasma 6 + kitty + shell) pra restaurar depois.

**Atalho**: `../install-menu.sh` (raiz do repo) automatiza os passos abaixo num menu
interativo (setas + `gum`) — instala o que você marcar sem copiar/colar comando por
comando. Requer `gum` (`sudo pacman -S gum`). O passo a passo manual abaixo continua
valendo pra quem prefere rodar na mão ou entender exatamente o que cada restore faz.

## KDE Plasma

| Item | Valor |
|---|---|
| Look and Feel (global theme) | `com.github.vinceliuice.Layan` |
| Color scheme | Layan |
| Ícones | Papirus-Dark |
| Cursor | Layan-white-cursors |
| Decoração de janela (Aurorae) | Layan (`__aurorae__svg__Layan`) |
| Efeito blur do KWin | ativado |

Pacotes AUR necessários (Arch):

```bash
yay -S layan-kde-git papirus-icon-theme-git layan-cursor-theme
```

Restaurar via terminal (Plasma 6):

```bash
kwriteconfig6 --file kdeglobals --group KDE --key LookAndFeelPackage "com.github.vinceliuice.Layan"
kwriteconfig6 --file kdeglobals --group Icons --key Theme "Papirus-Dark"
kwriteconfig6 --file kcminputrc --group Mouse --key cursorTheme "Layan-white-cursors"
kwriteconfig6 --file kwinrc --group org.kde.kdecoration2 --key theme "__aurorae__svg__Layan"
kwriteconfig6 --file kwinrc --group org.kde.kdecoration2 --key library "org.kde.kwin.aurorae"
kwriteconfig6 --file kwinrc --group Plugins --key blurEnabled true
# relogar ou reiniciar plasmashell pra aplicar tudo
plasmashell --replace &
```

Ou mais simples: System Settings → Appearance → Global Theme → aplicar "Layan" (aplica tema, ícones, cursor e decoração de uma vez, se instalado via `lookandfeeltool`).

```bash
lookandfeeltool -a com.github.vinceliuice.Layan
```

## Terminal (kitty)

Config em `terminal/kitty/`. Tema "Idle Toes" com fundo transparente. Pasta `terminal/`
pensada pra caber outros emuladores no futuro (konsole, etc.), não só kitty.

- `background_opacity 0.8` + `dynamic_background_opacity yes` — transparência do fundo.
- Blur visual vem do **compositor KWin** (blur effect ativado), não do kitty — kitty no Linux/X11 não suporta `background_blur` nativo (isso é exclusivo de macOS).
- `shell` usa fish se instalado, senão cai pro zsh (`sh -c 'exec "$(command -v fish || command -v zsh)"'`) — funciona em qualquer máquina independente do shell padrão.

Restaurar (symlink, não copiar — mantém o arquivo do repo como fonte da verdade):

```bash
mkdir -p ~/.config/kitty
ln -s "$(pwd)/ricing/terminal/kitty/kitty.conf" ~/.config/kitty/kitty.conf
ln -s "$(pwd)/ricing/terminal/kitty/current-theme.conf" ~/.config/kitty/current-theme.conf
```

## fastfetch

Config em `fastfetch/config.jsonc`. Logo pequeno (`CachyOS_small`) pra caber em terminal reduzido, módulos agrupados em seções (`break` entre elas): sistema → shell/terminal → ambiente gráfico (DE/WM/tema) → hardware → rede/energia.

Restaurar (symlink):

```bash
mkdir -p ~/.config/fastfetch
ln -s "$(pwd)/ricing/fastfetch/config.jsonc" ~/.config/fastfetch/config.jsonc
```

## Prompt (oh-my-posh)

Tema em `shell/zsh/tema/` (mora dentro de `shell/zsh/` porque é especificamente o prompt
do zsh, não um item solto). `.zshrc` carrega via oh-my-posh:

```bash
eval "$(oh-my-posh init zsh --config ~/.poshthemes/p10k.omp.json)"
```

Restaurar:

```bash
mkdir -p ~/.poshthemes
cp ricing/shell/zsh/tema/p10k.omp.json ~/.poshthemes/
```

zsh também usa oh-my-zsh (`~/.oh-my-zsh`) — ver `plugins=(...)` e `ZSH_THEME` em `shell/zsh/.zshrc` (já versionado neste repo).

## Shell

`.zshrc`, `.bashrc` e `config.fish` ficam cada um na sua subpasta (`shell/zsh/`,
`shell/bash/`, `shell/fish/`) — mesmo conjunto de aliases nos três. Restaurar via
`shell/install.sh` (symlinka pra `~/`, com backup `.bak` se já existir algo lá) ou pelo
`../install-menu.sh`. Detalhe completo no `CLAUDE.md` da raiz do repo (seção Ricing).

## GRUB silencioso

`grub/silent-boot.sh` esconde o menu do GRUB e as mensagens de loading, silencia o wall
broadcast no shutdown e deixa o Plymouth assumir a tela — parte do "deixar o boot
bonito", por isso mora aqui. Precisa `sudo`; re-rodar depois de updates do pacote `grub`.
