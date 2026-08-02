# linux_scripts

Scripts Linux pessoais (automações, ricing, utilitários linha de comando) — centralizados, versionados, reuso entre máquinas.

**Conteúdo:**
- `sysup/` — engine update cross-distro em Go (ver `CLAUDE.md`, seção sysup)
- `ricing/` — dotfiles, KDE, kitty, fastfetch, GRUB (ver `ricing/README.md`)
- `claude/` — skills Claude Code, instaláveis via `claude/install.sh`
- `docker/` — helpers Docker (install, compose Postgres)
- `install-menu.sh` — menu interativo (gum), instala pedaços acima em máquina nova

**Uso:**
```bash
chmod +x install-menu.sh
./install-menu.sh
```
Leia topo de cada script pra dependências/opções — maioria exige `sudo` pra passos sistema.

Uso pessoal; issues/PRs bem-vindos. Licença: [LICENSE.md](LICENSE.md).