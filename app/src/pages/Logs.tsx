import { useEffect, useMemo, useState } from "react";
import { api, openPath } from "../api";
import { useApp } from "../store";
import { Badge, Button, Card } from "../components/ui";
import { Terminal } from "../components/Terminal";
import { useT } from "../i18n/core";
import type { LogEntry, LogFile } from "../types";
import { fmtBytes, fmtDate } from "../utils";

export function Logs() {
  const { t } = useT();
  const { logs, info, run, notify, refreshLogs } = useApp();
  const [filter, setFilter] = useState("");
  const [follow, setFollow] = useState(true);
  const [files, setFiles] = useState<LogFile[]>([]);
  const [selected, setSelected] = useState<LogFile | null>(null);
  const [preview, setPreview] = useState<LogEntry[]>([]);
  const [onlyErrors, setOnlyErrors] = useState(false);

  const loadFiles = async () => {
    try {
      const next = await api.listLogs();
      setFiles(next);
    } catch (error) {
      notify(String(error), "error");
    }
  };

  useEffect(() => {
    void loadFiles();
  }, []);

  const openFile = async (file: LogFile) => {
    setSelected(file);
    try {
      const text = await api.tailLog(file.path, 500);
      setPreview(
        text.split("\n").map((line) => ({ ts: "", stream: "file", message: line })),
      );
    } catch (error) {
      notify(String(error), "error");
    }
  };

  const visible = useMemo(() => {
    const query = filter.trim().toLowerCase();
    return logs.filter((line) => {
      if (onlyErrors && !/error|failed|kick|banned|could not|not found/i.test(line.message)) return false;
      if (!query) return true;
      return line.message.toLowerCase().includes(query);
    });
  }, [logs, filter, onlyErrors]);

  return (
    <div className="page">
      <Card
        title={t("logs.liveTitle")}
        subtitle={t(onlyErrors ? "logs.subtitleErrors" : "logs.subtitle", { n: visible.length })}
        actions={
          <div className="row gap">
            <input
              className="search small"
              placeholder={t("logs.filterPlaceholder")}
              value={filter}
              onChange={(event) => setFilter(event.target.value)}
            />
            <Button variant={onlyErrors ? "danger" : "ghost"} onClick={() => setOnlyErrors(!onlyErrors)}>
              {t("logs.errors")}
            </Button>
            <Button variant={follow ? "success" : "ghost"} onClick={() => setFollow(!follow)}>
              {follow ? t("logs.follow") : t("logs.paused")}
            </Button>
            <Button
              variant="ghost"
              onClick={() =>
                void run("clear", async () => {
                  await api.clearLogs();
                  await refreshLogs();
                  notify(t("logs.cleared"), "success");
                })
              }
            >
              {t("logs.clear")}
            </Button>
          </div>
        }
      >
        <Terminal lines={visible} height={420} follow={follow} />
      </Card>

      <Card
        title={t("logs.sessionsTitle")}
        subtitle={t("logs.sessionsSubtitle")}
        actions={
          <div className="row gap">
            <Button variant="ghost" onClick={() => void loadFiles()}>{t("logs.refresh")}</Button>
            <Button variant="ghost" onClick={() => void openPath((info?.server_dir ?? "") + "\\logs")}>
              {t("logs.openFolder")}
            </Button>
          </div>
        }
      >
        <div className="log-layout">
          <ul className="file-list">
            {files.length === 0 && <li className="muted">{t("logs.empty")}</li>}
            {files.slice(0, 40).map((file) => (
              <li key={file.path}>
                <button className={selected?.path === file.path ? "active" : ""} onClick={() => void openFile(file)}>
                  <span className="file-name">{file.name}</span>
                  <span className="file-meta">{fmtBytes(file.size)} · {fmtDate(file.modified)}</span>
                </button>
              </li>
            ))}
          </ul>
          <div className="preview">
            {!selected && <p className="muted">{t("logs.selectFile")}</p>}
            {selected && (
              <>
                <div className="row between">
                  <strong className="mono">{selected.name}</strong>
                  <div className="row gap">
                    <Badge tone="muted">{fmtBytes(selected.size)}</Badge>
                    <Button variant="ghost" onClick={() => void openPath(selected.path)}>{t("logs.openFile")}</Button>
                  </div>
                </div>
                <Terminal lines={preview} height={260} follow={false} />
              </>
            )}
          </div>
        </div>
      </Card>
    </div>
  );
}
