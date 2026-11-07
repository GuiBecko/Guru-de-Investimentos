# Amostras fictícias do extrato da BTG

Todas inventadas (titular "FULANO DE TAL", CPF e conta zerados). Não há dado real aqui.

- `extrato-completo.pdf`: capa, fundos (posição e detalhamento, um com dois lotes), renda fixa (CDB pré e LCA pós, com células quebradas), renda variável (Ações e BDR), conta corrente, valores em trânsito, cripto e página de disclaimers.
- `extrato-secao-desconhecida.pdf`: uma seção de renda fixa (Tesouro Direto) e uma de renda variável (Opções) que o leitor não conhece, mais uma ação válida.
- `extrato-varias-paginas.pdf`: 30 ações que passam da página 1 para a 2, com o cabeçalho de página e o de colunas repetidos.
- `outro-banco.pdf`: extrato de outro banco, que não pode ser aceito como BTG.

Os três extratos da BTG terminam com a página de disclaimers e o contato `@btgpactual.com`, como o real (o leitor exige esse texto).

Para regerar: `cargo run -p guru-core --example gerar_extratos_btg -- crates/guru-core/tests/fixtures/btg`
