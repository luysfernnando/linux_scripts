#
# ~/.bashrc
#

# If not running interactively, don't do anything
[[ $- != *i* ]] && return

PS1='[\u@\h \W]\$ '

# ---------------------------
# PATH CUSTOMIZADO
# ---------------------------
export PATH="$HOME/bin:/usr/local/bin:$PATH"
export PATH="$HOME/.local/bin:$PATH"
export PATH="$HOME/.config/composer/vendor/bin:$PATH"
export PATH="$PATH:/usr/bin/elixir"
export GOPATH=$HOME/go
export PATH=$PATH:$GOPATH/bin
export BUN_INSTALL="$HOME/.bun"
export PATH="$BUN_INSTALL/bin:$PATH"

# mise (gerenciador de versão — elixir/erlang/etc, ver `mise ls`).
command -v mise >/dev/null 2>&1 && eval "$(mise activate bash)"

# ---------------------------
# STARSHIP (PROMPT)
# ---------------------------
command -v starship >/dev/null 2>&1 && eval "$(starship init bash)"

# ---------------------------
# ALIASES DE USO COMUM
# ---------------------------
alias ..='cd ..'
alias ...='cd ../..'
alias ....='cd ../../..'

alias mkdirp='mkdir -p'
alias please='sudo'  # porque "sudo" é chato de digitar
alias grep='grep --color=auto'

alias ports='sudo lsof -i -P -n | grep LISTEN'

alias editbash='code ~/.bashrc'
alias reloadbash='source ~/.bashrc'

# ---------------------------
# ALIASES PARA DEV
# ---------------------------

# ALIASES GIT
alias gs='git status'
alias ga='git add .'
alias gc='git commit -m'
alias gp='git push'
alias gpl='git pull'
alias gco='git checkout'
alias gb='git branch'
alias gd='git diff'
alias gl='git log --oneline --graph --decorate --all'

alias dev='bun run dev'
alias build='bun run build'
alias bunx='bunx'

# Corrije o Clear do terminal Kitty
alias clear='printf "\033[2J\033[3J\033[H"'

alias pas='php artisan serve'
alias pa='php artisan'
alias cache='php artisan cache:clear && php artisan view:clear && php artisan config:clear && php artisan route:clear'

alias btop='bpytop'
alias matrix='cmatrix'

# ALIASES VS CODE
alias code='/usr/bin/code'
alias agy="agy --dangerously-skip-permissions"

# ALIASES POSTGRESQL
alias pg.start='sudo systemctl start postgresql.service && sudo systemctl status postgresql.service --no-pager'
alias pg.stop='sudo systemctl stop postgresql.service && sudo systemctl status postgresql.service --no-pager'

# ALIASES MYSQL
alias mysql.start='sudo systemctl start mysqld.service && sudo systemctl status mysqld.service --no-pager'
alias mysql.stop='sudo systemctl stop mysqld.service && sudo systemctl status mysqld.service --no-pager'

# ALIASES DOCKER
alias docker.start='sudo systemctl start docker && sudo systemctl status docker --no-pager'
alias docker.stop='sudo systemctl stop docker && sudo systemctl status docker --no-pager'

# ALIASES SAMBA
alias samba.start='sudo systemctl start smb && sudo systemctl start nmb && sudo systemctl status smb --no-pager && sudo systemctl status nmb --no-pager'
alias samba.stop='sudo systemctl stop smb && sudo systemctl stop nmb && sudo systemctl status smb --no-pager && sudo systemctl status nmb --no-pager'

