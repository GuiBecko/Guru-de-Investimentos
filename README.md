# Guru de Investimentos

Um app para computador que conhece a sua carteira, o seu perfil de investidor e o seu objetivo, e usa a IA que você escolher para responder a pergunta de todo mês: **onde coloco o próximo dinheiro?**

> **Status:** fases 0 a 4 concluídas. A ordem das fases está em [`docs/roadmap.md`](docs/roadmap.md); o escopo do MVP e o protótipo das telas estão em [`docs/`](docs/).

## Ideia

- **Sabe o que você tem.** A carteira é montada pela planilha da Área do Investidor da B3 ou por lançamento manual, com gráficos que mostram para onde o dinheiro está indo.
- **Sabe por que você investe.** O perfil segue o método da ANBIMA, e o objetivo em texto livre e as metas com prazo viram contexto para cada análise.
- **Mostra de onde tirou cada coisa.** A IA cita as fontes de mercado com link, e você vê e edita o contexto que ela recebe.
- **Traga a sua IA.** Você conecta a sua própria chave (Claude primeiro, depois OpenAI, Gemini e modelos locais). O app só integra a IA ao seu contexto de investimentos e não cobra pelo uso dela.

## Princípios

- **Programa instalado.** A carteira e a chave da IA ficam no seu computador. Não há servidor no MVP.
- **A IA apresenta, você decide.** O app mostra opções, prós e contras e as fontes. A escolha é sempre sua. Nada aqui é recomendação individual de investimento.
- **Indicadores de fonte oficial.** Selic, CDI, IPCA e taxas do Tesouro são buscados direto nas fontes oficiais. A busca na web da IA é usada só para notícias.

## Documentação

- [`docs/mvp.md`](docs/mvp.md): telas, escopo do MVP e decisões tomadas.
- [`docs/arquitetura.md`](docs/arquitetura.md): Electron, núcleo em Rust e como os dois conversam.
- [`docs/roadmap.md`](docs/roadmap.md): ordem de construção, fase a fase.
- [`docs/prototipo/index.html`](docs/prototipo/index.html): protótipo navegável das telas. Abra o arquivo no navegador. Os números são fictícios.

## Rodando localmente

Pré-requisitos: [Rust estável](https://rustup.rs) (via `rustup`) e Node 24.

```bash
# 1. Construir o núcleo em Rust como addon do Node
cd crates/guru-node && npm install && npm run build

# 2. Rodar o app
cd ../../app && npm install && npm start
```

Testes, a partir da raiz do repositório:

```bash
cargo fmt --check && cargo clippy --workspace -- -D warnings   # formatação e lint
cargo test --workspace                                         # núcleo em Rust
(cd crates/guru-node && npm test)                              # addon
(cd app && npm test)                                           # unitários do app
(cd app && npx playwright test)                                # E2E (abre o Electron)
```

O E2E roda com as janelas escondidas (`GURU_JANELA_OCULTA=1`, definido em `app/playwright.config.js`; só vale fora do app empacotado), então nada aparece na sua tela.

No Linux sem tela, rode o E2E com `xvfb-run -a npx playwright test`.

No Ubuntu 24.04, se o Electron falhar com erro de sandbox, libere os namespaces de usuário: `sudo sysctl -w kernel.apparmor_restrict_unprivileged_userns=0`.

## Licença

[MIT](LICENSE) © 2026 Guilherme Becker
