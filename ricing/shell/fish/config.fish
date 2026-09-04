# Aliases/exports do repo linux_scripts. Sourced no fim do ~/.config/fish/config.fish
# local (depois do source do cachyos-config.fish), pra estes aliases (lsd etc)
# vencerem os equivalentes que o CachyOS já define (eza etc). Ver install.sh.

set -g fish_greeting

# ---------------------------
# PATH CUSTOMIZADO (paridade com zsh/.zshrc e bash/.bashrc)
# ---------------------------
fish_add_path $HOME/bin
fish_add_path /usr/local/bin
fish_add_path $HOME/.local/bin
fish_add_path $HOME/.config/composer/vendor/bin
fish_add_path /usr/bin/elixir
set -gx GOPATH $HOME/go
fish_add_path $GOPATH/bin
set -gx BUN_INSTALL $HOME/.bun
fish_add_path $BUN_INSTALL/bin

# mise (gerenciador de versão — elixir/erlang/etc, ver `mise ls`). Nunca
# tinha sido ativado em shell nenhum (zsh/bash/fish) — por isso `mix` não
# existia no PATH em lugar nenhum, não é coisa específica do fish.
if command -v mise >/dev/null 2>&1
  mise activate fish | source
end

# ---------------------------
# ALIASES DE USO COMUM
# ---------------------------
alias .. 'cd ..'
alias ... 'cd ../..'
alias .... 'cd ../../..'

alias mkdirp 'mkdir -p'
alias please 'sudo'

alias ports 'sudo lsof -i -P -n | grep LISTEN'

alias editfish 'code ~/linux_scripts/dotfiles/fish/config.fish'
alias reloadfish 'source ~/.config/fish/config.fish'

# ---------------------------
# ALIASES GIT
# ---------------------------
alias gs 'git status'
alias ga 'git add .'
alias gc 'git commit -m'
alias gp 'git push'
alias gpl 'git pull'
alias gco 'git checkout'
alias gb 'git branch'
alias gd 'git diff'
alias gl 'git log --oneline --graph --decorate --all'

alias dev 'bun run dev'
alias build 'bun run build'
alias bunx 'bunx'

# Corrige o Clear do terminal Kitty
alias clear 'printf "\033[2J\033[3J\033[H"'

alias pas 'php artisan serve'
alias pa 'php artisan'
alias cache 'php artisan cache:clear; and php artisan view:clear; and php artisan config:clear; and php artisan route:clear'

alias btop 'bpytop'
alias matrix 'cmatrix'

alias code '/usr/bin/code'
alias agy 'agy --dangerously-skip-permissions'

# ALIASES POSTGRESQL
alias pg.start 'sudo systemctl start postgresql.service; and sudo systemctl status postgresql.service --no-pager'
alias pg.stop 'sudo systemctl stop postgresql.service; and sudo systemctl status postgresql.service --no-pager'

# ALIASES MYSQL
alias mysql.start 'sudo systemctl start mysqld.service; and sudo systemctl status mysqld.service --no-pager'
alias mysql.stop 'sudo systemctl stop mysqld.service; and sudo systemctl status mysqld.service --no-pager'

# ALIASES DOCKER
alias docker.start 'sudo systemctl start docker; and sudo systemctl status docker --no-pager'
alias docker.stop 'sudo systemctl stop docker; and sudo systemctl status docker --no-pager'

# ALIASES SAMBA
alias samba.start 'sudo systemctl start smb; and sudo systemctl start nmb; and sudo systemctl status smb --no-pager; and sudo systemctl status nmb --no-pager'
alias samba.stop 'sudo systemctl stop smb; and sudo systemctl stop nmb; and sudo systemctl status smb --no-pager; and sudo systemctl status nmb --no-pager'

# ---------------------------
# LSD (LS_COLORS 100% manual, só com as 10 cores da paleta do starship —
# ver [palettes.gruvbox_dark] em ricing/shell/starship/linux.toml. Sem
# vivid: qualquer extensão fora das categorias abaixo fica sem cor, em vez
# de herdar tom fora da paleta.)
# ---------------------------
set -l bg1 "100;210;255"   # color_bg1 (azul claro)   - diretórios
set -l blue "136;162;255"  # color_blue               - web (html/css)
set -l aqua "154;138;255"  # color_aqua               - links/imagens
set -l green "152;151;26"  # color_green              - código-fonte/exec
set -l orange "190;90;255" # color_orange             - arquivos/especiais
set -l purple "177;98;134" # color_purple             - mídia (audio/vídeo)
set -l red "204;36;29"     # color_red                - broken/danger
set -l yellow "172;114;255" # color_yellow            - config/dados

