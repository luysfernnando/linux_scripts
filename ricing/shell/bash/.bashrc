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

# ALIASES PARA LSD
alias ls='lsd'
alias l='lsd -l'
alias la='lsd -a'
alias lla='lsd -la'
alias lstree='lsd --tree' # Isso vai te dar o comportamento de 'tree' com ícones

# ---------------------------
# UPDATE (engine sysup, cross-distro)
# ---------------------------
alias update='sysup update'
alias mirrors='sysup mirrors'
alias tidewave='sysup tidewave'

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
