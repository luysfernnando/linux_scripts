#!/usr/bin/env bash
# Menu interativo (setas + gum) pra instalar/restaurar pedaços do ricing e o
# sysup numa máquina nova — automatiza o passo a passo manual documentado em
# ricing/README.md. Ferramenta separada do sysup em si.
set -euo pipefail

REPO_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

source "$REPO_DIR/ricing/shell/lib/log.sh"
# is_windows() + backup_and_link() — sourced aqui no topo (e não depois do
# winget) porque is_windows já é usada por need()/winget_install_if_missing.
# backup_and_link inclui o fix e a verificação do symlink no Windows
# (armadilha do MSYS documentada em lib/link.sh).
source "$REPO_DIR/ricing/shell/lib/link.sh"

need() {
  command -v "$1" >/dev/null 2>&1 || {
    echo "faltando: $1" >&2
    if is_windows; then
      echo "Instale com: winget install $1" >&2
    elif [[ -f /etc/debian_version ]]; then
      echo "Instale com: sudo apt install $1" >&2
    else
      echo "Instale com: sudo pacman -S $1" >&2
    fi
    exit 1
  }
}

# add_winget_pkg_to_path bin winget_id — winget "cria" um alias de linha de
# comando em WinGet/Links via symlink NTFS, mas isso exige Developer Mode
# ligado ou terminal elevado — sem isso ele finge sucesso e o symlink nunca
# existe. O jeito confiável é achar o .exe direto dentro de
# WinGet/Packages/<winget_id>_* (estrutura interna varia por pacote, então
# busca recursiva) e somar o dir dele no PATH. Retorna 1 se não achou nada
# instalado (nem pacote, nem exe dentro dele).
add_winget_pkg_to_path() {
  local bin="$1" winget_id="$2"
  local appdata_unix pkg_glob pkg_dir exe_path
  appdata_unix="${LOCALAPPDATA//\\//}"
  appdata_unix="$(sed -E 's#^([A-Za-z]):#/\L\1#' <<<"$appdata_unix")"
  pkg_glob="$appdata_unix/Microsoft/WinGet/Packages/${winget_id}"_*
  pkg_dir="$(compgen -G "$pkg_glob" | head -1)"
  [[ -n "$pkg_dir" ]] || return 1
  exe_path="$(find "$pkg_dir" -iname "${bin}.exe" | head -1)"
  [[ -n "$exe_path" ]] || return 1
  export PATH="$PATH:$(dirname "$exe_path")"
  hash -r
}

# winget_install_if_missing bin winget_id — no Windows (git bash), winget é o
# caminho normal pra instalar sozinho em vez de só reclamar, já que não tem
# pacman/apt pra sugerir.
winget_install_if_missing() {
  local bin="$1" winget_id="$2"
  is_windows || return 0
  command -v "$bin" >/dev/null 2>&1 && return 0

  # Já instalado por uma sessão anterior, só que o PATH desta sessão nova
  # ainda não sabe disso (o fix de PATH abaixo só vale pro processo atual) —
  # acha e usa sem chamar o winget de novo.
  add_winget_pkg_to_path "$bin" "$winget_id" && return 0

  need winget
  echo "faltando: $bin — instalando via winget..." >&2
  winget install --id "$winget_id" -e --source winget --accept-source-agreements --accept-package-agreements
  add_winget_pkg_to_path "$bin" "$winget_id"
}

winget_install_if_missing gum charmbracelet.gum
need gum


action_dotfiles() {
  log_step "Shell (.bashrc/fish)"
  bash "$REPO_DIR/ricing/shell/install.sh"
}

action_kitty() {
  log_step "kitty"
  for f in kitty.conf current-theme.conf; do
    backup_and_link "$REPO_DIR/ricing/terminal/kitty/$f" "$HOME/.config/kitty/$f"
  done
}

