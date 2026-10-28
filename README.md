# Guru de Investimentos

Um app para computador que conhece a sua carteira, o seu perfil de investidor e o seu objetivo, e usa a IA que você escolher para responder a pergunta de todo mês: **onde coloco o próximo dinheiro?**

> **Status:** em planejamento. O escopo do MVP e o protótipo das telas estão em [`docs/`](docs/).

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
- [`docs/prototipo/index.html`](docs/prototipo/index.html): protótipo navegável das telas. Abra o arquivo no navegador. Os números são fictícios.

## Licença

[MIT](LICENSE) © 2026 Guilherme Becker
