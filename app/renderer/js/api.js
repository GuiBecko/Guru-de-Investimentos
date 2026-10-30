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
  carteira: {
    resumo: () => desembrulhar(window.guru.carteira.resumo()),
    painel: (meses, classe) => desembrulhar(window.guru.carteira.painel(meses, classe)),
  },
  perfil: {
    questionario: () => desembrulhar(window.guru.perfil.questionario()),
    registrar: (respostas) => desembrulhar(window.guru.perfil.registrar(respostas)),
    atual: () => desembrulhar(window.guru.perfil.atual()),
  },
  alocacao: {
    ler: () => desembrulhar(window.guru.alocacao.ler()),
    salvar: (valores) => desembrulhar(window.guru.alocacao.salvar(valores)),
  },
  objetivo: {
    ler: () => desembrulhar(window.guru.objetivo.ler()),
    salvar: (texto) => desembrulhar(window.guru.objetivo.salvar(texto)),
  },
  metas: {
    listar: () => desembrulhar(window.guru.metas.listar()),
    ler: (id) => desembrulhar(window.guru.metas.ler(id)),
    criar: (dados) => desembrulhar(window.guru.metas.criar(dados)),
    atualizar: (id, dados) => desembrulhar(window.guru.metas.atualizar(id, dados)),
    apagar: (id) => desembrulhar(window.guru.metas.apagar(id)),
  },
  mercado: {
    atualizar: () => desembrulhar(window.guru.mercado.atualizar()),
    status: () => desembrulhar(window.guru.mercado.status()),
  },
};
