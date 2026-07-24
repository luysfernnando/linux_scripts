# Ricing Backup

Snapshot do visual atual (KDE Plasma 6 + kitty + shell) pra restaurar depois.

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

## kitty

Config em `kitty/`. Tema "Idle Toes" com fundo transparente.

- `background_opacity 0.8` + `dynamic_background_opacity yes` — transparência do fundo.
- Blur visual vem do **compositor KWin** (blur effect ativado), não do kitty — kitty no Linux/X11 não suporta `background_blur` nativo (isso é exclusivo de macOS).

Restaurar:

```bash
mkdir -p ~/.config/kitty
cp ricing/kitty/kitty.conf ricing/kitty/current-theme.conf ~/.config/kitty/
cp -r ricing/kitty/themes ~/.config/kitty/
```

## Prompt (oh-my-posh)

`.zshrc` carrega tema p10k via oh-my-posh:

```bash
eval "$(oh-my-posh init zsh --config ~/.poshthemes/p10k.omp.json)"
```

Restaurar:

```bash
mkdir -p ~/.poshthemes
cp ricing/oh-my-posh/p10k.omp.json ~/.poshthemes/
```

zsh também usa oh-my-zsh (`~/.oh-my-zsh`) — ver `plugins=(...)` e `ZSH_THEME` em `dotfiles/.zshrc` (já versionado neste repo).