action_kde_theme() {
  log_step "Tema KDE (Layan)"
  if ! command -v kwriteconfig6 >/dev/null 2>&1; then
    log_warn "kwriteconfig6 não encontrado — isso não parece uma sessão KDE Plasma 6, pulando."
    return 0
  fi
  gum confirm "Isso assume que layan-kde-git, papirus-icon-theme-git e layan-cursor-theme já estão instalados (AUR). Continuar?" || {
    log_warn "cancelado (tema KDE)"
    return 0
  }
  kwriteconfig6 --file kdeglobals --group KDE --key LookAndFeelPackage "com.github.vinceliuice.Layan"
  kwriteconfig6 --file kdeglobals --group Icons --key Theme "Papirus-Dark"
  kwriteconfig6 --file kcminputrc --group Mouse --key cursorTheme "Layan-white-cursors"
  kwriteconfig6 --file kwinrc --group org.kde.kdecoration2 --key theme "__aurorae__svg__Layan"
  kwriteconfig6 --file kwinrc --group org.kde.kdecoration2 --key library "org.kde.kwin.aurorae"
  kwriteconfig6 --file kwinrc --group Plugins --key blurEnabled true
  plasmashell --replace >/dev/null 2>&1 &
  log_ok "tema aplicado (plasmashell reiniciando em segundo plano)"
}

# Cada terminal pede um protocolo de imagem diferente no fastfetch — "auto" do
# fastfetch não acerta de forma confiável (viu na prática: escolheu kitty
# dentro do WSL e caiu pra bytes crus na tela).
is_wsl() { grep -qi microsoft /proc/version 2>/dev/null || [[ -n "${WSL_DISTRO_NAME:-}" ]]; }

# fastfetch_logo_type — protocolo por terminal, não só por SO:
#   kitty nativo -> kitty
#   Windows      -> raw (sixel PRÉ-CONVERTIDO, ver render_sixel_logo)
#   WSL          -> sixel (lá o fastfetch acha a libMagickCore certa)
#
# No Windows o `sixel` do fastfetch não funciona: ele dlopen
# libMagickCore-7.Q16HDRI-10.dll (naming do MSYS2, ABI do 7.1.1) e o instalador
# oficial entrega CORE_RL_MagickCore_.dll do 7.1.2 — renomear tira o "library
# not found" mas cai em "Failed to load / convert", porque a ABI difere. `iterm`
# só serve pro WezTerm (o Windows Terminal não fala iTerm2, imprime um "m"
# solto). Solução: converter com o chafa e imprimir o .sixel com
# `--logo-type raw`, que despeja o arquivo byte a byte. `file-raw` NÃO serve:
# trata o arquivo como linhas de texto e injeta escapes no meio do blob DCS,
# embaralhando a tela. Como o WezTerm também renderiza sixel, `raw` cobre os
# dois terminais do Windows.
fastfetch_logo_type() {
  if is_wsl; then
    echo sixel
  elif is_windows; then
    echo raw
  else
    echo kitty
  fi
}

# fastfetch_os_icon — ícone de SO pro @OS_ICON@ do template.
#
# O template fixa um ícone por módulo (todos do set Material Design, pra ficarem
# do mesmo tamanho visual — ver ricing/README.md), mas o de SO é o único que tem
# que variar por plataforma.
#
# Bytes UTF-8 crus e não o glifo literal (nem `\U`, que o printf do bash não
# entende): esses codepoints estão no plano 1 do Unicode e não sobrevivem a
# edição por ferramenta que normaliza texto.
fastfetch_os_icon() {
  if is_windows; then
    printf '\xf3\xb0\x96\xb3' # U+F05B3 md-microsoft_windows
  elif [[ "$(uname -s)" == Darwin ]]; then
    printf '\xf3\xb0\x80\xb5' # U+F0035 md-apple
  else
    printf '\xf3\xb0\x8c\xbd' # U+F033D md-linux
  fi
}

