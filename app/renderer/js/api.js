// Desembrulha o envelope do main (`{ ok: true, value }` ou `{ ok: false, code, message }`).
// O Electron e o contextBridge só deixam passar a mensagem de um erro, então a falha
// volta a ser um `Error` com `code` aqui, no mundo da página.
export async function desembrulhar(promessa) {
  const resposta = await promessa;
  if (resposta.ok) return resposta.value;
  const erro = new Error(resposta.message);
  erro.code = resposta.code;
  throw erro;
}

export const api = {
  app: { info: () => desembrulhar(window.guru.app.info()) },
  investimentos: {
    catalogo: () => desembrulhar(window.guru.investimentos.catalogo()),
    listar: () => desembrulhar(window.guru.investimentos.listar()),
    ler: (id) => desembrulhar(window.guru.investimentos.ler(id)),
    criar: (dados) => desembrulhar(window.guru.investimentos.criar(dados)),
    atualizar: (id, dados) => desembrulhar(window.guru.investimentos.atualizar(id, dados)),
    apagar: (id) => desembrulhar(window.guru.investimentos.apagar(id)),
  },
  carteira: { resumo: () => desembrulhar(window.guru.carteira.resumo()) },
};
