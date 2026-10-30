// Saudação do painel: data por extenso e uma frase sorteada entre modelos aplicáveis.
// Nunca recomenda compra ou venda.
import { formatarReais, formatarData, formatarRentabilidade } from "./formato.js";

const FORMATO_DATA = new Intl.DateTimeFormat("pt-BR", { weekday: "long", day: "numeric", month: "long", year: "numeric" });

function cumprimento(hora) {
  if (hora >= 5 && hora < 12) return "Bom dia";
  if (hora >= 12 && hora < 18) return "Boa tarde";
  return "Boa noite";
}

const DIA_MS = 86400000;
const diaUtc = (iso) => Date.UTC(...iso.split("-").map((n, i) => (i === 1 ? Number(n) - 1 : Number(n))));

export function modelosAplicaveis(painel, hoje) {
  const modelos = [
    "Bom ver você por aqui.",
    "Sua carteira está atualizada com os dados de mercado mais recentes.",
    "Um passo de cada vez: o que importa é a constância dos aportes.",
    "Tudo o que está aqui fica só no seu computador.",
  ];
  const { mesCentesimos: mes, mesCdiCentesimos: cdi, rendeuCentavos } = painel.numeros;
  if (mes !== null && cdi !== null && mes !== undefined && cdi !== undefined) {
    if (mes > cdi) modelos.push("Seu mês está acima do CDI até agora.");
    if (mes < cdi) modelos.push("O mês está abaixo do CDI até agora. Meses assim fazem parte.");
  }
  const melhor = painel.variacoes[0];
  if (melhor && melhor.variacaoDecimos > 0) {
    modelos.push(`${melhor.ticker} é o destaque do mês, com ${formatarRentabilidade(melhor.variacaoDecimos)}.`);
  }
  const hojeUtc = Date.UTC(hoje.getFullYear(), hoje.getMonth(), hoje.getDate());
  const proximo = painel.vencimentos.find((v) => {
    const dias = (diaUtc(v.vencimento) - hojeUtc) / DIA_MS;
    return dias >= 0 && dias <= 30;
  });
  if (proximo) modelos.push(`${proximo.nome} vence em ${formatarData(proximo.vencimento).slice(0, 5)}.`);
  if (rendeuCentavos > 0) modelos.push(`Seus investimentos renderam ${formatarReais(rendeuCentavos)} no período.`);
  return modelos;
}

export function saudacao({ agora, painel, sorteio }) {
  const modelos = modelosAplicaveis(painel, agora);
  const texto = modelos[Math.min(modelos.length - 1, Math.floor(sorteio() * modelos.length))];
  return {
    data: FORMATO_DATA.format(agora).replace("-feira", ""),
    frase: `${cumprimento(agora.getHours())}. ${texto}`,
  };
}
