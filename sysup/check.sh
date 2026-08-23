#!/usr/bin/env bash
# Gate único de pré-commit pro workspace sysup: fmt, clippy, testes
# (unit + integração, incluindo self-update ponta a ponta contra um
# mock local da API do GitHub). Sucesso imprime só um "✔" por etapa;
# falha despeja o log completo daquela etapa e para.
set -uo pipefail
cd "$(dirname "$0")"

export SYSUP_VERSION=0.0.1

run_stage() {
    local name="$1"
    shift
    local log
    log="$(mktemp)"
    if "$@" >"$log" 2>&1; then
        echo "✔ $name"
        rm -f "$log"
    else
        echo "✘ $name falhou:" >&2
        cat "$log" >&2
        rm -f "$log"
        exit 1
    fi
}

run_stage "cargo fmt --check" cargo fmt --check
run_stage "cargo clippy" cargo clippy --workspace --all-targets -- -D warnings
run_stage "cargo test" cargo test --workspace

echo "tudo passou"
