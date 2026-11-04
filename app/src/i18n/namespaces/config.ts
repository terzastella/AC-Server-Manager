import { defineNamespace } from "../helpers";

export const config = defineNamespace({
  it: {
    "config.title": "Configurazione",
    "config.loading": "Caricamento server_cfg.ini…",
    "config.searchPlaceholder": "Cerca chiave, valore o sezione…",
    "config.unsaved": "modifiche non salvate",
    "config.backupNow": "Backup ora",
    "config.backupPath": "Backup: {path}",
    "config.reload": "Ricarica",
    "config.save": "Salva config",
    "config.saved": "Config salvata",
    "config.savedWithBackup": "Config salvata · backup creato",
    "config.welcomeBadge": "msg",
    "config.welcomeTitle": "Messaggio di benvenuto",
    "config.welcomeSubtitle": "Mostrato in chat quando un giocatore entra",
    "config.welcomeSave": "Salva messaggio",
    "config.welcomeSaved": "welcome.txt salvato",
    "config.noResults": "Nessun risultato per “{query}”.",
    "config.sectionSubtitle":
      "{n} chiavi · il file originale mantiene commenti e chiavi sconosciute",
    "config.sectionNotFound": "Sezione non trovata.",
    "config.toggleOn": "attivo",
    "config.toggleOff": "disattivo",
    "config.fieldFallback":
      "Impostazione “{key}” della sezione [{section}]: viene letta dal server e riscritta senza modifiche.",

    "config.field.name.help": "Nome mostrato ai client per questa voce.",
    "config.field.password.help": "Password richiesta per entrare. Vuoto = nessuna password.",
    "config.field.adminPassword.help": "Password per usare i comandi admin nella chat.",
    "config.field.udpPort.help":
      "Porta UDP di gioco. Deve essere aperta nel firewall e usata dai client.",
    "config.field.tcpPort.help": "Porta TCP del server, di norma uguale alla UDP.",
    "config.field.httpPort.help":
      "Porta HTTP per Content Manager, la dashboard live e il controllo del server.",
    "config.field.registerToLobby.help":
      "1 = il server si registra nella lobby Kunos (serve Steam valido). 0 = solo IP diretto/LAN.",
    "config.field.clientSendIntervalHz.help":
      "Quanti aggiornamenti al secondo il server invia ai client. Più alto = più traffico.",
    "config.field.sendBufferSize.help": "Dimensione del buffer di invio in byte. 0 = automatico.",
    "config.field.recvBufferSize.help":
      "Dimensione del buffer di ricezione in byte. 0 = automatico.",
    "config.field.kickQuorum.help": "Percentuale di voti necessaria per espellere un giocatore.",
    "config.field.votingQuorum.help":
      "Percentuale di voti necessaria perché una votazione passi.",
    "config.field.voteDuration.help": "Durata di una votazione in secondi.",
    "config.field.blacklistMode.help":
      "0 = blacklist disattivata. 1 = i giocatori in blacklist vengono rifiutati.",
    "config.field.numThreads.help":
      "Numero di thread usati dal server. 2 è un buon valore per pochi giocatori.",
    "config.field.sleepTime.help":
      "Pausa del ciclo principale del server in millisecondi. 1 è il default.",
    "config.field.welcomeMessage.help":
      "File di testo mostrato in chat quando un giocatore entra.",
    "config.field.cars.help":
      "Auto disponibili, separate da ; senza spazi. Ogni nome deve esistere in content\\cars.",
    "config.field.track.help": "Cartella della pista in content\\tracks.",
    "config.field.configTrack.help":
      "Layout della pista, se la pista ne ha più di uno. Vuoto = layout predefinito.",
    "config.field.sunAngle.help": "Angolo del sole: determina l'ora del giorno in pista.",
    "config.field.fuelRate.help": "Consumo carburante in percentuale. 100 = consumo reale.",
    "config.field.damageMultiplier.help":
      "Danno in percentuale. 0 = nessun danno, 100 = danno reale.",
    "config.field.tyreWearRate.help": "Consumo gomme in percentuale. 100 = usura reale.",
    "config.field.allowedTyresOut.help":
      "Quante gomme fuori pista sono consentite prima del richiamo. -1 = nessun limite.",
    "config.field.absAllowed.help": "ABS: 0 = vietato, 1 = consentito, 2 = obbligatorio.",
    "config.field.tcAllowed.help":
      "Controllo di trazione: 0 = vietato, 1 = consentito, 2 = obbligatorio.",
    "config.field.stabilityAllowed.help":
      "Aiuto alla stabilità: 1 = disponibile, 0 = disattivato.",
    "config.field.autoclutchAllowed.help":
      "Frizione automatica: 1 = disponibile, 0 = solo manuale.",
    "config.field.tyreBlanketsAllowed.help": "Coperte termiche: 1 = disponibili, 0 = vietate.",
    "config.field.forceVirtualMirror.help":
      "1 = forza lo specchietto virtuale per tutti i client.",
    "config.field.reversedGridRacePositions.help":
      "1 = griglia di partenza invertita rispetto ai risultati.",
    "config.field.timeOfDayMult.help":
      "Moltiplicatore della velocità con cui scorre il tempo in pista.",
    "config.field.qualifyMaxWaitPerc.help":
      "Attesa massima in qualifica, in percentuale della durata della sessione.",
    "config.field.raceGasPenaltyDisabled.help":
      "1 = disattiva la penalità per chi taglia con il gas.",
    "config.field.maxBallastKg.help":
      "Zavorra massima che i giocatori possono impostare, in kg. 0 = nessuna.",
    "config.field.raceExtraLap.help":
      "1 = aggiunge un giro dopo il tempo/scadenza della gara.",
    "config.field.maxContactsPerKm.help":
      "Contatti consentiti per chilometro prima di una penalità.",
    "config.field.resultScreenTime.help": "Secondi di permanenza della schermata risultati.",
    "config.field.pickupModeEnabled.help":
      "1 = il giocatore entra e sceglie l'auto dall'elenco CARS.",
    "config.field.lockedEntryList.help":
      "1 = possono entrare solo i GUID presenti in entry_list.ini.",
    "config.field.loopMode.help": "1 = dopo la gara si riparte dalla prima sessione.",
    "config.field.maxClients.help":
      "Numero massimo di giocatori connessi. Con pickup mode conviene almeno 3x le auto.",
    "config.field.raceOverTime.help":
      "Secondi di tolleranza oltre il tempo massimo della gara.",
    "config.field.startRule.help": "0 = partenza libera, 1 = serve il semaforo.",
    "config.field.time.help": "Durata della sessione in minuti (sessioni a tempo).",
    "config.field.laps.help": "Numero di giri della gara.",
    "config.field.waitTime.help": "Secondi di attesa prima del via della sessione.",
    "config.field.isOpen.help":
      "1 = i giocatori possono entrare durante la sessione. 0 = sessione chiusa.",
    "config.field.sessionStart.help": "Gripposità iniziale della pista, da 0 a 100.",
    "config.field.randomness.help":
      "Quanto varia casualmente la gripposità durante la sessione.",
    "config.field.sessionTransfer.help":
      "Percentuale di gripposità che passa alla sessione successiva.",
    "config.field.lapGain.help": "Quanto aumenta la gripposità per ogni giro completato.",
    "config.field.graphics.help":
      "Cartella meteo in content\\weather (es. 3_clear, 7_heavy_clouds).",
    "config.field.baseTemperatureAmbient.help": "Temperatura dell'aria in gradi.",
    "config.field.baseTemperatureRoad.help": "Temperatura dell'asfalto in gradi.",
    "config.field.variationAmbient.help":
      "Variazione della temperatura aria durante la sessione.",
    "config.field.variationRoad.help":
      "Variazione della temperatura asfalto durante la sessione.",
    "config.field.windBaseSpeedMin.help": "Velocità minima del vento in km/h.",
    "config.field.windBaseSpeedMax.help": "Velocità massima del vento in km/h.",
    "config.field.windBaseDirection.help": "Direzione base del vento in gradi.",
    "config.field.windVariationDirection.help": "Quanto può cambiare la direzione del vento.",
    "config.field.host.help": "Host FTP per il caricamento dei risultati. Vuoto = disattivato.",
    "config.field.login.help": "Utente FTP.",
    "config.field.folder.help": "Cartella FTP di destinazione.",
    "config.field.linux.help": "1 = il server è su Linux, 0 = Windows.",
    "config.field.description.help": "Descrizione libera del server.",
    "config.field.exserverexe.help": "Eseguibile esterno da lanciare insieme al server.",
    "config.field.exserverbat.help": "Script batch esterno da lanciare insieme al server.",
    "config.field.exserverhidewin.help": "1 = nasconde la finestra del processo esterno.",
    "config.field.weblink.help": "Link web associato al server.",
    "config.field.welcomePath.help": "Percorso del file di benvenuto.",

    "config.section.server.name":
      "Nome del server visibile nella lista e quando i client si connettono.",
    "config.section.server.password":
      "Password richiesta per entrare nel server. Vuoto = nessuna password.",
    "config.section.ftp.host":
      "Host FTP per il caricamento dei risultati. Vuoto = FTP disattivato.",
    "config.section.ftp.login": "Utente per l'accesso FTP.",
    "config.section.ftp.password": "Password per l'accesso FTP.",
    "config.section.ftp.folder": "Cartella FTP in cui caricare i risultati.",
    "config.section.ftp.linux": "1 = il server FTP è su Linux, 0 = su Windows.",
    "config.section.practice.name":
      "Nome della sessione di prove libere mostrato ai client.",
    "config.section.practice.time": "Durata delle prove libere in minuti.",
    "config.section.practice.isOpen": "1 = si può entrare durante le prove libere.",
    "config.section.qualify.name": "Nome della sessione di qualifica.",
    "config.section.qualify.time": "Durata della qualifica in minuti.",
    "config.section.qualify.waitTime": "Secondi di attesa prima del via della qualifica.",
    "config.section.qualify.isOpen": "1 = si può entrare durante la qualifica.",
    "config.section.race.name": "Nome della gara.",
    "config.section.race.laps": "Numero di giri della gara.",
    "config.section.race.waitTime": "Secondi di attesa prima del via della gara.",
    "config.section.race.isOpen": "1 = si può entrare durante la gara.",
  },
  en: {
    "config.title": "Configuration",
    "config.loading": "Loading server_cfg.ini…",
    "config.searchPlaceholder": "Search key, value or section…",
    "config.unsaved": "unsaved changes",
    "config.backupNow": "Backup now",
    "config.backupPath": "Backup: {path}",
    "config.reload": "Reload",
    "config.save": "Save config",
    "config.saved": "Config saved",
    "config.savedWithBackup": "Config saved · backup created",
    "config.welcomeBadge": "msg",
    "config.welcomeTitle": "Welcome message",
    "config.welcomeSubtitle": "Shown in chat when a player joins",
    "config.welcomeSave": "Save message",
    "config.welcomeSaved": "welcome.txt saved",
    "config.noResults": "No results for “{query}”.",
    "config.sectionSubtitle":
      "{n} keys · the original file keeps comments and unknown keys",
    "config.sectionNotFound": "Section not found.",
    "config.toggleOn": "on",
    "config.toggleOff": "off",
    "config.fieldFallback":
      "Setting “{key}” in section [{section}]: it is read by the server and written back unchanged.",

    "config.field.name.help": "Name shown to clients for this entry.",
    "config.field.password.help": "Password required to join. Empty = no password.",
    "config.field.adminPassword.help": "Password to use admin commands in chat.",
    "config.field.udpPort.help":
      "Game UDP port. Must be open in the firewall and used by the clients.",
    "config.field.tcpPort.help": "Server TCP port, usually the same as UDP.",
    "config.field.httpPort.help":
      "HTTP port for Content Manager, the live dashboard and server control.",
    "config.field.registerToLobby.help":
      "1 = the server registers in the Kunos lobby (valid Steam required). 0 = direct IP/LAN only.",
    "config.field.clientSendIntervalHz.help":
      "How many updates per second the server sends to clients. Higher = more traffic.",
    "config.field.sendBufferSize.help": "Send buffer size in bytes. 0 = automatic.",
    "config.field.recvBufferSize.help": "Receive buffer size in bytes. 0 = automatic.",
    "config.field.kickQuorum.help": "Percentage of votes needed to kick a player.",
    "config.field.votingQuorum.help": "Percentage of votes needed for a vote to pass.",
    "config.field.voteDuration.help": "Duration of a vote in seconds.",
    "config.field.blacklistMode.help":
      "0 = blacklist disabled. 1 = blacklisted players are rejected.",
    "config.field.numThreads.help":
      "Number of threads used by the server. 2 is a good value for few players.",
    "config.field.sleepTime.help": "Main server loop pause in milliseconds. 1 is the default.",
    "config.field.welcomeMessage.help": "Text file shown in chat when a player joins.",
    "config.field.cars.help":
      "Available cars, separated by ; without spaces. Each name must exist in content\\cars.",
    "config.field.track.help": "Track folder in content\\tracks.",
    "config.field.configTrack.help":
      "Track layout, if the track has more than one. Empty = default layout.",
    "config.field.sunAngle.help": "Sun angle: sets the time of day on track.",
    "config.field.fuelRate.help": "Fuel consumption in percent. 100 = real consumption.",
    "config.field.damageMultiplier.help":
      "Damage in percent. 0 = no damage, 100 = real damage.",
    "config.field.tyreWearRate.help": "Tyre wear in percent. 100 = real wear.",
    "config.field.allowedTyresOut.help":
      "How many wheels off track are allowed before a warning. -1 = no limit.",
    "config.field.absAllowed.help": "ABS: 0 = not allowed, 1 = allowed, 2 = required.",
    "config.field.tcAllowed.help":
      "Traction control: 0 = not allowed, 1 = allowed, 2 = required.",
    "config.field.stabilityAllowed.help": "Stability aid: 1 = available, 0 = off.",
    "config.field.autoclutchAllowed.help": "Automatic clutch: 1 = available, 0 = manual only.",
    "config.field.tyreBlanketsAllowed.help": "Tyre blankets: 1 = available, 0 = not allowed.",
    "config.field.forceVirtualMirror.help": "1 = force the virtual mirror for all clients.",
    "config.field.reversedGridRacePositions.help":
      "1 = starting grid reversed from the results.",
    "config.field.timeOfDayMult.help": "Multiplier for how fast time passes on track.",
    "config.field.qualifyMaxWaitPerc.help":
      "Maximum wait in qualifying, as a percentage of session duration.",
    "config.field.raceGasPenaltyDisabled.help":
      "1 = disables the penalty for cutting with the throttle.",
    "config.field.maxBallastKg.help":
      "Maximum ballast players can set, in kg. 0 = none.",
    "config.field.raceExtraLap.help": "1 = adds a lap after the race time runs out.",
    "config.field.maxContactsPerKm.help":
      "Allowed contacts per kilometre before a penalty.",
    "config.field.resultScreenTime.help": "Seconds the results screen stays visible.",
    "config.field.pickupModeEnabled.help":
      "1 = the player joins and picks a car from the CARS list.",
    "config.field.lockedEntryList.help": "1 = only GUIDs in entry_list.ini can join.",
    "config.field.loopMode.help": "1 = after the race it restarts from the first session.",
    "config.field.maxClients.help":
      "Maximum number of connected players. With pickup mode at least 3x the cars is advisable.",
    "config.field.raceOverTime.help": "Seconds of tolerance past the race time limit.",
    "config.field.startRule.help": "0 = free start, 1 = traffic light required.",
    "config.field.time.help": "Session duration in minutes (timed sessions).",
    "config.field.laps.help": "Number of race laps.",
    "config.field.waitTime.help": "Seconds to wait before the session starts.",
    "config.field.isOpen.help":
      "1 = players can join during the session. 0 = session closed.",
    "config.field.sessionStart.help": "Initial track grip, from 0 to 100.",
    "config.field.randomness.help": "How much grip varies randomly during the session.",
    "config.field.sessionTransfer.help":
      "Percentage of grip carried over to the next session.",
    "config.field.lapGain.help": "How much grip increases for each completed lap.",
    "config.field.graphics.help":
      "Weather folder in content\\weather (e.g. 3_clear, 7_heavy_clouds).",
    "config.field.baseTemperatureAmbient.help": "Air temperature in degrees.",
    "config.field.baseTemperatureRoad.help": "Asphalt temperature in degrees.",
    "config.field.variationAmbient.help":
      "Air temperature variation during the session.",
    "config.field.variationRoad.help":
      "Asphalt temperature variation during the session.",
    "config.field.windBaseSpeedMin.help": "Minimum wind speed in km/h.",
    "config.field.windBaseSpeedMax.help": "Maximum wind speed in km/h.",
    "config.field.windBaseDirection.help": "Base wind direction in degrees.",
    "config.field.windVariationDirection.help": "How much the wind direction can change.",
    "config.field.host.help": "FTP host for uploading results. Empty = disabled.",
    "config.field.login.help": "FTP user.",
    "config.field.folder.help": "Destination FTP folder.",
    "config.field.linux.help": "1 = the server runs on Linux, 0 = Windows.",
    "config.field.description.help": "Free-form server description.",
    "config.field.exserverexe.help": "External executable to launch with the server.",
    "config.field.exserverbat.help": "External batch script to launch with the server.",
    "config.field.exserverhidewin.help": "1 = hides the window of the external process.",
    "config.field.weblink.help": "Web link associated with the server.",
    "config.field.welcomePath.help": "Path of the welcome file.",

    "config.section.server.name":
      "Server name shown in the list and when clients connect.",
    "config.section.server.password":
      "Password required to join the server. Empty = no password.",
    "config.section.ftp.host": "FTP host for uploading results. Empty = FTP disabled.",
    "config.section.ftp.login": "User for FTP access.",
    "config.section.ftp.password": "Password for FTP access.",
    "config.section.ftp.folder": "FTP folder where results are uploaded.",
    "config.section.ftp.linux": "1 = the FTP server runs on Linux, 0 = on Windows.",
    "config.section.practice.name": "Name of the practice session shown to clients.",
    "config.section.practice.time": "Practice duration in minutes.",
    "config.section.practice.isOpen": "1 = players can join during practice.",
    "config.section.qualify.name": "Name of the qualifying session.",
    "config.section.qualify.time": "Qualifying duration in minutes.",
    "config.section.qualify.waitTime": "Seconds to wait before qualifying starts.",
    "config.section.qualify.isOpen": "1 = players can join during qualifying.",
    "config.section.race.name": "Name of the race.",
    "config.section.race.laps": "Number of race laps.",
    "config.section.race.waitTime": "Seconds to wait before the race starts.",
    "config.section.race.isOpen": "1 = players can join during the race.",
  },
});