# render_sixel_logo <png> <rows> — converte o PNG pra sixel com o chafa e ecoa
# `<path> <largura_px> <altura_px>`. Falha (status 1) se o chafa não existir ou
# a conversão der errado.
#
# chafa e não ImageMagick: sixel não tem canal alpha, e o magick achata os
# pixels transparentes numa cor (branco por padrão, daí a borda serrilhada;
# achatar na cor do terminal troca a borda por um retângulo opaco, que aparece
# porque o terminal usa opacity 80). O chafa aproveita o `P2=1` do header DCS,
# que deixa pixel não pintado transparente de verdade — o fundo do terminal
# atravessa. Verificado decodificando o blob: chafa pinta ~60% da área, o
# magick pinta 100%.
#
# A altura em px tem que casar com a grade do terminal, senão o texto do
# fastfetch sobrepõe a imagem: `raw` não sabe o tamanho do blob, só reserva as
# células que a config declarar. O chafa dimensiona em células assumindo uma
# célula própria (não detecta a real quando a saída é um arquivo), então quem
# manda são as dimensões em px que ele grava no header — é de lá que sai a
# reserva, não de cálculo nosso.
render_sixel_logo() {
  local png="$1" rows="$2" out src attrs
  out="$HOME/.config/fastfetch/${FASTFETCH_LOGO_IMG%.*}.sixel"
  command -v chafa >/dev/null 2>&1 || return 1
  src="$png"
  is_windows && src="$(cygpath -m "$png")"

  # Largura folgada de propósito: a altura é que limita, e a proporção da imagem
  # decide o resto. --polite/--animate desligam sequências de controle que só
  # fazem sentido em terminal ao vivo.
  # `--colors full` e não 256: no chafa, 256 significa a paleta FIXA do xterm
  # (#0=0,0,0 #1=50,0,0 #2=0,43,0...), que não tem os tons da imagem e enche
  # tudo de chuvisco por mais dither que se aplique. `full` faz ele montar uma
  # paleta pra esta imagem — o sixel ainda cabe nos 256 registradores, só que
  # com as cores certas. Com paleta boa o dither deixa de ser necessário.
  chafa --format sixel --size "200x$rows" \
    --colors full --dither none \
    --polite on --animate off "$src" > "$out" 2>/dev/null || return 1
  [[ -s "$out" ]] || return 1

  # Raster attributes do header: `"<pan>;<pad>;<largura>;<altura>`.
  attrs="$(head -c 64 "$out" | tr -d '\0' | sed -n 's/.*"[0-9]*;[0-9]*;\([0-9]*\);\([0-9]*\).*/\1 \2/p')"
  [[ -n "$attrs" ]] || return 1
  echo "$out $attrs"
}

# Célula do terminal em px — 10x20 pro JetBrainsMono NF no tamanho padrão do
# Windows Terminal. Usado só pra dimensionar o sixel: valor pequeno demais deixa
# vão em branco entre a imagem e o prompt, grande demais faz o texto passar por
# cima da imagem.
#
# Medir num terminal novo (fonte ou tamanho diferente muda isso) — a resposta
# vem como ESC[6;<altura>;<largura>t:
#
#   $e=[char]27; [Console]::Write("$e[16t"); Start-Sleep -Milliseconds 300
#   $s=''; while([Console]::KeyAvailable){ $s += [Console]::ReadKey($true).KeyChar }; $s
FASTFETCH_CELL_PX_W=10
FASTFETCH_CELL_PX_H=20

# Logo: nome de um arquivo em ricing/fastfetch/images/. Trocar a imagem = trocar
# esta linha e rodar a ação "fastfetch" do menu; o resto (sixel, largura,
# altura) é recalculado.
FASTFETCH_LOGO_IMG=165.png

# Altura do logo em linhas do terminal — é o tamanho da imagem. Só um palpite
# inicial: render_fastfetch_config mede quantas linhas os módulos realmente
# imprimem nesta máquina e reajusta. Serve de fallback se a medição falhar.
FASTFETCH_LOGO_ROWS=16

# `logo.padding.top` do template. Entra na conta da altura: a imagem começa uma
# linha abaixo do título, então ocupa padding + altura.
FASTFETCH_LOGO_PADDING_TOP=1

# write_fastfetch_configs tipo path largura altura backup — gera os dois .jsonc.
#
# `backup` só na primeira escrita: render_fastfetch_config escreve duas vezes, e
# sem isso a segunda faria backup do arquivo que a primeira acabou de gerar.
write_fastfetch_configs() {
  local logo_type="$1" logo_path="$2" logo_w="$3" logo_h="$4" backup="$5"
  local dst_dir="$HOME/.config/fastfetch" name tmpl dst

  mkdir -p "$dst_dir"
  for name in config full; do
    tmpl="$REPO_DIR/ricing/fastfetch/$name.jsonc.tmpl"
    dst="$dst_dir/$name.jsonc"
    if [[ "$backup" == "yes" && -e "$dst" && ! -L "$dst" ]]; then
      log_warn "backup: $dst -> $dst.bak"
      mv "$dst" "$dst.bak"
    fi
    rm -f "$dst" # se ainda for symlink de uma instalação antiga, troca por arquivo real

    sed -e "s/@LOGO_TYPE@/$logo_type/" \
      -e "s|@LOGO_PATH@|$logo_path|" \
      -e "s/@LOGO_W@/$logo_w/" \
      -e "s/@LOGO_H@/$logo_h/" \
      -e "s/@OS_ICON@/$(fastfetch_os_icon)/" \
      "$tmpl" > "$dst"
  done
}

