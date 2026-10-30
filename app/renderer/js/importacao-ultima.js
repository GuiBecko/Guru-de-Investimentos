// "Última importação B3: dd/mm/aaaa" no rodapé do menu; só aparece quando já houve uma importação.
import { api } from "./api.js";
import { formatarData } from "./formato.js";

export async function atualizarUltimaImportacao() {
  const raiz = document.querySelector('[data-testid="importacao-ultima"]');
  if (!raiz) return;
  try {
    const data = await api.importacao.ultima();
    raiz.textContent = data ? `Última importação B3: ${formatarData(data)}` : "";
    raiz.hidden = !data;
  } catch (e) {
    console.error("Falha ao ler a última importação:", e);
  }
}
