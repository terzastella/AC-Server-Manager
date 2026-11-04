import { useEffect, useMemo, useState } from "react";
import { api } from "../api";
import { useApp } from "../store";
import { Badge, Button, Card, Empty, Field, Toggle } from "../components/ui";
import { CarsField, ContentPicker, TrackField } from "../components/ContentSelect";
import { useT } from "../i18n/core";
import type { ContentLibrary, IniDoc, IniEntry } from "../types";

type TFunc = ReturnType<typeof useT>["t"];

const BOOL_KEYS = new Set([
  "PICKUP_MODE_ENABLED",
  "LOCKED_ENTRY_LIST",
  "REGISTER_TO_LOBBY",
  "LOOP_MODE",
  "IS_OPEN",
  "FORCE_VIRTUAL_MIRROR",
  "REVERSED_GRID_RACE_POSITIONS",
  "RACE_GAS_PENALTY_DISABLED",
  "RACE_EXTRA_LAP",
  "SPECTATOR_MODE",
  "AUTOCLUTCH_ALLOWED",
  "TYRE_BLANKETS_ALLOWED",
  "STABILITY_ALLOWED",
  "ABS_ALLOWED",
  "TC_ALLOWED",
  "LINUX",
  "EXSERVERHIDEWIN",
]);

const DESCRIPTION_KEYS = {
  NAME: "config.field.name.help",
  PASSWORD: "config.field.password.help",
  ADMIN_PASSWORD: "config.field.adminPassword.help",
  UDP_PORT: "config.field.udpPort.help",
  TCP_PORT: "config.field.tcpPort.help",
  HTTP_PORT: "config.field.httpPort.help",
  REGISTER_TO_LOBBY: "config.field.registerToLobby.help",
  CLIENT_SEND_INTERVAL_HZ: "config.field.clientSendIntervalHz.help",
  SEND_BUFFER_SIZE: "config.field.sendBufferSize.help",
  RECV_BUFFER_SIZE: "config.field.recvBufferSize.help",
  KICK_QUORUM: "config.field.kickQuorum.help",
  VOTING_QUORUM: "config.field.votingQuorum.help",
  VOTE_DURATION: "config.field.voteDuration.help",
  BLACKLIST_MODE: "config.field.blacklistMode.help",
  NUM_THREADS: "config.field.numThreads.help",
  SLEEP_TIME: "config.field.sleepTime.help",
  WELCOME_MESSAGE: "config.field.welcomeMessage.help",
  CARS: "config.field.cars.help",
  TRACK: "config.field.track.help",
  CONFIG_TRACK: "config.field.configTrack.help",
  SUN_ANGLE: "config.field.sunAngle.help",
  FUEL_RATE: "config.field.fuelRate.help",
  DAMAGE_MULTIPLIER: "config.field.damageMultiplier.help",
  TYRE_WEAR_RATE: "config.field.tyreWearRate.help",
  ALLOWED_TYRES_OUT: "config.field.allowedTyresOut.help",
  ABS_ALLOWED: "config.field.absAllowed.help",
  TC_ALLOWED: "config.field.tcAllowed.help",
  STABILITY_ALLOWED: "config.field.stabilityAllowed.help",
  AUTOCLUTCH_ALLOWED: "config.field.autoclutchAllowed.help",
  TYRE_BLANKETS_ALLOWED: "config.field.tyreBlanketsAllowed.help",
  FORCE_VIRTUAL_MIRROR: "config.field.forceVirtualMirror.help",
  REVERSED_GRID_RACE_POSITIONS: "config.field.reversedGridRacePositions.help",
  TIME_OF_DAY_MULT: "config.field.timeOfDayMult.help",
  QUALIFY_MAX_WAIT_PERC: "config.field.qualifyMaxWaitPerc.help",
  RACE_GAS_PENALTY_DISABLED: "config.field.raceGasPenaltyDisabled.help",
  MAX_BALLAST_KG: "config.field.maxBallastKg.help",
  RACE_EXTRA_LAP: "config.field.raceExtraLap.help",
  MAX_CONTACTS_PER_KM: "config.field.maxContactsPerKm.help",
  RESULT_SCREEN_TIME: "config.field.resultScreenTime.help",
  PICKUP_MODE_ENABLED: "config.field.pickupModeEnabled.help",
  LOCKED_ENTRY_LIST: "config.field.lockedEntryList.help",
  LOOP_MODE: "config.field.loopMode.help",
  MAX_CLIENTS: "config.field.maxClients.help",
  RACE_OVER_TIME: "config.field.raceOverTime.help",
  START_RULE: "config.field.startRule.help",
  TIME: "config.field.time.help",
  LAPS: "config.field.laps.help",
  WAIT_TIME: "config.field.waitTime.help",
  IS_OPEN: "config.field.isOpen.help",
  SESSION_START: "config.field.sessionStart.help",
  RANDOMNESS: "config.field.randomness.help",
  SESSION_TRANSFER: "config.field.sessionTransfer.help",
  LAP_GAIN: "config.field.lapGain.help",
  GRAPHICS: "config.field.graphics.help",
  BASE_TEMPERATURE_AMBIENT: "config.field.baseTemperatureAmbient.help",
  BASE_TEMPERATURE_ROAD: "config.field.baseTemperatureRoad.help",
  VARIATION_AMBIENT: "config.field.variationAmbient.help",
  VARIATION_ROAD: "config.field.variationRoad.help",
  WIND_BASE_SPEED_MIN: "config.field.windBaseSpeedMin.help",
  WIND_BASE_SPEED_MAX: "config.field.windBaseSpeedMax.help",
  WIND_BASE_DIRECTION: "config.field.windBaseDirection.help",
  WIND_VARIATION_DIRECTION: "config.field.windVariationDirection.help",
  HOST: "config.field.host.help",
  LOGIN: "config.field.login.help",
  FOLDER: "config.field.folder.help",
  LINUX: "config.field.linux.help",
  DESCRIPTION: "config.field.description.help",
  EXSERVEREXE: "config.field.exserverexe.help",
  EXSERVERBAT: "config.field.exserverbat.help",
  EXSERVERHIDEWIN: "config.field.exserverhidewin.help",
  WEBLINK: "config.field.weblink.help",
  WELCOME_PATH: "config.field.welcomePath.help",
} as const;

