<h1 align="center">get-env</h1>

<p align="center">
  <strong>Acha a variável de ambiente que você quer pelo nome aproximado, mostra sem vazar o segredo e copia o valor.</strong>
</p>

<p align="center">
  <a href="LICENSE"><img alt="Licença MIT" src="https://img.shields.io/badge/licen%C3%A7a-MIT-a6e3a1?style=flat-square&labelColor=1e1e2e"></a>
  <img alt="Rust 2024" src="https://img.shields.io/badge/rust-edition%202024-fab387?style=flat-square&logo=rust&logoColor=fab387&labelColor=1e1e2e">
  <img alt="macOS" src="https://img.shields.io/badge/feito%20para-macOS-89b4fa?style=flat-square&logo=apple&logoColor=89b4fa&labelColor=1e1e2e">
</p>

<p align="center">
  <img src="assets/demo.gif" alt="get-env mostrando o cartão de um resultado e a tabela interativa com busca" width="820">
</p>

---

`echo $OPENAI_API_KEY` exige lembrar o nome exato e despeja o segredo inteiro na tela (e no histórico de quem está olhando). `env | grep -i openai` mostra tudo cru. O `get-env` resolve as duas coisas:

```console
$ get-env openai_key      # um resultado: cartão + valor no clipboard
$ get-env aws             # vários: tabela interativa, já filtrada
$ get-env                 # tudo, na tabela
```

## ✨ O que ele faz

- 🔎 **Busca aproximada, sem caixa.** `openai_key` ou só `oak` chegam em `OPENAI_API_KEY`.
- 📋 **Copia direto.** Com um resultado só, o valor real vai para o clipboard e um cartão confirma.
- 🔒 **Não vaza segredo na tela.** Nomes com `TOKEN`, `KEY`, `SECRET` ou `PASS` aparecem mascarados (`demo-…`); o clipboard recebe o valor inteiro.
- 🖱️ **Tabela interativa.** Teclado e mouse, busca ao vivo com `/`, um clique no `⧉` copia a linha pronta para colar no shell.
- 🎨 **Cores que combinam com o sistema.** Catppuccin Mocha no tema escuro do macOS, Latte no claro, as 16 cores do terminal quando não dá para saber, nenhuma com `NO_COLOR`.
- 🧰 **Bom em pipe.** Fora de um terminal imprime `NOME=valor` mascarado, sem cor e sem copiar nada.
- 📄 **Lê arquivos `.env`.** `get-env -f .env db` procura só no arquivo.

<p align="center">
  <img src="assets/card.png" alt="Cartão do get-env com OPENAI_API_KEY mascarada e a confirmação de cópia" width="600">
</p>

## 📦 Instalação

Precisa do Rust 1.85 ou mais novo (o crate usa a edition 2024).

```sh
cargo install --git https://github.com/gustavobittencourt/get-env
```

O binário vai para `~/.cargo/bin/get-env`.

## 🚀 Uso

```text
get-env [nome] [--file|-f <arquivo.env>]
```

| situação | o que acontece |
|---|---|
| **um** resultado, no terminal | mostra o cartão e copia o valor |
| **vários** resultados, ou sem `nome`, no terminal | abre a tabela, com o `nome` como filtro inicial |
| saída em pipe ou arquivo | imprime `NOME=valor` mascarado, uma por linha, sem copiar |
| `-f arquivo.env` | troca o ambiente pelas variáveis do arquivo |

### Como a busca escolhe

Sem diferenciar maiúsculas de minúsculas, nesta ordem:

1. o nome **igual** ao que você digitou;
2. senão, os nomes que **contêm** o texto, em ordem alfabética;
3. senão, os nomes que têm as **letras na mesma ordem**, do trecho mais curto para o mais longo (`oak` → `OPENAI_API_KEY`).

### Na tabela

<p align="center">
  <img src="assets/table.png" alt="Tabela do get-env filtrada por aws, com a linha selecionada e o status de cópia" width="820">
</p>

| tecla ou gesto | ação |
|---|---|
| `↑` `↓` ou rolagem | move a seleção |
| `⏎` ou clique no `⧉` | copia a linha `NOME='valor'`, com aspas só quando o shell precisa |
| `n` / duplo clique no nome | copia o nome |
| `v` / duplo clique no valor | copia o valor |
| `/` | busca ao vivo; `⏎` ou `↓` confirmam, `Esc` limpa |
| `Esc` | limpa o filtro; sem filtro, sai |
| `q` ou `Ctrl-C` | sai |

### Máscara

| valor | aparece como |
|---|---|
| nome sem `TOKEN`/`KEY`/`SECRET`/`PASS` | inteiro |
| sensível, até 5 caracteres | `•••` |
| sensível, mais longo | os 5 primeiros + `…` |

Caracteres de controle (`\n`, `\t`, `ESC`…) aparecem escapados, então um valor malicioso não consegue mudar a cor nem o título do terminal.

### Códigos de saída

| código | quando |
|---|---|
| `0` | deu certo |
| `1` | nenhuma variável casou, o arquivo não abriu ou a cópia falhou |
| `2` | argumentos inválidos |

## ⚙️ Configuração

| variável | efeito |
|---|---|
| `GET_ENV_THEME` | `dark`, `light` ou `ansi` forçam a paleta; sem ela, vale o tema do macOS |
| `NO_COLOR` | qualquer valor não vazio desliga as cores ([no-color.org](https://no-color.org)) |
| `GET_ENV_CLIP_CMD` | comando que recebe a cópia no stdin; o padrão é `pbcopy` |

`GET_ENV_CLIP_CMD` permite usar o `get-env` fora do macOS, por exemplo com `wl-copy` ou `xclip -selection clipboard`. Isso não foi testado: o desenvolvimento e os testes são todos no macOS.

> [!WARNING]
> `-f` **executa** o arquivo com `source` num `zsh -f` separado, para que referências como `B=$A-x` resolvam como no shell. Use só com arquivos em que você confia.

## 🛠️ Desenvolvimento

```sh
cargo fmt --check
cargo clippy -- -D warnings
cargo test
```

Os testes rodam o binário com o ambiente zerado e só com variáveis falsas, e a cópia vai para um arquivo temporário via `GET_ENV_CLIP_CMD`. Nenhum teste lê o seu ambiente nem usa o clipboard de verdade.

A demo deste README é gravada com o [VHS](https://github.com/charmbracelet/vhs), também num ambiente só com valores falsos:

```sh
cargo build --release
assets/demo.sh
```

## 📄 Licença

[MIT](LICENSE) © 2026 Gustavo Bittencourt
