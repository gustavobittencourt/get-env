#!/bin/bash
# Abre a tabela do get-env num ambiente só com variáveis falsas, para o UAT
# no terminal. O clipboard é o pbcopy real: os valores são falsos.
#
# Uso: get-env/uat.sh [nome]
#   nome       filtro opcional, como no get-env
# GET_ENV_BIN aponta para outro binário (default: get-env no PATH).
# GET_ENV_THEME (dark|light|ansi) e NO_COLOR passam adiante quando definidos.
set -euo pipefail

long=$(printf 'x%.0s' {1..300})

theme=()
[[ -n ${GET_ENV_THEME:-} ]] && theme+=("GET_ENV_THEME=$GET_ENV_THEME")
[[ -n ${NO_COLOR:-} ]] && theme+=("NO_COLOR=$NO_COLOR")

exec env -i \
    HOME="$HOME" \
    PATH="$HOME/.cargo/bin:/usr/bin:/bin" \
    TERM="${TERM:-xterm-256color}" \
    ${theme[@]+"${theme[@]}"} \
    FAKE_API_KEY=sk-fake-1234567890 \
    FAKE_PASS=abc \
    FAKE_MSG='olá mundo' \
    FAKE_MULTI=$'linha 1\nlinha 2' \
    FAKE_LONG="$long" \
    FAKE_HOME=/tmp/fake \
    FAKE_OPEN_ORG=org-fake-123 \
    'FAKE_日本=ok' \
    "${GET_ENV_BIN:-get-env}" ${1:+"$1"}
