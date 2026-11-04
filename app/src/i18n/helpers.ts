/**
 * Dizionari per namespace: ogni file esporta `it` ed `en` con le stesse chiavi.
 * Le chiavi sono piatte e prefissate dal namespace (es. "dashboard.title"),
 * così i file restano indipendenti e si possono aggiungere senza conflitti.
 */
export type Namespace<IT extends Record<string, string>> = {
  it: IT;
  en: Record<keyof IT, string>;
};

export function defineNamespace<IT extends Record<string, string>>(
  ns: Namespace<IT>,
): Namespace<IT> {
  return ns;
}
