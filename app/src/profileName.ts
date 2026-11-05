import type { ServerProfile } from "./types";
import type { TKey } from "./i18n/core";

/** Nome con cui il backend crea il profilo storico. */
const BUILT_IN_NAME = "Predefinito";

/**
 * Nome da mostrare: il profilo predefinito creato dall'app viene tradotto
 * nella lingua corrente, un profilo rinominato dall'utente resta com'e'.
 */
export function profileName(
  profile: Pick<ServerProfile, "name" | "legacy">,
  t: (key: TKey) => string,
): string {
  return profile.legacy && profile.name === BUILT_IN_NAME
    ? t("servers.defaultProfileName")
    : profile.name;
}
