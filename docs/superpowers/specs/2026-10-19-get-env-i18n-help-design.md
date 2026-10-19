# get-env — idioma automático e `--help`

Data: 19/10/2026. Complementa o design original do get-env (cartão, tabela, busca, máscara e paleta), que segue valendo onde este não muda nada.

## Intenção

Hoje todo texto visível do `get-env` está fixo em português e `--help`/`-h` caem no erro de uso (rc 2). O user quer:

1. que o comando fale o idioma do ambiente sozinho, sem flag;
2. um `--help` bonito, na mesma paleta do cartão, que explique o comando inteiro.

Sucesso: `get-env --help` num terminal em português mostra a ajuda colorida em português; o mesmo comando com `LANG=en_US.UTF-8` mostra tudo em inglês, inclusive mensagens de erro, cartão e tabela.

Anti-objetivos: dependência nova de i18n (`fluent`, `gettext`, `rust-i18n`) para ~30 strings; ler o locale do ambiente real nos testes; mudar o comportamento de busca, cópia ou máscara.

## Idiomas (decisão do user)

- **pt-BR** e **en**. Qualquer locale `pt*` (`pt_BR`, `pt_PT`, `pt-BR`, `pt`) vira pt-BR.
- **Fallback: en.** Locale reconhecido mas não suportado (`fr_FR`, `es`) vira inglês.

## Detecção (decisão do user: override > POSIX > macOS)

A primeira fonte que **decide** vence:

1. `GET_ENV_LANG`: só decide com valor reconhecido (`pt*` ou `en*`); valor vazio ou desconhecido é ignorado e a busca segue, no mesmo espírito do `GET_ENV_THEME`.
2. `LC_ALL`, depois `LC_MESSAGES`, depois `LANG`: a primeira que existir, não vazia e diferente de `C`/`POSIX`, decide: `pt*` → pt-BR, qualquer outro idioma → en.
3. `AppleLanguages` do macOS (`/usr/bin/defaults read -g AppleLanguages`): o primeiro item da lista decide, com a mesma regra. Só é consultado quando nada acima decidiu.
4. Nada decidiu (sem variáveis, `defaults` ausente ou com erro): **en**.

Normalização (chamada minha): corta em `.` e `@` (`pt_BR.UTF-8@euro` → `pt_BR`), compara o prefixo do idioma sem caixa, aceita `_` e `-` como separador. A comparação com `C`/`POSIX` vem **depois** do corte: `C.UTF-8` e `POSIX.UTF-8` também não decidem.

Exemplos:

| ambiente | idioma |
|---|---|
| `GET_ENV_LANG=en`, `LANG=pt_BR.UTF-8` | en |
| `GET_ENV_LANG=klingon`, `LANG=pt_BR.UTF-8` | pt-BR |
| `LC_ALL=C`, `LANG=pt_BR.UTF-8` | pt-BR (C não decide) |
| `LC_ALL=C.UTF-8`, `LANG=pt_BR.UTF-8` | pt-BR (idem, depois do corte) |
| `LANG=fr_FR.UTF-8`, macOS em pt-BR | en (fr decide, sem suporte) |
| nenhuma variável, `AppleLanguages = (pt-BR, en-US)` | pt-BR |
| nenhuma variável, sem `defaults` | en |

## Como os testes fixam o idioma (decisão do user)

- A detecção é uma função pura que recebe as variáveis e um closure para o `AppleLanguages`, igual ao `theme::choose`; os unitários dela cobrem a tabela acima sem tocar no ambiente.
- O helper dos testes de CLI (`tests/cli.rs`) passa `GET_ENV_LANG=pt` em toda execução (o `no_query_pipe_lists_all_sorted`, que lista o ambiente inteiro, passa a esperar também essa linha); testes próprios provam o inglês com `GET_ENV_LANG=en` e com `LANG=en_US.UTF-8`. Assim nenhum teste depende do idioma da máquina que roda o `cargo test`.
- Os unitários dos módulos de saída (cartão, tabela, erros) continuam afirmando o texto em português (chamada minha: em `cfg(test)` o idioma global é pt-BR); o catálogo em inglês tem teste próprio.

## Catálogo

Um módulo `i18n` com o idioma detectado uma vez por execução e os textos dos dois idiomas no código, sem arquivo externo (chamada minha). Strings com valor no meio viram funções do catálogo. Tradução (chamada minha, revisável):

