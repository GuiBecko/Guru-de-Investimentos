# Roteiro

Ordem de construção do MVP descrito em [`mvp.md`](mvp.md). Cada fase termina com o app abrindo e fazendo algo novo de ponta a ponta. Cada uma ganha o próprio plano em `docs/superpowers/plans/` quando for começar.

| Fase | Entrega | Telas do protótipo |
|------|---------|--------------------|
| 0. Fundação | Janela do Electron com a moldura verde e o menu lateral, núcleo Rust carregado, banco SQLite criado e CI nas três plataformas. | moldura |
| 1. Carteira manual | Lançar, editar e apagar investimentos com campos fechados, IR e garantia preenchidos pelo tipo. Lista por classe e totais. | 4, 6 (sem gráfico) |
| 2. Perfil e objetivo | Questionário ANBIMA sem prévia, perguntas de consistência, alocação-alvo editável, objetivo em texto livre e metas. | 1, 2 |
| 3a. Dados de mercado | Selic, CDI, IPCA e Tesouro das fontes oficiais, cotações da B3, valor atual, rentabilidade e atualização de mercado. | 6 (sem gráfico) |
| 3b. Painel | Gráficos do painel: patrimônio × aportado ao longo do tempo, comparação com CDI, IPCA, BOVA11 e XFIX11, mês a mês, instituições, vencimentos, altas e quedas, e a saudação. | 5, 6 (gráficos) |
| 4. Importação da B3 | Ler a planilha da Área do Investidor e comparar com o cadastrado, linha a linha. | 3 |
| 5. Próximo aporte | Divisão do valor por classe por rebalanceamento, sem IA. | 7 (sem chat) |
| 6. IA | Chave no cofre do sistema, instruções próprias, editor de contexto, assistente com fontes, chat por classe, insights e notícias. | 7, 8, 9 |
| 7. Distribuição | Instaladores para Windows, macOS e Linux, atualização automática e revisão jurídica dos textos. | — |

Os planos detalhados:

- Fase 0: [`superpowers/plans/2026-10-28-fase-0-fundacao.md`](superpowers/plans/2026-10-28-fase-0-fundacao.md)
- Fase 1: [`superpowers/plans/2026-10-28-fase-1-carteira-manual.md`](superpowers/plans/2026-10-28-fase-1-carteira-manual.md)
- Fase 2: [`superpowers/plans/2026-10-29-fase-2-perfil-objetivo.md`](superpowers/plans/2026-10-29-fase-2-perfil-objetivo.md)
- Fase 3a: [`superpowers/plans/2026-10-29-fase-3a-dados-de-mercado.md`](superpowers/plans/2026-10-29-fase-3a-dados-de-mercado.md)
