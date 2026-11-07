# Importação do extrato da BTG (PDF) — Design

## Por quê

O app já importa a planilha de posição da B3, mas a B3 não enxerga tudo: fundos de investimento, por exemplo, não aparecem lá. O usuário tem conta na BTG Investimentos e o "Extrato da Conta Investimento" em PDF da BTG mostra a carteira inteira daquela conta. Importar esse PDF faz o app captar todos os investimentos, sem cadastro manual.

Sucesso: o usuário escolhe o PDF da BTG, revisa a lista na mesma tela de revisão que já conhece da B3, confirma, e a carteira passa a ter os fundos, CDBs, ações, BDRs e criptoativos da BTG — sem duplicar o que já veio da B3 ou foi lançado à mão.

## Objetivos

- Ler o PDF "Extrato da Conta Investimento" da BTG (exemplo real: extrato de alguns dias, 13 páginas, gerado por PyPDF2, texto extraível — não é imagem).
- Mostrar o resultado na **mesma tela de revisão da importação da B3**, com os mesmos estados por linha (novo, atualizar, divergente, igual) e as mesmas decisões do usuário, e aplicar tudo numa transação só, como a B3.
- **Não duplicar**: antes de sugerir "novo", o app procura o ativo na carteira pela identidade dele. Se já existe, o item vira igual/atualizar/divergente, nunca um segundo cadastro. Isso vale também entre fontes: um PETR4 ou um CDB que veio da B3 é reconhecido quando aparece no PDF da BTG, e vice-versa.
- **Dois botões separados**: um para importar a planilha da B3 (o que existe hoje) e outro para importar o PDF da BTG.
- **Um investimento por ativo e por corretora**, nas duas importações.
- **Valor atual de fundos e cripto** vindo do PDF, em vez de só o valor aplicado.
- **Nenhuma importação sugere apagar** o que não veio no arquivo (sai o estado "fora da B3").

## Não objetivos

- Outras corretoras ou outros relatórios da BTG (nota de corretagem, informe de rendimentos).
- Histórico de movimentações: o extrato é lido como **posição** numa data, não como lista de compras e vendas.
- Conta corrente e valores em trânsito não viram investimento (no máximo um aviso no topo da revisão).
- Leitura por OCR ou por IA. O texto do PDF sai limpo.

## Anti-objetivos (o que tornaria isso um fracasso mesmo funcionando)

- **Duplicar investimentos** que já estão na carteira.
- **Sugerir apagar** qualquer coisa porque não veio no arquivo. A BTG só enxerga a própria conta; o Tesouro de outra corretora não está ali e isso é normal. **Decisão do usuário: nenhuma das duas revisões (BTG e B3) tem o estado "fora do arquivo".**
- Importar um valor errado em silêncio. Se o leitor não reconhece uma seção ou uma linha, ela vira aviso na revisão, nunca um palpite gravado.
- Dados pessoais do usuário (nome, CPF, endereço, número da conta) no repositório. As amostras de teste são PDFs fictícios gerados para isso, como as amostras da B3 em `crates/guru-core/tests/fixtures/b3/`.

## O que o PDF traz (conferido no exemplo real)

Cabeçalho: nome, conta, CPF, endereço, "Período de dd/mm/aa a dd/mm/aa", "Emitido em dd/mm/aa hh:mm". Datas com **ano de dois dígitos**. Números no formato `1.234,56`. Campo vazio é `-`. Cada página repete o cabeçalho "Extrato da Conta / Período ... / Investimento", o número da página e uma linha de "Disclaimers". As últimas páginas são avisos legais e contatos.

Seções e o que cada uma dá:

| Seção | Campos úteis | Vira |
|-------|--------------|------|
| **Fundo de Investimento - Posição** | nome do fundo + `Classe CNPJ: xx.xxx.xxx/xxxx-xx`, data de referência, quantidade de cotas (fracionária), cotação atual, saldo bruto, provisão de IR/IOF, saldo líquido | tipo `fundo` |
| **Fundo de Investimento - Detalhamento - <fundo>** | por aplicação: data de compra, cotas, cotação de compra, **valor de compra ajustado** | data de aplicação e valor aplicado |
| **Renda fixa - Posição - CDB** (e, por analogia, outras subseções de renda fixa) | emissor, ativo (`CDB-CDBA26FICT1`), emissão, vencimento, liquidez, taxa (`12,75% a.a.`, `103,00% do CDI`), quantidade, preço, saldo bruto/IR/IOF/líquido | tipo `rf-*` |
| **Renda fixa - Detalhamento - CDB \| <emissor>** | por aquisição: data de aquisição, taxa de compra, quantidade, preço de compra, **valor de compra** | data de aplicação e valor aplicado |
| **Renda variável - Posição - Ações** / **- BDR** (e, por analogia, FII/ETF se vierem) | código (ticker, às vezes com `*`), nome, quantidade, preço de fechamento, **preço médio**, saldo bruto | tipo `rv-acao`, `rv-bdr`, … |
| **CriptoAtivos** | no exemplo só existe "Movimentação" (compra e venda no mês, sem saldo). A seção de **posição** quando há saldo não foi vista. | tipo `cripto` |
| Conta corrente, Valores em trânsito, Sumário, Distribuição, Rentabilidade, Perfil de risco, Disclaimers | — | ignorados (conta corrente e trânsito podem virar aviso) |