const SECTION_DESCRIPTION_KEYS = {
  "SERVER.NAME": "config.section.server.name",
  "SERVER.PASSWORD": "config.section.server.password",
  "FTP.HOST": "config.section.ftp.host",
  "FTP.LOGIN": "config.section.ftp.login",
  "FTP.PASSWORD": "config.section.ftp.password",
  "FTP.FOLDER": "config.section.ftp.folder",
  "FTP.LINUX": "config.section.ftp.linux",
  "PRACTICE.NAME": "config.section.practice.name",
  "PRACTICE.TIME": "config.section.practice.time",
  "PRACTICE.IS_OPEN": "config.section.practice.isOpen",
  "QUALIFY.NAME": "config.section.qualify.name",
  "QUALIFY.TIME": "config.section.qualify.time",
  "QUALIFY.WAIT_TIME": "config.section.qualify.waitTime",
  "QUALIFY.IS_OPEN": "config.section.qualify.isOpen",
  "RACE.NAME": "config.section.race.name",
  "RACE.LAPS": "config.section.race.laps",
  "RACE.WAIT_TIME": "config.section.race.waitTime",
  "RACE.IS_OPEN": "config.section.race.isOpen",
} as const;

function describe(t: TFunc, section: string, key: string): string {
  const upper = key.toUpperCase();
  const sectionKey = `${section.toUpperCase()}.${upper}` as keyof typeof SECTION_DESCRIPTION_KEYS;
  const sectionDescription = SECTION_DESCRIPTION_KEYS[sectionKey];
  if (sectionDescription) return t(sectionDescription);
  const description = DESCRIPTION_KEYS[upper as keyof typeof DESCRIPTION_KEYS];
  if (description) return t(description);
  return t("config.fieldFallback", { key, section });
}

