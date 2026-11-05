import { defineNamespace } from "../helpers";

export const servers = defineNamespace({
  it: {
    "servers.title": "I tuoi server",
    "servers.subtitle":
      "Ogni server ha config, slot e risultati propri; auto e piste sono condivise",
    "servers.running":
      "Server in esecuzione: fermalo per creare, duplicare o eliminare un profilo.",
    "servers.active": "attivo",
    "servers.default": "predefinito",
    "servers.defaultProfileName": "Predefinito",
    "servers.activate": "Attiva",
    "servers.rename": "Rinomina",
    "servers.duplicate": "Duplica",
    "servers.confirmDelete": "Eliminare “{name}”? La cartella finisce in backups.",
    "servers.confirm": "Confermi?",

    "servers.notifyCreated": "Server creato e selezionato",
    "servers.notifyDuplicated": "Server duplicato",
    "servers.notifyRenamed": "Nome aggiornato",
    "servers.notifyDeleted": "Server spostato in backups",
    "servers.notifyActivated": "Server attivo: {name}",
    "servers.notifyImported": "Server importato e selezionato",

    "servers.importPickTitle": "Cartella del server da importare",
    "servers.importTitle": "Importa server",
    "servers.importSubtitle": "Da una cartella con cfg/server_cfg.ini",
    "servers.pathPlaceholder": "Percorso della cartella server",
    "servers.browse": "Sfoglia…",
    "servers.namePlaceholder": "Nome da dare al server (vuoto = nome della config)",
    "servers.import": "Importa",
    "servers.importHelp":
      "Vengono copiati `server_cfg.ini`, `entry_list.ini` e `welcome.txt`. Auto e piste restano condivise con gli altri server.",

    "servers.newTitle": "Nuovo server",
    "servers.newSubtitle": "Copia le impostazioni del server attivo",
    "servers.newPlaceholder": "Nome del server (es. Serata GT3)",
    "servers.create": "Crea",
    "servers.newHelp":
      "Auto e piste restano condivise: il nuovo server parte con le stesse auto del server attivo e la sua config è indipendente.",
  },
  en: {
    "servers.title": "Your servers",
    "servers.subtitle":
      "Each server has its own config, slots and results; cars and tracks are shared",
    "servers.running":
      "Server is running: stop it to create, duplicate or delete a profile.",
    "servers.active": "active",
    "servers.default": "default",
    "servers.defaultProfileName": "Default",
    "servers.activate": "Activate",
    "servers.rename": "Rename",
    "servers.duplicate": "Duplicate",
    "servers.confirmDelete": "Delete “{name}”? The folder goes to backups.",
    "servers.confirm": "Confirm?",

    "servers.notifyCreated": "Server created and selected",
    "servers.notifyDuplicated": "Server duplicated",
    "servers.notifyRenamed": "Name updated",
    "servers.notifyDeleted": "Server moved to backups",
    "servers.notifyActivated": "Active server: {name}",
    "servers.notifyImported": "Server imported and selected",

    "servers.importPickTitle": "Server folder to import",
    "servers.importTitle": "Import server",
    "servers.importSubtitle": "From a folder with cfg/server_cfg.ini",
    "servers.pathPlaceholder": "Path of the server folder",
    "servers.browse": "Browse…",
    "servers.namePlaceholder": "Name for the server (empty = config name)",
    "servers.import": "Import",
    "servers.importHelp":
      "`server_cfg.ini`, `entry_list.ini` and `welcome.txt` are copied. Cars and tracks stay shared with the other servers.",

    "servers.newTitle": "New server",
    "servers.newSubtitle": "Copies the settings of the active server",
    "servers.newPlaceholder": "Server name (e.g. GT3 night)",
    "servers.create": "Create",
    "servers.newHelp":
      "Cars and tracks stay shared: the new server starts with the same cars as the active server and its config is independent.",
  },
});