Diferença importante para a B3: o PDF da BTG traz **valor aplicado, taxa contratada, data de compra e preço médio** — a planilha da B3 não traz. Então a sugestão de um item "novo" vem bem mais completa e o usuário tem menos campos para preencher. Ações e BDRs são exceção: **não têm data de compra** no PDF.

## Como funciona (decidido com o usuário)

1. Na interface há **dois botões separados**: "Importar da B3" (o atual, `.xlsx`) e "Importar da BTG" (`.pdf`).
2. O usuário escolhe o PDF. O app lê, compara com a carteira e mostra a **mesma tela de revisão da B3**, linha a linha.
3. Identidade do ativo ("já existe no banco?"): ticker para renda variável, código do título para renda fixa (`CDBA26FICT1`), CNPJ para fundo. Se já existe, o item é igual/atualizar/divergente; se não, é novo com a sugestão preenchida.
4. O usuário decide cada linha, completa o que faltar e aplica. Tudo ou nada, numa transação.
5. **Sem estado "fora da BTG"** e sem opção de remover nada que não veio no arquivo.
6. **Cripto entra.** Como não há amostra da seção de posição, o leitor reconhece "CriptoAtivos - Posição" se existir; formato que não entende vira aviso, não importação. Movimentação de cripto (compra/venda no mês) não vira investimento.

7. **Um investimento por ativo e por corretora** (decisão do usuário). PETR4 na BTG e PETR4 na XP são dois investimentos. A identidade do ativo passa a ser *identidade + instituição*. Isso vale também para a **importação da B3**, que hoje soma o mesmo ticker (e o mesmo Tesouro) de várias corretoras num grupo só: ela passa a gerar uma linha por ativo e por instituição (a planilha já traz a coluna `Instituição` em cada linha). Assim uma importação não desfaz a outra.
8. **Valor atual de fundos e cripto** (decisão do usuário). Hoje o app avalia `fundo` e `cripto` sempre pelo valor aplicado (`mercado/valorizacao.rs`, origem "sem fonte"). A importação da BTG passa a gravar o **saldo bruto e a data de referência** do PDF para esses ativos, e a valorização usa esse saldo quando ele existir (com a data visível, para o usuário saber de quando é o valor), caindo no valor aplicado quando não houver. Importações seguintes substituem o saldo pelo mais recente.
9. **Nenhuma importação pergunta mais sobre o que ficou de fora** (decisão do usuário). O estado "fora da B3" sai também da importação da B3: investimentos que não vieram no arquivo simplesmente não aparecem na revisão. Remover continua possível pela tela de investimentos, como qualquer exclusão manual.

## Escolhas que fiz (o usuário pode revisar)

- **Dados já gravados com a soma de várias corretoras** (gravados pela importação da B3 antes desta mudança): nada é dividido nem alterado sozinho. Na próxima importação esses investimentos casam com a linha de uma das corretoras e aparecem como "atualizar"/"divergente", e o usuário decide. Como exatamente o casamento trata esses casos fica com quem constrói, desde que nada mude sem confirmação.
- **Saldo usado é o bruto**, não o líquido, para ficar coerente com o valor atual que o app já mostra para os outros tipos.

