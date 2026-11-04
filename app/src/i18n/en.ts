import { app } from "./namespaces/app";
import { assists } from "./namespaces/assists";
import { cm } from "./namespaces/cm";
import { config } from "./namespaces/config";
import { content } from "./namespaces/content";
import { contentSelect } from "./namespaces/contentSelect";
import { dashboard } from "./namespaces/dashboard";
import { invite } from "./namespaces/invite";
import { join } from "./namespaces/join";
import { launcher } from "./namespaces/launcher";
import { logs } from "./namespaces/logs";
import { network } from "./namespaces/network";
import { presets } from "./namespaces/presets";
import { servers } from "./namespaces/servers";
import { settings } from "./namespaces/settings";
import { slots } from "./namespaces/slots";
import { ui } from "./namespaces/ui";
import { it as itDict } from "./it";

// `Record<keyof typeof itDict, string>`: se manca una chiave italiana in inglese
// TypeScript segnala l'errore in compilazione.
export const en: Record<keyof typeof itDict, string> = {
  ...app.en,
  ...assists.en,
  ...cm.en,
  ...config.en,
  ...content.en,
  ...contentSelect.en,
  ...dashboard.en,
  ...invite.en,
  ...join.en,
  ...launcher.en,
  ...logs.en,
  ...network.en,
  ...presets.en,
  ...servers.en,
  ...settings.en,
  ...slots.en,
  ...ui.en,
};
