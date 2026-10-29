# MVP do Guru de Investimentos

Escopo combinado durante a fase de protótipo. O protótipo das telas está em [`prototipo/index.html`](prototipo/index.html).

## Telas

| # | Tela | O que faz |
|---|------|-----------|
| 1 | Perfil de investidor | Questionário pelo método ANBIMA. Nenhum sinal do resultado durante o teste. Perguntas de consistência e resultado só no fim, com alocação-alvo ajustável. |
| 2 | Objetivo e metas | Objetivo em texto livre ("por que você investe, o que é sucesso"), lido pela IA em toda análise. Metas com valor, prazo, aporte mensal necessário e alerta de atraso. |
| 3 | Importar da B3 | Lê a planilha da Área do Investidor e compara com o que já está cadastrado: novo, atualizar, divergente, fora da B3, igual. O usuário escolhe o que fazer em cada linha. O que não veio na planilha nunca é apagado sozinho. |
| 4 | Lançar manualmente | Campos fechados: tipo em lista, indexador, rendimento em dois campos (% + benchmark), datas por calendário, valor em reais. Preenche IR e garantia conforme o tipo. |
| 5 | Carteira | Painel principal: saudação diferente a cada abertura, KPIs, evolução do patrimônio × total aportado, pizza por classe × alvo, rentabilidade × CDI/IPCA/Ibovespa, mês a mês, proventos, instituições, vencimentos, altas e quedas, insights da IA e notícias com link. |
| 6 | Detalhe da classe | Investimentos da classe e gráfico contra um benchmark à escolha (CDI, Selic, IPCA, Ibovespa, IFIX). |
| 7 | Próximo aporte | Divisão do valor por classe por rebalanceamento (cálculo sem IA). Cada classe abre um chat com a IA focado em escolher o investimento dentro dela. |
| 8 | Assistente | Chat com contexto da carteira, perfil, objetivo e mercado. Citações numeradas e painel com cada fonte, link e data. |
| 9 | Configurar IA | Provedor e chave do usuário, instruções próprias, privacidade e editor do contexto enviado (cada parte pode ser desligada ou editada). |

## Escopo

### MVP

- Questionário ANBIMA, alocação-alvo editável, objetivo em texto livre, metas
- Lançamento manual com campos fechados, importação da B3 com comparação, cotações automáticas
- Painel da carteira com todos os gráficos, saudação e notícias com link
- Detalhe da classe com benchmark
- Insights no painel, próximo aporte por classe, chat por classe, assistente
- Fontes citadas com link, contexto de mercado, instruções próprias, editor de contexto
- Integração com Claude (chave do usuário) e controles de privacidade

### V2

- IA sugere metas a partir do objetivo
- Importar exportação das corretoras (depois de pesquisar os formatos)
- Ler nota de corretagem em PDF com IA
- Proventos e calendário de vencimentos detalhados
- OpenAI, Gemini e modelos locais (Ollama)
- Histórico de decisões de aporte

### Depois

- Open Finance
- Cálculo de IR e DARF
- Comparar respostas de duas IAs
- Alertas no computador ou por e-mail

## Decisões

| Tema | Decisão |
|------|---------|
| Plataforma | Só computador. Nada mobile. |
| Distribuição | Programa instalado. Carteira e chave da IA ficam no computador do usuário, sem servidor no MVP. |
| Tecnologia | Electron com interface em HTML, CSS e JS puros, e núcleo em Rust. Detalhes em [`arquitetura.md`](arquitetura.md). |
| Perfil | Questionário próprio de 12 perguntas nas dimensões da ANBIMA (objetivos, situação financeira, conhecimento), sem prévia do resultado, com travas por prazo e reserva e avisos de inconsistência. Três perfis: Conservador, Moderado e Arrojado. |
| Regulatório | A IA apresenta opções, prós e contras e fontes. A decisão é sempre do usuário. Revisão jurídica dos textos antes do lançamento. |
| Dados de mercado | Indicadores buscados pelo próprio app nas fontes oficiais. Busca na web da IA só para notícias. |
| Custo da IA | O usuário usa e paga a própria IA. O app só integra. |
| Cotações | Arquivo diário oficial da B3 (série histórica), com o preço de fechamento do último pregão. Sem preço em tempo real no MVP. |
| Saudação | Escrita pelo app, sorteada de uma lista fixa de frases. Não usa a IA nem gasta tokens. |
| Contexto da IA | Cada parte do contexto (instruções, perfil, carteira, mercado, metas) pode ser desligada ou editada. |
| Classes | Renda fixa (inclui Tesouro), Ações (inclui ETFs), FIIs, Exterior (BDRs) e Outros (fundos, previdência e cripto). A classe Caixa do protótipo fica para quando houver um tipo para ela. |

## Em aberto


## Identidade visual

- Verde principal `#04321F`, sempre com textura granulada e degradê suave, no estilo do navegador Zen: moldura da janela, menu lateral, botões, mensagens do usuário.
- Conteúdo em painéis claros flutuantes com fundo de papel quadriculado leve, cantos recortados em pixel, títulos em caixa alta e traço fino (IBM Plex Sans), rótulos em IBM Plex Mono.
- Barras de progresso inteiras com cantos retos. Grade pontilhada nos gráficos.