export function Config() {
  const { t } = useT();
  const { config, refreshConfig, refreshContent, run, notify } = useApp();
  const [draft, setDraft] = useState<IniDoc | null>(null);
  const [active, setActive] = useState("SERVER");
  const [search, setSearch] = useState("");
  const [welcome, setWelcome] = useState("");
  const [welcomeDraft, setWelcomeDraft] = useState("");
  const [library, setLibrary] = useState<ContentLibrary | null>(null);
  const [picker, setPicker] = useState<{ kind: "cars" | "tracks"; section: string } | null>(null);

  useEffect(() => {
    void api.contentLibrary().then(setLibrary).catch(() => setLibrary(null));
  }, []);

  useEffect(() => {
    if (config) {
      setDraft(JSON.parse(JSON.stringify(config)) as IniDoc);
      const names = config.sections.map((section) => section.name);
      setActive((current) => (names.includes(current) ? current : names[0] ?? "SERVER"));
    }
  }, [config]);

  useEffect(() => {
    void api
      .readWelcome()
      .then((text) => {
        setWelcome(text);
        setWelcomeDraft(text);
      })
      .catch(() => {
        setWelcome("");
        setWelcomeDraft("");
      });
  }, []);

  const dirty = useMemo(
    () => JSON.stringify(draft) !== JSON.stringify(config),
    [draft, config],
  );

  if (!draft) {
    return (
      <div className="page">
        <Card title={t("config.title")}><Empty>{t("config.loading")}</Empty></Card>
      </div>
    );
  }

  const update = (sectionName: string, key: string, value: string) => {
    setDraft((current) => {
      if (!current) return current;
      return {
        ...current,
        sections: current.sections.map((section) =>
          section.name !== sectionName
            ? section
            : {
                ...section,
                entries: section.entries.map((entry) =>
                  entry.key === key ? { ...entry, value } : entry,
                ),
              },
        ),
      };
    });
  };

  const save = () =>
    void run("save-config", async () => {
      const backup = await api.saveConfig(draft.sections);
      await refreshConfig();
      await refreshContent();
      notify(backup ? t("config.savedWithBackup") : t("config.saved"), "success");
    });

  const saveWelcome = () =>
    void run("save-welcome", async () => {
      await api.saveWelcome(welcomeDraft);
      setWelcome(welcomeDraft);
      notify(t("config.welcomeSaved"), "success");
    });

  const query = search.trim().toLowerCase();
  const visibleSections = query
    ? draft.sections
        .map((section) => ({
          ...section,
          entries: section.entries.filter(
            (entry) =>
              entry.key.toLowerCase().includes(query) ||
              entry.value.toLowerCase().includes(query) ||
              section.name.toLowerCase().includes(query),
          ),
        }))
        .filter((section) => section.entries.length > 0)
    : draft.sections;

  const current = draft.sections.find((section) => section.name === active);

  const renderEntry = (sectionName: string, entry: IniEntry) => {
    const upper = entry.key.toUpperCase();
    const hint = describe(t, sectionName, entry.key);
    if (upper === "CARS") {
      return (
        <Field key={entry.key} label={entry.key} hint={hint}>
          <CarsField
            value={entry.value}
            onChange={(value) => update(sectionName, entry.key, value)}
            library={library}
            onOpen={() => setPicker({ kind: "cars", section: sectionName })}
          />
        </Field>
      );
    }
    if (upper === "TRACK") {
      return (
        <Field key={entry.key} label={entry.key} hint={hint}>
          <TrackField
            value={entry.value}
            onChange={(value) => update(sectionName, entry.key, value)}
            library={library}
            onOpen={() => setPicker({ kind: "tracks", section: sectionName })}
          />
        </Field>
      );
    }
    return (
      <EntryEditor
        key={entry.key}
        entry={entry}
        section={sectionName}
        onChange={(value) => update(sectionName, entry.key, value)}
      />
    );
  };

  const pickerValue = picker
    ? draft.sections
        .find((section) => section.name === picker.section)
        ?.entries.find(
          (entry) => entry.key.toUpperCase() === (picker.kind === "cars" ? "CARS" : "TRACK"),
        )?.value ?? ""
    : "";


  return (
    <div className="page page-config">
      <div className="config-bar">
        <input
          className="search"
          placeholder={t("config.searchPlaceholder")}
          value={search}
          onChange={(event) => setSearch(event.target.value)}
        />
        <div className="row gap">
          {dirty && <Badge tone="amber">{t("config.unsaved")}</Badge>}
          <Button
            onClick={() =>
              void run("backup", async () => {
                const path = await api.backupNow();
                notify(t("config.backupPath", { path }), "success");
              })
            }
          >
            {t("config.backupNow")}
          </Button>
          <Button variant="ghost" onClick={() => void refreshConfig()}>
            {t("config.reload")}
          </Button>
          <Button variant="primary" disabled={!dirty} onClick={save}>
            {t("config.save")}
          </Button>
        </div>
      </div>

      <div className="config-layout">
        <aside className="section-nav">
          {draft.sections.map((section) => (
            <button
              key={section.name}
              className={active === section.name ? "active" : ""}
              onClick={() => setActive(section.name)}
            >
              <span>{section.name}</span>
              <em>{section.entries.length}</em>
            </button>
          ))}
          <button className={active === "__welcome" ? "active" : ""} onClick={() => setActive("__welcome")}>
            <span>welcome.txt</span>
            <em>{t("config.welcomeBadge")}</em>
          </button>
        </aside>

        <div className="config-body">
          {active === "__welcome" ? (
            <Card
              title={t("config.welcomeTitle")}
              subtitle={t("config.welcomeSubtitle")}
              actions={
                <Button variant="primary" disabled={welcomeDraft === welcome} onClick={saveWelcome}>
                  {t("config.welcomeSave")}
                </Button>
              }
            >
              <textarea
                className="welcome-area"
                rows={16}
                value={welcomeDraft}
                onChange={(event) => setWelcomeDraft(event.target.value)}
              />
            </Card>
          ) : query ? (
            <div className="stack">
              {visibleSections.map((section) => (
                <Card key={section.name} title={section.name}>
                  <div className="form-grid">
                    {section.entries.map((entry) => (
                      <div key={entry.key}>{renderEntry(section.name, entry)}</div>
                    ))}
                  </div>
                </Card>
              ))}
              {visibleSections.length === 0 && (
                <Empty>{t("config.noResults", { query: search })}</Empty>
              )}
            </div>
          ) : current ? (
            <Card
              title={`[${current.name}]`}
              subtitle={t("config.sectionSubtitle", { n: current.entries.length })}
            >
              <div className="form-grid">
                {current.entries.map((entry) => (
                  <div key={entry.key}>{renderEntry(current.name, entry)}</div>
                ))}
              </div>
            </Card>
          ) : (
            <Empty>{t("config.sectionNotFound")}</Empty>
          )}
        </div>
      </div>

      {picker && library && (
        <ContentPicker
          kind={picker.kind}
          library={library}
          initial={pickerValue
            .split(";")
            .map((item) => item.trim())
            .filter(Boolean)}
          onClose={() => setPicker(null)}
          onApply={(ids) => {
            const value = picker.kind === "cars" ? ids.join(";") : ids[0] ?? "";
            update(picker.section, picker.kind === "cars" ? "CARS" : "TRACK", value);
            setPicker(null);
          }}
        />
      )}
    </div>
  );
}

function EntryEditor({
  entry,
  section,
  onChange,
}: {
  entry: IniEntry;
  section: string;
  onChange: (value: string) => void;
}) {
  const { t } = useT();
  const upper = entry.key.toUpperCase();
  const hint = describe(t, section, entry.key);
  const isBool = BOOL_KEYS.has(upper) && (entry.value === "0" || entry.value === "1");
  const isNumber = /^-?\d+$/.test(entry.value) && !isBool;

  if (isBool) {
    return (
      <Field label={entry.key} hint={hint}>
        <Toggle
          checked={entry.value === "1"}
          onChange={(checked) => onChange(checked ? "1" : "0")}
          label={entry.value === "1" ? t("config.toggleOn") : t("config.toggleOff")}
        />
      </Field>
    );
  }

  return (
    <Field label={entry.key} hint={hint}>
      <input
        type={isNumber ? "number" : "text"}
        value={entry.value}
        onChange={(event) => onChange(event.target.value)}
        spellCheck={false}
      />
    </Field>
  );
}
