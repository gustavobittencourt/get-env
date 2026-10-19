
#!/bin/bash
# Regrava assets/demo.gif e os PNGs com o vhs, num ambiente só com
# variáveis falsas e um HOME vazio: nenhum rc nem segredo real entra.
# Uso: assets/demo.sh (depois de cargo build --release)
set -euo pipefail
root=$(cd "$(dirname "$0")/.." && pwd)
work=/tmp/get-env-demo
rm -rf "$work"
mkdir -p "$work/home" "$work/bin"
ln -s "$root/target/release/get-env" "$work/bin/get-env"
cd "$root"
env -i \
    HOME="$work/home" \
    PATH="$work/bin:/opt/homebrew/bin:/usr/local/bin:/usr/bin:/bin" \
    TERM=xterm-256color \
    GET_ENV_THEME=dark \
    GET_ENV_CLIP_CMD="cat > '$work/clip'" \
    AWS_ACCESS_KEY_ID=demo-aws-access-0001 \
    AWS_REGION=sa-east-1 \
    AWS_SECRET_ACCESS_KEY=demo-aws-secret-0002 \
    DATABASE_URL=postgres://localhost:5432/app \
    EDITOR=nvim \
    GITHUB_TOKEN=demo-github-token-0003 \
    OPENAI_API_KEY=demo-openai-key-0004 \
    OPENAI_ORG_ID=org-demo \
    STRIPE_SECRET_KEY=demo-stripe-0005 \
    vhs assets/demo.tape


# O cartão ocupa só o topo da tela: corta o PNG nos primeiros 200 px.
ffmpeg -v error -y -i assets/card.png -vf crop=1200:200:0:0 "$work/card.png"
mv "$work/card.png" assets/card.png