| onde | pt-BR (hoje) | en |
|---|---|---|
| cartão | `🔒 mascarado` | `🔒 masked` |
| cartão | `✅ valor copiado` | `✅ value copied` |
| cartão, tabela | `erro ao copiar: …` | `copy failed: …` |
| erro | `nenhuma variável casa com <q>` | `no variable matches <q>` |
| erro | `erro na tabela: …` | `table error: …` |
| erro | `erro ao escrever: …` | `write error: …` |
| uso | `uso: get-env [nome] [--file\|-f <arquivo.env>]` | `usage: get-env [name] [--file\|-f <file.env>]` |
| clipboard | `comando de clipboard falhou: …` / `saiu com N` / `terminou por sinal` | `clipboard command failed: …` / `exited with N` / `killed by signal` |
| `-f` | `arquivo não encontrado: …` | `file not found: …` |
| `-f` | `erro ao ler <arq>: …` / `o zsh saiu com N` / `sinal` | `error reading <file>: …` / `zsh exited with N` / `signal` |
| `-f` | `erro ao rodar o zsh: …` | `error running zsh: …` |
| tabela, título | `N variáveis` · `filtro: …` | `N variables` · `filter: …` |
| tabela, cabeçalho | `NOME`, `VALOR` | `NAME`, `VALUE` |
| tabela, rodapé | `linha`, `nome`, `valor`, `buscar`, `sair`, `Esc limpa` | `line`, `name`, `value`, `search`, `quit`, `Esc clears` |
| tabela, status | `copiado: <nome\|valor\|linha> de X` | `copied: <name\|value\|line> of X` |

Singular no título (chamada minha): `1 variável` / `1 variable`; hoje sai `1 variáveis`, e em inglês o erro ficaria mais visível.

A largura das colunas, do cartão e do rodapé já é calculada pelo texto (`width`), então o inglês não exige mudança de layout; os testes de layout em pt continuam valendo.

## `--help` (decisão do user: uso, exemplos, atalhos, variáveis)

- `-h` ou `--help` em qualquer posição → ajuda no **stdout**, **rc 0**. Nenhuma variável é coletada para listar e nenhum arquivo é lido; idioma e tema continuam detectados.
- A ajuda vence qualquer erro de uso (chamada minha): `get-env --bogus -h`, `get-env a b --help` e `get-env -f -h` mostram a ajuda com rc 0. Um arquivo chamado `-h` não é caso real.
- Flag desconhecida continua rc 2 com a linha de uso no stderr, agora traduzida e com `[-h|--help]` no fim.
- Cor só com stdout em TTY, pela mesma `Palette` do cartão (tema do macOS, `GET_ENV_THEME`, `NO_COLOR`); fora de TTY, texto puro sem nenhum `\x1b`.
- Largura fixa de até 72 colunas, sem quebra dinâmica (chamada minha).

Conteúdo em pt-BR (o en é a tradução direta):

```
get-env — acha variáveis de ambiente pelo nome aproximado

USO
  get-env [nome] [-f|--file <arquivo.env>]
  get-env -h|--help

ARGUMENTOS
  nome               trecho do nome; sem caixa, tolera erro de digitação
  -f, --file <arq>   lê as variáveis do arquivo .env, não do ambiente
  -h, --help         mostra esta ajuda

COMPORTAMENTO
  1 resultado        mostra o cartão e copia o valor
  vários ou nenhum   abre a tabela interativa, filtrada pelo nome
  em pipe            imprime NOME=valor, segredos mascarados, sem copiar

EXEMPLOS
  get-env openai             acha OPENAI_API_KEY e copia o valor
  get-env -f .env db         procura só no .env
  get-env | grep AWS         lista mascarada para outro comando

TABELA
  ↑ ↓   move        ⏎   copia NOME=valor    n   copia o nome
  v     copia o valor           /   busca     Esc limpa a busca ou sai
  q     sai         clique em ⧉ copia a linha
  duplo clique no nome ou no valor copia o que foi clicado

VARIÁVEIS
  GET_ENV_LANG       pt ou en; sem ela, LC_ALL, LC_MESSAGES, LANG e o macOS
  GET_ENV_THEME      dark, light ou ansi; sem ela, o tema do macOS
  GET_ENV_CLIP_CMD   comando que recebe a cópia no stdin (padrão: pbcopy)
  NO_COLOR           desliga as cores
```

Papéis de cor (chamada minha): `get-env` e a linha de título em `title`; cabeçalhos de seção (`USO`, `TABELA`…) em `header`; flags, nomes de variável e comandos de exemplo em `name`; teclas da tabela em `key`; descrições em `value`; comentário dos exemplos em `secondary`.

## Restrições

- Sem dependência nova no `Cargo.toml`.
- Nenhum teste lê o ambiente real nem chama `pbcopy`; clipboard sempre por `GET_ENV_CLIP_CMD` apontando para arquivo temporário.
- Gate: `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test`; E2E sintético pelo binário com `env -i`.

## Fica com o builder

- Formato do catálogo (struct com campos, `match` por chave, funções) e onde o idioma global mora (`OnceLock` ou parâmetro), desde que os unitários fiquem herméticos.
- Como `cli::parse` sinaliza ajuda versus erro de uso.
- Alinhamento exato das colunas do help, desde que fique em até 72 colunas nos dois idiomas.
