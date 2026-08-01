#!/usr/bin/env bash
# Silent boot: esconde menu do GRUB e mensagens "Loading Linux .../Loading initial ramdisk ...",
# silencia o wall broadcast no shutdown/reboot, deixando o Plymouth assumir a tela direto.
#
# Precisa rodar de novo toda vez que o pacote `grub` for atualizado — o pacman
# sobrescreve /etc/grub.d/10_linux e o patch some.
set -euo pipefail

need() {
    command -v "$1" >/dev/null 2>&1 || { echo "faltando: $1" >&2; exit 1; }
}

need grub-mkconfig
need sed

if [[ $EUID -ne 0 ]]; then
    echo "roda com sudo" >&2
    exit 1
fi

echo "[1/5] GRUB_TIMEOUT_STYLE menu -> hidden"
sed -i 's/^GRUB_TIMEOUT_STYLE=menu/GRUB_TIMEOUT_STYLE=hidden/' /etc/default/grub

echo "[2/5] patch /etc/grub.d/10_linux (esvazia a mensagem de Loading Linux / Loading initial ramdisk)"
# Nao dá pra comentar a linha "echo" direto: ela vive dentro de um heredoc
# (<< EOF ... EOF) que vira texto literal do grub.cfg. Comentar ali corrompe
# a sintaxe do script GRUB gerado. O jeito seguro é esvaziar a variável
# $message ANTES do heredoc — o heredoc então emite "echo ''" (linha em
# branco), sem quebrar a estrutura.
# só faz backup se o arquivo atual ainda for o original (pristine) —
# evita sobrescrever o backup com uma versão já patchada em reruns
if grep -q 'gettext_printf "Loading Linux %s' /etc/grub.d/10_linux; then
    cp /etc/grub.d/10_linux /etc/grub.d/10_linux.bak
fi
sed -i \
    -e 's/message="\$(gettext_printf "Loading Linux %s \.\.\." \${version})"/message=""/' \
    -e 's/message="\$(gettext_printf "Loading initial ramdisk \.\.\.")"/message=""/' \
    /etc/grub.d/10_linux

echo "[3/5] EnableWallMessages=no em /etc/systemd/logind.conf"
if grep -q "^\[Login\]" /etc/systemd/logind.conf; then
    if grep -q "^EnableWallMessages=" /etc/systemd/logind.conf; then
        sed -i 's/^EnableWallMessages=.*/EnableWallMessages=no/' /etc/systemd/logind.conf
    else
        sed -i '/^\[Login\]/a EnableWallMessages=no' /etc/systemd/logind.conf
    fi
else
    printf '\n[Login]\nEnableWallMessages=no\n' >> /etc/systemd/logind.conf
fi

echo "[4/5] regenerando grub.cfg"
grub-mkconfig -o /boot/grub/grub.cfg

echo "[5/5] verificacao"
echo "--- grep 'Loading Linux' no grub.cfg (deve ficar vazio) ---"
grep -i "loading linux" /boot/grub/grub.cfg || echo "OK: nenhuma ocorrencia"
echo "--- GRUB_TIMEOUT_STYLE ---"
grep TIMEOUT_STYLE /etc/default/grub
echo "--- logind.conf ---"
grep -A1 "^\[Login\]" /etc/systemd/logind.conf

echo
echo "Feito. Reinicie pra ver o efeito."
