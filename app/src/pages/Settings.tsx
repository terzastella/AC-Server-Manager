import { useEffect, useState } from "react";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { api, openPath } from "../api";
import { useApp } from "../store";
import { Badge, Button, Card, Stat } from "../components/ui";
import { useT } from "../i18n/core";

export function Settings() {
  const { t, lang, setLang } = useT();
  const { info, settings, run, refreshAll, notify } = useApp();
  const [gameDir, setGameDir] = useState("");
  const [cmDir, setCmDir] = useState("");

  useEffect(() => {
    setGameDir(settings?.game_dir ?? "");
  }, [settings?.game_dir]);

  useEffect(() => {
    setCmDir(settings?.cm_dir ?? "");
  }, [settings?.cm_dir]);

  const pickServer = async () => {
    const selected = await openDialog({
      directory: true,
      multiple: false,
      title: t("settings.pickServerTitle"),
    });
    if (typeof selected !== "string") return;
    void run("server-dir", async () => {
      const next = await api.setServerDir(selected);
      await refreshAll();
      notify(t("settings.notifyServerDir", { path: next.server_dir }), "success");
    });
  };

  const pickGame = async () => {
    const selected = await openDialog({
      directory: true,
      multiple: false,
      title: t("settings.pickGameTitle"),
    });
    if (typeof selected !== "string") return;
    setGameDir(selected);
    await saveGameDir(selected);
  };

  const saveGameDir = async (dir = gameDir) => {
    if (!settings) return;
    void run("game-dir", async () => {
      await api.saveSettings({ ...settings, game_dir: dir || null });
      await refreshAll();
      notify(t("settings.notifyGameSaved"), "success");
    });
  };

  const pickCm = async () => {
    const selected = await openDialog({
      directory: true,
      multiple: false,
      title: t("settings.pickCmTitle"),
    });
    if (typeof selected !== "string") return;
    setCmDir(selected);
    await saveCmDir(selected);
  };

  const saveCmDir = async (dir = cmDir) => {
    if (!settings) return;
    void run("cm-dir", async () => {
      await api.saveSettings({ ...settings, cm_dir: dir || null });
      await refreshAll();
      notify(t("settings.notifyCmSaved"), "success");
    });
  };

  const serverDir = info?.server_dir ?? "";

  return (
    <div className="page">
      <div className="grid grid-2">
        <Card
          title={t("settings.serverTitle")}
          subtitle={t("settings.serverSubtitle")}
        >
          <code className="path-box">{serverDir || t("settings.notSet")}</code>
          <div className="row gap top-gap">
            <Button variant="primary" onClick={() => void pickServer()}>
              {t("settings.changeFolder")}
            </Button>
            <Button variant="ghost" onClick={() => void openPath(serverDir || ".")}>
              {t("common.open")}
            </Button>
          </div>
          <p className="muted small">{t("settings.serverHelp")}</p>
        </Card>

        <Card
          title={t("settings.gameTitle")}
          subtitle={t("settings.gameSubtitle")}
        >
          <div className="row gap">
            <input
              className="grow mono"
              placeholder="C:\\Game\\nuovo\\Assetto Corsa"
              value={gameDir}
              onChange={(event) => setGameDir(event.target.value)}
            />
            <Button onClick={() => void pickGame()}>{t("settings.browse")}</Button>
          </div>
          <div className="row gap top-gap">
            <Button variant="primary" onClick={() => void saveGameDir()}>
              {t("common.save")}
            </Button>
            <Button
              variant="ghost"
              onClick={() => void openPath(gameDir || ".")}
              disabled={!gameDir}
            >
              {t("common.open")}
            </Button>
          </div>
        </Card>
      </div>

      <Card title={t("settings.cmTitle")} subtitle={t("settings.cmSubtitle")}>
        <div className="row gap">
          <input
            className="grow mono"
            placeholder={t("settings.autoDetect")}
            value={cmDir}
            onChange={(event) => setCmDir(event.target.value)}
          />
          <Button onClick={() => void pickCm()}>{t("settings.browse")}</Button>
        </div>
        <div className="row gap top-gap">
          <Button variant="primary" onClick={() => void saveCmDir()}>
            {t("common.save")}
          </Button>
          <Button
            variant="ghost"
            onClick={() => {
              setCmDir("");
              void saveCmDir("");
            }}
          >
            {t("settings.auto")}
          </Button>
        </div>
      </Card>

      <div className="grid grid-3">
        <Stat label={t("settings.appVersion")} value={info?.version ?? "—"} />
        <Stat
          label="server_cfg.ini"
          value={serverDir ? t("settings.linked") : t("settings.notFound")}
          tone={serverDir ? "green" : "red"}
        />
        <Stat
          label="settings.json"
          value="portable"
          hint={t("settings.besideExe")}
        />
      </div>

      <Card title={t("common.language")} subtitle={t("settings.languageSubtitle")}>
        <div className="row gap">
          <Button
            variant={lang === "it" ? "primary" : "ghost"}
            aria-pressed={lang === "it"}
            onClick={() => setLang("it")}
          >
            {t("common.italian")}
          </Button>
          <Button
            variant={lang === "en" ? "primary" : "ghost"}
            aria-pressed={lang === "en"}
            onClick={() => setLang("en")}
          >
            {t("common.english")}
          </Button>
        </div>
      </Card>

      <div className="grid grid-2">
        <Card title={t("settings.toolsTitle")}>
          <div className="stack-sm">
            <Button
              variant="primary"
              onClick={() =>
                void run("backup", async () => {
                  const path = await api.backupNow();
                  notify(t("settings.notifyBackup", { path }), "success");
                })
              }
            >
              {t("settings.backupConfig")}
            </Button>
            <Button onClick={() => void openPath(serverDir + "\\cfg")}>
              {t("settings.openCfg")}
            </Button>
            <Button onClick={() => void openPath(serverDir + "\\logs")}>
              {t("settings.openLogs")}
            </Button>
            <Button onClick={() => void openPath(serverDir + "\\content")}>
              {t("settings.openContent")}
            </Button>
            <Button onClick={() => void openPath(serverDir + "\\README.md")}>
              {t("settings.openReadme")}
            </Button>
          </div>
        </Card>

        <Card title={t("settings.infoTitle")}>
          <div className="info-list">
            <div><span>{t("settings.exe")}</span><code>{info?.exe_path ?? "—"}</code></div>
            <div><span>{t("settings.serverTitle")}</span><code>{serverDir || "—"}</code></div>
            <div><span>{t("settings.mode")}</span><Badge tone="green">portable · Tauri + WebView2</Badge></div>
            <div><span>{t("settings.backup")}</span><span className="muted">{t("settings.backupHint")}</span></div>
          </div>
          <p className="muted small">{t("settings.prototypeNote")}</p>
        </Card>
      </div>
    </div>
  );
}
