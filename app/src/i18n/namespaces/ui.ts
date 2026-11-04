import { defineNamespace } from "../helpers";

export const ui = defineNamespace({
  it: {
    "ui.fieldHelpFallback": "Impostazione “{label}” letta dal file di configurazione.",
    "ui.fieldHelpAria": "Spiegazione di {label}",
    "ui.terminalEmpty": "Nessun output. Avvia il server per vedere i log.",
  },
  en: {
    "ui.fieldHelpFallback": "Setting “{label}” read from the configuration file.",
    "ui.fieldHelpAria": "Explanation of {label}",
    "ui.terminalEmpty": "No output yet. Start the server to see the log.",
  },
});