# ---------------------------
# LSD (LS_COLORS 100% manual, só com as 10 cores da paleta do starship —
# ver [palettes.gruvbox_dark] em ricing/shell/starship/linux.toml. Extensão
# fora das categorias abaixo fica sem cor, em vez de herdar tom fora da
# paleta. Mesmo bloco replicado em zsh/.zshrc e fish/config.fish — syntax
# de export difere por shell, então não dá pra compartilhar 1 arquivo só.)
# ---------------------------
_bg1="100;210;255"    # color_bg1 (azul claro) - diretórios
_blue="136;162;255"   # color_blue             - web (html/css)
_aqua="154;138;255"   # color_aqua             - links/imagens/docs/ícones
_green="152;151;26"   # color_green            - código-fonte/exec
_orange="190;90;255"  # color_orange           - arquivos/especiais
_purple="177;98;134"  # color_purple           - mídia (audio/vídeo)
_red="204;36;29"      # color_red              - broken/danger
_yellow="172;114;255" # color_yellow           - config/dados
export LS_COLORS="di=38;2;${_bg1}:ln=38;2;${_aqua}:ex=38;2;${_green}:or=38;2;${_red}:mi=38;2;${_red}:so=38;2;${_orange}:pi=38;2;${_orange}:bd=38;2;${_orange}:cd=38;2;${_orange}:su=38;2;${_orange}:sg=38;2;${_orange}:tw=38;2;${_orange}:ow=38;2;${_orange}:*.rs=38;2;${_green}:*.go=38;2;${_green}:*.py=38;2;${_green}:*.rb=38;2;${_green}:*.php=38;2;${_green}:*.js=38;2;${_green}:*.jsx=38;2;${_green}:*.ts=38;2;${_green}:*.tsx=38;2;${_green}:*.ex=38;2;${_green}:*.exs=38;2;${_green}:*.java=38;2;${_green}:*.kt=38;2;${_green}:*.c=38;2;${_green}:*.cpp=38;2;${_green}:*.h=38;2;${_green}:*.hpp=38;2;${_green}:*.cs=38;2;${_green}:*.swift=38;2;${_green}:*.lua=38;2;${_green}:*.pl=38;2;${_green}:*.sh=38;2;${_green}:*.bash=38;2;${_green}:*.zsh=38;2;${_green}:*.fish=38;2;${_green}:*.json=38;2;${_yellow}:*.yaml=38;2;${_yellow}:*.yml=38;2;${_yellow}:*.toml=38;2;${_yellow}:*.lock=38;2;${_yellow}:*.env=38;2;${_yellow}:*.ini=38;2;${_yellow}:*.cfg=38;2;${_yellow}:*.conf=38;2;${_yellow}:*.xml=38;2;${_yellow}:*.csv=38;2;${_yellow}:*.md=38;2;${_aqua}:*.txt=38;2;${_aqua}:*.rst=38;2;${_aqua}:*.adoc=38;2;${_aqua}:*.pdf=38;2;${_aqua}:*.doc=38;2;${_aqua}:*.docx=38;2;${_aqua}:*.odt=38;2;${_aqua}:*.rtf=38;2;${_aqua}:*Dockerfile=38;2;${_aqua}:*Makefile=38;2;${_aqua}:*Rakefile=38;2;${_aqua}:*Gemfile=38;2;${_aqua}:*Vagrantfile=38;2;${_aqua}:*Procfile=38;2;${_aqua}:*LICENSE=38;2;${_aqua}:*CHANGELOG=38;2;${_aqua}:*.png=38;2;${_aqua}:*.jpg=38;2;${_aqua}:*.jpeg=38;2;${_aqua}:*.gif=38;2;${_aqua}:*.svg=38;2;${_aqua}:*.webp=38;2;${_aqua}:*.bmp=38;2;${_aqua}:*.ico=38;2;${_aqua}:*.tiff=38;2;${_aqua}:*.mp4=38;2;${_purple}:*.mkv=38;2;${_purple}:*.avi=38;2;${_purple}:*.mov=38;2;${_purple}:*.webm=38;2;${_purple}:*.mp3=38;2;${_purple}:*.wav=38;2;${_purple}:*.flac=38;2;${_purple}:*.ogg=38;2;${_purple}:*.m4a=38;2;${_purple}:*.zip=38;2;${_orange}:*.tar=38;2;${_orange}:*.gz=38;2;${_orange}:*.bz2=38;2;${_orange}:*.xz=38;2;${_orange}:*.7z=38;2;${_orange}:*.rar=38;2;${_orange}:*.zst=38;2;${_orange}:*.tgz=38;2;${_orange}:*.html=38;2;${_blue}:*.css=38;2;${_blue}:*.scss=38;2;${_blue}:*.sass=38;2;${_blue}:*.less=38;2;${_blue}"
unset _bg1 _blue _aqua _green _orange _purple _red _yellow

# ALIASES PARA LSD
alias ls='lsd -1'
alias l='lsd -l'
alias la='lsd -1a'
alias lla='lsd -la'
alias lstree='lsd --tree' # Isso vai te dar o comportamento de 'tree' com ícones

# ---------------------------
# UPDATE (engine sysup, cross-distro)
# ---------------------------
alias update='sysup update'
alias mirrors='sysup mirrors'
alias tidewave='sysup tidewave'

# ---------------------------
# FASTFETCH
# ---------------------------
# `fastfetch --full` mostra tudo (host, board, bios, disco, som, IP...); sem
# argumento mostra o perfil enxuto. `--full` não é flag do fastfetch, é atalho
# pro segundo config gerado pelo install-menu.sh.
fastfetch() {
  local a args=()
  for a in "$@"; do
    if [[ "$a" == "--full" ]]; then
      args+=(--config full)
    else
      args+=("$a")
    fi
  done
  command fastfetch "${args[@]}"
}

# ---------------------------
# NGROK
# ---------------------------
alias ngrok.start='ngrok http 4000 --url https://candeia.ngrok.app'

# ---------------------------
# CLOUDFLARE TUNNEL
# ---------------------------
alias cf.start='cloudflared tunnel run --url http://localhost:4000 candeia'


# Added by Antigravity CLI installer
export PATH="/home/lulfex/.local/bin:$PATH"