# count_fastfetch_lines — quantas linhas o perfil enxuto imprime NESTA máquina.
#
# Não dá pra deduzir do template: módulo que não encontra nada (DE num Windows,
# battery/poweradapter num desktop) não imprime linha nenhuma, e é justamente
# essa diferença que virava vão em branco entre a imagem e o prompt.
count_fastfetch_lines() {
  local cfg="$HOME/.config/fastfetch/config.jsonc"
  is_windows && cfg="$(cygpath -m "$cfg")"
  fastfetch --config "$cfg" --logo none 2>/dev/null | wc -l
}

# render_fastfetch_config — gera (não symlinka) ~/.config/fastfetch/{config,full}.jsonc
# a partir dos templates do repo, substituindo @LOGO_TYPE@ pelo protocolo certo e
# @LOGO_PATH@ pelo path absoluto do logo nessa máquina (no Windows o
# fastfetch.exe não entende o path POSIX do Git Bash). É um COPY de propósito,
# não symlink: os dois valores variam por máquina, então symlinkar geraria diff
# no git toda vez que WSL != nativo.
#
# São dois perfis: `config` é o de todo dia, `full` tem tudo e sai no
# `fastfetch --full` (ver fastfetch_wrapper_* nos configs de shell).
render_fastfetch_config() {
  local logo_type logo_path png rows pass logo_w logo_h
  logo_type="$(fastfetch_logo_type)"
  png="$HOME/.config/fastfetch/images/$FASTFETCH_LOGO_IMG"
  rows="$FASTFETCH_LOGO_ROWS"

  # Duas passadas: a primeira só pra existir um config que o fastfetch aceite,
  # a segunda com a altura medida. Sem a primeira não há o que medir.
  for pass in 1 2; do
    local sixel px_w px_h
    logo_w=40
    logo_h="$rows"
    logo_path="$png"

    if [[ "$logo_type" == "raw" ]]; then
      if read -r sixel px_w px_h < <(render_sixel_logo "$png" "$rows") && [[ -n "$px_h" ]]; then
        logo_path="$sixel"
        # Reserva em células arredondada pra cima a partir das dimensões que o
        # chafa gravou no header — arredondar pra baixo deixaria o texto ou o
        # prompt por cima da última faixa da imagem.
        logo_w=$(((px_w + FASTFETCH_CELL_PX_W - 1) / FASTFETCH_CELL_PX_W))
        logo_h=$(((px_h + FASTFETCH_CELL_PX_H - 1) / FASTFETCH_CELL_PX_H))
      else
        log_warn "chafa não encontrado — sem ele não dá pra converter o logo pra sixel"
        log_dim "instale com: winget install hpjansson.Chafa"
        logo_type=builtin
      fi
    fi

    # Forward slash mesmo no Windows: o fastfetch aceita, e evita ter que
    # escapar backslash dentro do JSON.
    is_windows && logo_path="$(cygpath -m "$logo_path")"
    write_fastfetch_configs "$logo_type" "$logo_path" "$logo_w" "$logo_h" \
      "$([[ $pass -eq 1 ]] && echo yes || echo no)"

    [[ $pass -eq 2 ]] && break

    local lines target
    lines="$(count_fastfetch_lines)"
    target=$((lines - FASTFETCH_LOGO_PADDING_TOP))
    if [[ "$lines" -gt "$FASTFETCH_LOGO_PADDING_TOP" && "$target" -ne "$rows" ]]; then
      log_dim "módulos imprimem $lines linhas aqui — ajustando logo de $rows pra $target"
      rows="$target"
    else
      break # já casa, ou a medição falhou: fica no palpite
    fi
  done
  log_ok "~/.config/fastfetch/{config,full}.jsonc (logo.type=$logo_type, ${logo_w}x${logo_h})"

  if [[ "$logo_type" == "sixel" ]] && ! command -v magick >/dev/null 2>&1 && ! command -v convert >/dev/null 2>&1; then
    log_warn "sixel precisa de imagemagick pra decodificar a imagem — sem ele o logo não aparece"
  fi
}

