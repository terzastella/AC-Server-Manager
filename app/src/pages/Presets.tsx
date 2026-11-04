import { useEffect, useState } from "react";
import { api, openPath } from "../api";
import { useApp } from "../store";
import { Badge, Button, Card, Empty } from "../components/ui";
import { useT } from "../i18n/core";
import type { BackupInfo, PresetInfo } from "../types";
import { fmtDate } from "../utils";

export function Presets() {
  const { t } = useT();
  const { info, run, notify, refreshAll } = useApp();
  const [presets, setPresets] = useState<PresetInfo[]>([]);
  const [backups, setBackups] = useState<BackupInfo[]>([]);
  const [name, setName] = useState("");

  const load = async () => {
    try {
      const [nextPresets, nextBackups] = await Promise.all([api.listPresets(), api.listBackups()]);
      setPresets(nextPresets);
      setBackups(nextBackups);
    } catch (error) {
      notify(String(error), "error");
    }
  };

  useEffect(() => {
    void load();
  }, []);

  return (
    <div className="page">
      <div className="grid grid-2">
        <Card
          title={t("presets.title")}
          subtitle={t("presets.subtitle")}
          actions={<Button variant="ghost" onClick={() => void load()}>{t("presets.refresh")}</Button>}
        >
          <div className="row gap">
            <input
              className="grow"
              placeholder={t("presets.namePlaceholder")}
              value={name}
              onChange={(event) => setName(event.target.value)}
            />
            <Button
              variant="primary"
              disabled={!name.trim()}
              onClick={() =>
                void run("save-preset", async () => {
                  await api.savePreset(name.trim());
                  setName("");
                  await load();
                  notify(t("presets.saved"), "success");
                })
              }
            >
              {t("presets.saveCurrent")}
            </Button>
          </div>

          <div className="stack-sm top-gap">
            {presets.length === 0 && <Empty>{t("presets.empty")}</Empty>}
            {presets.map((preset) => (
              <div className="preset" key={preset.name}>
                <div className="preset-main">
                  <strong>{preset.name}</strong>
                  <span>
                    {t("presets.meta", {
                      server: preset.server_name || t("presets.noName"),
                      track: preset.track || t("presets.noTrack"),
                      cars: preset.cars.length,
                    })}
                  </span>
                </div>
                <Button
                  onClick={() =>
                    void run("load-preset", async () => {
                      await api.loadPreset(preset.name);
                      await refreshAll();
                      notify(t("presets.loaded", { name: preset.name }), "success");
                    })
                  }
                >
                  {t("presets.load")}
                </Button>
              </div>
            ))}
          </div>
        </Card>

        <Card
          title={t("presets.backupTitle")}
          subtitle={t("presets.backupSubtitle")}
          actions={
            <div className="row gap">
              <Button
                onClick={() =>
                  void run("backup", async () => {
                    const path = await api.backupNow();
                    await load();
                    notify(t("presets.notifyBackup", { path }), "success");
                  })
                }
              >
                {t("presets.backupNow")}
              </Button>
              <Button variant="ghost" onClick={() => void openPath((info?.server_dir ?? "") + "\\backups")}>
                {t("presets.folder")}
              </Button>
            </div>
          }
        >
          {backups.length === 0 && <Empty>{t("presets.emptyBackups")}</Empty>}
          <ul className="content-list">
            {backups.map((backup) => (
              <li key={backup.path}>
                <div className="content-main">
                  <strong>{backup.name}</strong>
                  <span>{backup.path}</span>
                </div>
                <Badge tone="muted">{fmtDate(backup.modified)}</Badge>
              </li>
            ))}
          </ul>
        </Card>
      </div>
    </div>
  );
}