- **Um item por ativo**, não por lote: se o fundo ou o CDB tiver várias aplicações no detalhamento, o valor aplicado é a soma dos lotes e a data de aplicação é a do primeiro lote. É o mesmo agrupamento que a B3 já faz para renda variável e Tesouro.
- **Ações/BDRs sem data**: a sugestão usa a data de emissão do extrato (o fim do período) como data de aplicação, editável na revisão. Valor aplicado = quantidade × preço médio, como a B3 já calcula.
- **Fundos não têm ticker**: o CNPJ ocupa o mesmo papel que o código da B3 ocupa hoje (`investimentos.codigo_b3`). Se esse campo deve ser renomeado para algo neutro ("código externo"), fica a critério de quem constrói, desde que a B3 continue funcionando.
- **Casamento entre fontes**: o código de renda fixa do PDF (`CDBA26FICT1`) parece ser o mesmo código de registro que a planilha da B3 traz na coluna `Código`. Isso é uma suposição (não comparei os dois arquivos do mesmo título); se não bater, vale o casamento por emissor + vencimento que a B3 já usa quando não há código.
- **Instituição**: tudo o que vem do PDF tem instituição "BTG Pactual" (escrita igual ao que a B3 grava para a BTG, para não parecer duas corretoras).
- **"Atualizar"**: renda variável segue a regra da B3 — muda só a quantidade e recalcula o aplicado; o preço médio já lançado nunca é sobrescrito; nada é apagado sem pedido explícito. Para fundo e renda fixa, como o PDF traz o valor aplicado, o comparador compara o valor aplicado: igual → "igual"; diferente → "atualizar", e "Usar BTG" grava o valor aplicado do PDF. Campos vazios no app (ex.: taxa de um CDB que veio da B3) são preenchidos com o do PDF; campos já preenchidos nunca são sobrescritos, exceto o valor aplicado/quantidade pela ação "Usar BTG".
- **Fundo ou cripto lançado à mão (sem CNPJ/código)**: segunda regra de casamento, como a renda fixa da B3 já faz — mesmo tipo, sem código gravado, nome normalizado igual (o nome do fundo/cripto fica em `emissor`). Um candidato → "igual"/"atualizar"; vários, ou nome parecido mas não igual → "divergente" com `Vincular`. Confirmado o casamento, o CNPJ (ou código) passa a ser gravado no investimento, como a B3 grava o código hoje.
- **Identidade da cripto**: o nome do ativo como aparece no PDF (ex.: `BITCOIN (XBT)`), normalizado. O formato da seção de posição é presumido; o que não bater vira aviso.
- **Última importação**: o registro em `importacao` passa a guardar de qual fonte veio (B3 ou BTG).
- **Onde ficam os dois botões**: no topo da tela de Investimentos, onde já está o "Importar da B3": "Importar da B3" e "Importar da BTG", lado a lado. Cada um abre a tela de importação da sua fonte ("Importar da B3" / "Importar da BTG").
- **Mesma corretora com nomes diferentes**: a B3 grava o nome cru da planilha (ex.: "BTG PACTUAL CTVM S/A") e o PDF não traz nome de corretora. Para "um investimento por corretora" não virar duplicação, a comparação reconhece a corretora por uma chave: o nome normalizado, ligado ao nome do catálogo (`INSTITUICOES`) quando contém a marca dele (BTG, XP, Inter, Rico, Nu, Banco do Brasil, Itaú, Bradesco, Santander, Caixa, C6).
- Os textos da tela que dizem "B3" ("Na planilha B3", "Usar valor da B3", "Área do Investidor B3 · N linhas lidas", a nota "Fundos e previdência normalmente não aparecem na B3") passam a usar o nome da fonte que está sendo importada.

## Restrições

- Arquitetura atual: a leitura e a comparação ficam no núcleo Rust (`crates/guru-core/src/importacao/`), o main do Electron abre o diálogo e lê os bytes, o addon (`crates/guru-node`) guarda a comparação com um token, o caminho do arquivo nunca vai para a interface.
- O app é distribuído para Windows, macOS e Linux e roda offline. Nada de depender de programa instalado na máquina (ex.: `pdftotext`/poppler).
- Todas as restrições globais das fases anteriores continuam (segurança da janela, CSP, nada de `innerHTML` com dados, pt-BR, centavos em `i64`, taxas em `Decimal`, erros em SCREAMING_SNAKE_CASE, e2e com a janela escondida).
- Limite de tamanho do arquivo como na B3 (20 MB); arquivo que não é PDF, ou PDF que não é um extrato da BTG, dá `ARQUIVO_INVALIDO` com mensagem clara.
- `quantidade` no banco é inteira; fundos e cripto usam a forma "valor" do catálogo (sem quantidade), então cotas fracionárias não são problema.

## Fica a critério de quem constrói

- Como extrair o texto do PDF: crate Rust (ex.: `lopdf`/`pdf-extract`) ou outra via, desde que funcione nas três plataformas sem dependência externa e preserve a ordem das colunas das tabelas. O PDF de exemplo tem colunas com cabeçalho quebrado em várias linhas e, no detalhamento de renda fixa, o código do ativo quebrado em duas linhas (`CDB-` / `CDBA26FICT1`) — o leitor precisa aguentar isso.
- Como gerar os PDFs fictícios de teste.
- Como reorganizar o comparador da B3 para servir às duas fontes (generalizar ou ter um comparador próprio para a BTG reaproveitando as peças).
- Como mapear subseções de renda fixa além de CDB (LCI, LCA, Tesouro custodiado na BTG etc.) para os tipos do catálogo; o que não mapear vira aviso.