action_fastfetch() {
  log_step "fastfetch"
  winget_install_if_missing fastfetch Fastfetch-cli.Fastfetch
  need fastfetch
  # Converte o logo pra sixel preservando transparência (ver render_sixel_logo).
  is_windows && winget_install_if_missing chafa hpjansson.Chafa
  # images/ antes do config: render_fastfetch_config lê o PNG pra converter em
  # sixel e medir o aspect ratio.
  backup_and_link "$REPO_DIR/ricing/fastfetch/images" "$HOME/.config/fastfetch/images"
  backup_and_link "$REPO_DIR/ricing/fastfetch/presets" "$HOME/.config/fastfetch/presets"
  render_fastfetch_config
}

action_starship() {
  log_step "starship"
  # shellcheck source=ricing/shell/lib/install-cli-tools.sh
  source "$REPO_DIR/ricing/shell/lib/install-cli-tools.sh"
  # shellcheck source=ricing/shell/lib/install-shell-tools.sh
  source "$REPO_DIR/ricing/shell/lib/install-shell-tools.sh"
  detect_and_offer_uninstall_prompt_managers
  install_starship
}

action_git() {
  log_step "Git (assinatura SSH + ssh-agent)"
  # shellcheck source=ricing/shell/lib/install-git-ssh.sh
  source "$REPO_DIR/ricing/shell/lib/install-git-ssh.sh"

  if check_sign_key; then
    log_ok "chave de assinatura ($SIGN_KEY.pub)"
  else
    log_warn "faltando: chave de assinatura de commit"
    gum confirm "Gerar chave de assinatura agora?" && install_sign_key
  fi

  if check_allowed_signers; then
    log_ok "allowed_signers"
  else
    log_warn "faltando: allowed_signers"
    install_allowed_signers
  fi

  if check_ssh_agent_socket; then
    log_ok "ssh-agent.socket (systemd)"
  else
    log_warn "faltando: ssh-agent.socket habilitado"
    install_ssh_agent_socket
  fi

  if check_keys_loaded; then
    log_ok "chaves carregadas no ssh-agent"
  else
    log_warn "faltando: chaves no ssh-agent"
    install_keys_loaded
  fi
}

action_sysup() {
  log_step "sysup (primeira instalação)"
  # shellcheck source=ricing/shell/lib/install-sysup.sh
  source "$REPO_DIR/ricing/shell/lib/install-sysup.sh"
  install_sysup
}

action_claude() {
  log_step "Claude Code (skills/rules/RTK.md/settings.json/filtros rtk)"
  bash "$REPO_DIR/claude/install.sh"
}

gum style --bold --foreground 212 --border rounded --padding "0 2" --margin "1 0" \
  "install-menu.sh — setup de ambiente"

choice="$(gum choose \
  --header "O que instalar/configurar? (setas + enter escolhe 1; rode de novo pra outra opção)" \
  --cursor.foreground 212 --selected.foreground 42 \
  "Shell (.bashrc/fish)" \
  "kitty (symlink)" \
  "Tema KDE (Layan)" \
  "fastfetch (config gerado + symlink)" \
  "starship (symlink tema)" \
  "Git (assinatura SSH + ssh-agent)" \
  "sysup (primeira instalação)" \
  "Claude Code (skills/rules/configs)")"

if [[ -z "$choice" ]]; then
  log_dim "nada selecionado, saindo."
  exit 0
fi

gum confirm "Aplicar \"$choice\" agora?" || {
  log_dim "cancelado."
  exit 0
}

echo
case "$choice" in
  "Shell (.bashrc/fish)") action_dotfiles ;;
  "kitty (symlink)") action_kitty ;;
  "Tema KDE (Layan)") action_kde_theme ;;
  "fastfetch (config gerado + symlink)") action_fastfetch ;;
  "starship (symlink tema)") action_starship ;;
  "Git (assinatura SSH + ssh-agent)") action_git ;;
  "sysup (primeira instalação)") action_sysup ;;
  "Claude Code (skills/rules/configs)") action_claude ;;
esac

echo
log_box "Concluído: $choice" 42

# Recarrega o shell sozinho: como este script roda como processo filho, não
# dá pra dar "source ~/.bashrc" no shell que te chamou daqui de dentro — o
# jeito é substituir este processo por um shell novo (exec), que já nasce
# lendo o .bashrc/config.fish atualizado. No Windows isso não faz sentido: o
# script já roda dentro do Git Bash chamado pelo install-menu.ps1 a partir do
# PowerShell, e um exec aqui troca o processo por outro Git Bash em vez de
# voltar pro PowerShell de quem chamou.
if ! is_windows; then
  log_dim "recarregando shell..."
  exec "${SHELL:-/bin/bash}" -l
fi
