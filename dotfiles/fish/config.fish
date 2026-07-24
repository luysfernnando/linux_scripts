# Aliases/exports do repo linux_scripts. Sourced no fim do ~/.config/fish/config.fish
# local (depois do source do cachyos-config.fish), pra estes aliases (lsd etc)
# vencerem os equivalentes que o CachyOS já define (eza etc). Ver install.sh.

# ---------------------------
# PATH CUSTOMIZADO (só o que falta; bun/brew/local-bin já ficam no config.fish local)
# ---------------------------
fish_add_path $HOME/.config/composer/vendor/bin
set -gx GOPATH $HOME/go
fish_add_path $GOPATH/bin

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
# ALIASES PARA LSD (sobrescreve o eza do cachyos-config, sourced antes)
# ---------------------------
alias ls 'lsd'
alias l 'lsd -l'
alias la 'lsd -a'
alias lla 'lsd -la'
alias lstree 'lsd --tree'

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
# NGROK / CLOUDFLARE TUNNEL
# ---------------------------
alias ngrok.start 'ngrok http 4000 --url https://candeia.ngrok.app'
alias cf.start 'cloudflared tunnel run --url http://localhost:4000 candeia'
