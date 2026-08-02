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

## fastfetch

Config em `fastfetch/config.jsonc`. Logo `images/165.png` (kitty graphics protocol), módulos agrupados em seções (`break` entre elas): sistema → shell/terminal → ambiente gráfico (DE/WM/tema) → hardware → rede/energia. `images/` (250 PNGs) e `presets/` (jsonc) vêm de https://github.com/Maheswara660/fastfetch.

Restaurar (symlink):

```bash
mkdir -p ~/.config/fastfetch
ln -s "$(pwd)/ricing/fastfetch/config.jsonc" ~/.config/fastfetch/config.jsonc
ln -s "$(pwd)/ricing/fastfetch/images" ~/.config/fastfetch/images
ln -s "$(pwd)/ricing/fastfetch/presets" ~/.config/fastfetch/presets
```

## Prompt (oh-my-posh)

Tema em `shell/zsh/tema/` (mora dentro `shell/zsh/` — específico prompt zsh, não item solto). `.zshrc` carrega via oh-my-posh:

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