// Rótulos do período do painel e da legenda do detalhe por classe (puros, testados em Node).
import { mesAno } from "../../graficos.js";

// Com `desde` (primeiro aporte depois do começo do período), o período comparado começa nele.
export function rotulosPeriodo(desde, meses) {
  if (desde) {
    const quando = mesAno(desde);
    return { periodo: `Desde ${quando}`, rendeu: `Rendeu desde ${quando}` };
  }
  return { periodo: `Em ${meses} meses`, rendeu: `Rendeu em ${meses} meses` };
}

const SUA = {
  rf: "Sua renda fixa",
  acoes: "Suas ações",
  fii: "Seus FIIs",
  ext: "Seus investimentos no exterior",
  outros: "Seus outros investimentos",
};

export const rotuloDaClasse = (classe) => SUA[classe] ?? "Sua classe";
