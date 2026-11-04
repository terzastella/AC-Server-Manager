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

export const it = {
  ...app.it,
  ...assists.it,
  ...cm.it,
  ...config.it,
  ...content.it,
  ...contentSelect.it,
  ...dashboard.it,
  ...invite.it,
  ...join.it,
  ...launcher.it,
  ...logs.it,
  ...network.it,
  ...presets.it,
  ...servers.it,
  ...settings.it,
  ...slots.it,
  ...ui.it,
};
