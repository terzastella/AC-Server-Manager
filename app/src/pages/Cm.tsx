import { useEffect, useMemo, useState } from "react";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { api, openPath } from "../api";
import { useApp } from "../store";
import { Badge, Button, Card, Empty, Stat } from "../components/ui";
import { Thumb } from "../components/Thumb";
import { useT } from "../i18n/core";
import type { CmScan, CmSetting } from "../types";
import { copyText, fmtBytes } from "../utils";

export function Cm() {
  const { t } = useT();
  const { settings, run, notify, refreshAll } = useApp();
  const [scan, setScan] = useState<CmScan | null>(null);
  const [draft, setDraft] = useState<CmSetting[]>([]);
  const [search, setSearch] = useState("");
  const [group, setGroup] = useState("__all");
  const [loading, setLoading] = useState(true);
  const [expanded, setExpanded] = useState<Record<string, boolean>>({});

  const load = async () => {
    setLoading(true);
    try {
      const next = await api.cmScan();
      setScan(next);
      setDraft(next.settings);
    } catch (error) {
      notify(String(error), "error");
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    void load();
  }, []);

  const groups = useMemo(
    () => Array.from(new Set(draft.map((entry) => entry.group))).sort((a, b) => a.localeCompare(b)),
    [draft],
  );

  const visible = useMemo(() => {
    const query = search.trim().toLowerCase();
    return draft
      .filter((entry) => {
        if (group !== "__all" && entry.group !== group) return false;
        if (!query) return true;
        return (
          entry.key.toLowerCase().includes(query) ||
          entry.value.toLowerCase().includes(query) ||
          entry.group.toLowerCase().includes(query)
        );
      })
      .sort((a, b) => a.group.localeCompare(b.group) || a.key.localeCompare(b.key));
  }, [draft, search, group]);

  const dirty = useMemo(
    () => JSON.stringify(draft) !== JSON.stringify(scan?.settings ?? []),
    [draft, scan],
  );

  const visibleGroups = useMemo(() => {
    const map = new Map<string, CmSetting[]>();
    for (const entry of visible) {
      const list = map.get(entry.group) ?? [];
      list.push(entry);
      map.set(entry.group, list);
    }
    return Array.from(map.entries());
  }, [visible]);

  const update = (key: string, value: string) => {
    setDraft((current) =>
      current.map((entry) => (entry.key === key ? { ...entry, value } : entry)),
    );
  };

  const pickDataDir = async () => {
    const selected = await openDialog({
      directory: true,
      multiple: false,
      title: t("cm.pickDataDirTitle"),
    });
    if (typeof selected !== "string" || !settings) return;
    void run("cm-dir", async () => {
      await api.saveSettings({ ...settings, cm_dir: selected });
      await refreshAll();
      await load();
      notify(t("cm.cmFolderUpdated"), "success");
    });
  };

  const resetDataDir = async () => {
    if (!settings) return;
    void run("cm-reset", async () => {
      await api.saveSettings({ ...settings, cm_dir: null });
      await refreshAll();
      await load();
      notify(t("cm.autoDetectRestored"), "success");
    });
  };

  const useAcRoot = async () => {
    const acRoot = scan?.location.ac_root;
    if (!acRoot || !settings) return;
    void run("use-ac-root", async () => {
      await api.saveSettings({ ...settings, game_dir: acRoot });
      await refreshAll();
      notify(t("cm.gameFolderSet"), "success");
    });
  };

  const location = scan?.location;
  const settingsList = scan?.settings ?? [];

  return (
    <div className="page">
      <div className="banner banner-blue">
        <strong>{t("cm.readOnly")}</strong>
        <span>
          {t("cm.readOnlyHelp")}
        </span>
        {dirty && <Badge tone="amber">{t("cm.localChanges")}</Badge>}
      </div>

      {scan?.error && (
        <div className="banner banner-amber">
          <strong>{t("cm.warning")}</strong>
          <span>{scan.error}</span>
        </div>
      )}

      <div className="grid grid-2">
        <Card
          title={t("cm.detectedInstall")}
          subtitle={location?.running ? t("cm.cmRunning") : t("cm.cmNotRunning")}
          actions={
            <div className="row gap">
              <Button variant="ghost" onClick={() => void load()} busy={loading}>{t("cm.reload")}</Button>
              <Button variant="ghost" onClick={() => void pickDataDir()}>{t("cm.changeFolder")}</Button>
              <Button variant="ghost" onClick={() => void resetDataDir()}>{t("cm.auto")}</Button>
            </div>
          }
        >
          <div className="info-list">
            <div><span>{t("cm.executable")}</span><code>{location?.cm_exe ?? t("cm.notDetected")}</code></div>
            <div><span>{t("cm.version")}</span><span className="mono">{location?.version ?? "—"}</span></div>
            <div><span>{t("cm.dataFolder")}</span><code>{location?.data_dir ?? t("cm.folderNotFound")}</code></div>
            <div><span>Values.data</span><code>{location?.values_file ?? t("cm.fileNotFound")}</code></div>
            <div><span>AC root</span><code>{location?.ac_root ?? "—"}</code></div>
          </div>
          <div className="row gap top-gap">
            <Button disabled={!location?.ac_root} onClick={() => void useAcRoot()}>
              {t("cm.useAcRoot")}
            </Button>
            <Button variant="ghost" disabled={!location?.data_dir} onClick={() => void openPath(location?.data_dir ?? "")}>
              {t("cm.openDataFolder")}
            </Button>
          </div>
        </Card>

        <div className="stat-grid">
          <Stat label={t("cm.importedKeys")} value={scan?.total_keys ?? "—"} hint={t("cm.importedKeysHint")} />
          <Stat label={t("cm.serverPresets")} value={scan?.server_presets.length ?? "—"} hint="server\\presets" />
          <Stat label={t("cm.userPresets")} value={scan?.user_presets.length ?? "—"} hint={t("cm.userPresetsHint")} />
          <Stat label={t("cm.cmState")} value={location?.running ? t("cm.stateOpen") : t("cm.stateClosed")} tone={location?.running ? "red" : "green"} />
        </div>
      </div>

      <Card
        title={t("cm.settingsTitle")}
        subtitle={t("cm.settingsSubtitle", { visible: visible.length, total: settingsList.length })}
        actions={
          <div className="row gap">
            <select value={group} onChange={(event) => setGroup(event.target.value)}>
              <option value="__all">{t("cm.allGroups")}</option>
              {groups.map((name) => (
                <option key={name} value={name}>{name}</option>
              ))}
            </select>
            <input
              className="search small"
              placeholder={t("cm.searchPlaceholder")}
              value={search}
              onChange={(event) => setSearch(event.target.value)}
            />
            <Button
              disabled={settingsList.length === 0}
              onClick={() =>
                void copyText(JSON.stringify(draft, null, 2)).then((ok) =>
                  notify(ok ? t("cm.jsonCopied") : t("cm.copyFailed"), ok ? "success" : "error"),
                )
              }
            >
              {t("cm.copyJson")}
            </Button>
            <Button
              variant="primary"
              disabled={settingsList.length === 0}
              onClick={() =>
                void run("cm-snapshot", async () => {
                  const path = await api.cmSaveSnapshot(draft, location?.values_file ?? null);
                  notify(t("cm.snapshotSaved", { path }), "success");
                })
              }
            >
              {t("cm.saveSnapshot")}
            </Button>
          </div>
        }
      >
        {settingsList.length === 0 && !loading && <Empty>{t("cm.noData")}</Empty>}
        {settingsList.length > 0 && (
          <>
            <div className="cm-groupbar">
              <Button
                variant="ghost"
                onClick={() =>
                  setExpanded(Object.fromEntries(groups.map((name) => [name, true])))
                }
              >
                {t("cm.expandAll")}
              </Button>
              <Button variant="ghost" onClick={() => setExpanded({})}>{t("cm.collapse")}</Button>
              <span className="muted small">{t("cm.groupCount", { n: visibleGroups.length })}</span>
            </div>
            <div className="cm-list">
              {visibleGroups.map(([groupName, entries]) => {
                const open = Boolean(search.trim()) || expanded[groupName];
                return (
                  <div className="cm-group" key={groupName}>
                    <button
                      className="cm-group-head"
                      onClick={() => setExpanded((current) => ({ ...current, [groupName]: !open }))}
                    >
                      <span>{groupName}</span>
                      <em>{entries.length}</em>
                      <b>{open ? "−" : "+"}</b>
                    </button>
                    {open &&
                      entries.map((entry) => (
                        <div className="cm-row" key={entry.key}>
                          <div className="cm-key">
                            <code>{entry.key}</code>
                            <div className="cm-meta">
                              <Badge tone={entry.kind === "json" ? "blue" : "muted"}>{entry.kind}</Badge>
                              {entry.encrypted && <Badge tone="amber">{t("cm.encrypted")}</Badge>}
                            </div>
                          </div>
                          <input
                            value={entry.value}
                            spellCheck={false}
                            onChange={(event) => update(entry.key, event.target.value)}
                          />
                        </div>
                      ))}
                  </div>
                );
              })}
              {visible.length === 0 && <Empty>{t("cm.noKeys")}</Empty>}
            </div>
          </>
        )}
      </Card>

      <Card
        title={t("cm.serverPresetsTitle")}
        subtitle={location?.ac_root ? t("cm.presetsFrom", { path: location.ac_root + "\\server\\presets" }) : t("cm.acRootMissing")}
        actions={<Button variant="ghost" onClick={() => void load()} busy={loading}>{t("cm.refresh")}</Button>}
      >
        {(scan?.server_presets.length ?? 0) === 0 && (
          <Empty>{t("cm.noPresets")}</Empty>
        )}
        <div className="preset-grid">
          {(scan?.server_presets ?? []).map((preset) => (
            <div className="preset-card" key={preset.path}>
              <Thumb src={preset.preview} alt={preset.track} fallback={preset.track || preset.name} className="preset-cover" />
              <div className="preset-head">
                <strong>{preset.server_name || preset.name}</strong>
                <Badge tone="muted">{preset.name}</Badge>
              </div>
              <div className="preset-meta">
                <span>{t("cm.trackLabel")} <b>{preset.track || "—"}</b></span>
                <span>{t("cm.carsLabel")} <b>{preset.cars.join(", ") || "—"}</b></span>
                <span>{t("cm.slotsLabel")} <b>{preset.max_clients || "—"}</b> · {t("cm.passwordLabel")} <b>{preset.password ? t("cm.yes") : t("cm.no")}</b></span>
              </div>
              <div className="chip-row">
                {preset.files.map((file) => <span className="chip" key={file}>{file}</span>)}
                {preset.has_wrapper && <span className="chip chip-on">cm_wrapper_params.json</span>}
              </div>
              <div className="row gap">
                <Button
                  variant="primary"
                  onClick={() =>
                    void run("cm-import", async () => {
                      const destination = await api.cmImportPreset(preset.path);
                      notify(t("cm.presetImported", { path: destination }), "success");
                    })
                  }
                >
                  {t("cm.importPreset")}
                </Button>
                <Button variant="ghost" onClick={() => void openPath(preset.path)}>{t("cm.open")}</Button>
              </div>
            </div>
          ))}
        </div>
      </Card>

      <Card title={t("cm.userPresetsTitle")} subtitle={t("cm.userPresetsSubtitle")}>
        {(scan?.user_presets.length ?? 0) === 0 ? (
          <Empty>{t("cm.noUserPresets")}</Empty>
        ) : (
          <div className="cm-list">
            {scan?.user_presets.map((preset) => (
              <div className="cm-row" key={preset.path}>
                <div className="cm-key">
                  <code>{preset.name}</code>
                  <div className="cm-meta"><Badge tone="muted">{preset.category || "root"}</Badge></div>
                </div>
                <div className="row gap">
                  <Badge tone="muted">{fmtBytes(preset.size)}</Badge>
                  <Button variant="ghost" onClick={() => void openPath(preset.path)}>{t("cm.show")}</Button>
                </div>
              </div>
            ))}
          </div>
        )}
      </Card>
    </div>
  );
}
