#!/bin/bash
# Abre a tabela do get-env num ambiente só com variáveis falsas, para o UAT
# no terminal. O clipboard é o pbcopy real: os valores são falsos.
#
# Uso: get-env/uat.sh [end|start] [nome]
#   end|start  posição do ícone de copiar (default: end)
#   nome       filtro opcional, como no get-env
# GET_ENV_BIN aponta para outro binário (default: get-env no PATH).
set -euo pipefail

long=$(printf 'x%.0s' {1..300})

exec env -i \
    HOME="$HOME" \
    PATH="$HOME/.cargo/bin:/usr/bin:/bin" \
    TERM="${TERM:-xterm-256color}" \
    GET_ENV_ICON_POS="${1:-end}" \
    FAKE_API_KEY=sk-fake-1234567890 \
    FAKE_PASS=abc \
    FAKE_MSG='olá mundo' \
    FAKE_MULTI=$'linha 1\nlinha 2' \
    FAKE_LONG="$long" \
    FAKE_HOME=/tmp/fake \
    "${GET_ENV_BIN:-get-env}" ${2:+"$2"}