set -gx LS_COLORS "\
di=38;2;$bg1:\
ln=38;2;$aqua:\
ex=38;2;$green:\
or=38;2;$red:mi=38;2;$red:\
so=38;2;$orange:pi=38;2;$orange:bd=38;2;$orange:cd=38;2;$orange:su=38;2;$orange:sg=38;2;$orange:tw=38;2;$orange:ow=38;2;$orange:\
*.rs=38;2;$green:*.go=38;2;$green:*.py=38;2;$green:*.rb=38;2;$green:*.php=38;2;$green:\
*.js=38;2;$green:*.jsx=38;2;$green:*.ts=38;2;$green:*.tsx=38;2;$green:\
*.ex=38;2;$green:*.exs=38;2;$green:*.java=38;2;$green:*.kt=38;2;$green:\
*.c=38;2;$green:*.cpp=38;2;$green:*.h=38;2;$green:*.hpp=38;2;$green:*.cs=38;2;$green:\
*.swift=38;2;$green:*.lua=38;2;$green:*.pl=38;2;$green:\
*.sh=38;2;$green:*.bash=38;2;$green:*.zsh=38;2;$green:*.fish=38;2;$green:\
*.json=38;2;$yellow:*.yaml=38;2;$yellow:*.yml=38;2;$yellow:*.toml=38;2;$yellow:\
*.lock=38;2;$yellow:*.env=38;2;$yellow:*.ini=38;2;$yellow:*.cfg=38;2;$yellow:\
*.conf=38;2;$yellow:*.xml=38;2;$yellow:*.csv=38;2;$yellow:\
*.md=38;2;$aqua:*.txt=38;2;$aqua:*.rst=38;2;$aqua:*.adoc=38;2;$aqua:\
*.pdf=38;2;$aqua:*.doc=38;2;$aqua:*.docx=38;2;$aqua:*.odt=38;2;$aqua:*.rtf=38;2;$aqua:\
*Dockerfile=38;2;$aqua:*Makefile=38;2;$aqua:*Rakefile=38;2;$aqua:*Gemfile=38;2;$aqua:\
*Vagrantfile=38;2;$aqua:*Procfile=38;2;$aqua:*LICENSE=38;2;$aqua:*CHANGELOG=38;2;$aqua:\
*.png=38;2;$aqua:*.jpg=38;2;$aqua:*.jpeg=38;2;$aqua:*.gif=38;2;$aqua:\
*.svg=38;2;$aqua:*.webp=38;2;$aqua:*.bmp=38;2;$aqua:*.ico=38;2;$aqua:*.tiff=38;2;$aqua:\
*.mp4=38;2;$purple:*.mkv=38;2;$purple:*.avi=38;2;$purple:*.mov=38;2;$purple:*.webm=38;2;$purple:\
*.mp3=38;2;$purple:*.wav=38;2;$purple:*.flac=38;2;$purple:*.ogg=38;2;$purple:*.m4a=38;2;$purple:\
*.zip=38;2;$orange:*.tar=38;2;$orange:*.gz=38;2;$orange:*.bz2=38;2;$orange:\
*.xz=38;2;$orange:*.7z=38;2;$orange:*.rar=38;2;$orange:*.zst=38;2;$orange:*.tgz=38;2;$orange:\
*.html=38;2;$blue:*.css=38;2;$blue:*.scss=38;2;$blue:*.sass=38;2;$blue:*.less=38;2;$blue"

# ---------------------------
# ALIASES PARA LSD (sobrescreve o eza do cachyos-config, sourced antes)
# ---------------------------
alias ls 'lsd -1'
alias l 'lsd -l'
alias la 'lsd -1a'
alias lla 'lsd -la'
alias lstree 'lsd --tree'

# ---------------------------
# STARSHIP PROMPT (tema em ricing/shell/starship/linux.toml, symlinkado por install-zsh-plugins.sh)
# ---------------------------
starship init fish | source

# ---------------------------
# UPDATE (engine sysup, cross-distro)
# ---------------------------
alias update 'sysup update'
alias mirrors 'sysup mirrors'
alias tidewave 'sysup tidewave'
alias install 'sudo pacman -S'
alias remove 'sudo pacman -Rns'
alias search 'pacman -Ss'
alias info 'pacman -Si'

# ---------------------------
# FASTFETCH
# ---------------------------
# `fastfetch --full` mostra tudo (host, board, bios, disco, som, IP...); sem
# argumento mostra o perfil enxuto. `--full` não é flag do fastfetch, é atalho
# pro segundo config gerado pelo install-menu.sh.
function fastfetch
    set -l args
    for a in $argv
        if test "$a" = --full
            set -a args --config full
        else
            set -a args $a
        end
    end
    command fastfetch $args
end

# ---------------------------
# NGROK / CLOUDFLARE TUNNEL
# ---------------------------
alias ngrok.start 'ngrok http 4000 --url https://candeia.ngrok.app'
alias cf.start 'cloudflared tunnel run --url http://localhost:4000 candeia'
