# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Purpose

Personal collection of Linux shell scripts for automating environment setup and tooling on Arch/Debian-based systems. Scripts are standalone — no build system, no test suite.

## Running Scripts

All scripts need execute permission before first use:

```bash
chmod +x <script>.sh
./<script>.sh
```

Read the top of each script for dependencies and options before running — most require `sudo` for system-level steps.

## Scripts Overview

| Script | What it does |
|---|---|
| `docker/install-docker.sh` | apt update → get.docker.com install → add user to docker group → reboot |
| `docker/postgres/docker-compose.yml` | Local Postgres tuned for heavy queries (2GB shared_buffers, NVMe settings) |
| `gitkraken/gitkraken-install-or-update.sh` | Downloads latest `.tar.gz`, safe-swaps `/opt/gitkraken`, creates `/usr/local/bin/gitkraken` wrapper |
| `tidewave/tidewave.sh` | Manages `tidewave-cli` binary: `install`, `update`, `fix-codex-acp`, or runs CLI directly |

## Conventions

- Bash scripts use `set -euo pipefail` and a `need()` guard for dependency checks.
- GitKraken wrapper supports `UPDATE_ON_START=1 gitkraken` to auto-update on launch.
- Tidewave defaults to port `8000`; pass `-p <port>` to override.
- Postgres compose uses a named volume `pgdata` — data persists across `docker compose down`.

## Dotfiles

`dotfiles/` contém `.zshrc`, `.bashrc`, `.gitconfig` — linkados via symlink pra `~/`.

**Setup em máquina nova:**
```bash
git clone <repo> ~/linux_scripts
cd ~/linux_scripts/dotfiles
bash install.sh
```

`install.sh` faz backup do arquivo original (`*.bak`) se não for symlink, depois cria o link. Adicionar novo dotfile: copiar pra `dotfiles/`, incluir o nome em `FILES` no `install.sh`.

## Adding New Scripts

Place scripts in a subdirectory named after the tool/category. Follow the `need()` pattern for dependency checks and `set -euo pipefail` at the top.
