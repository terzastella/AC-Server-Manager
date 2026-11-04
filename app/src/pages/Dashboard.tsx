import { useEffect, useMemo, useState } from "react";
import { api } from "../api";
import { useApp } from "../store";
import { Badge, Button, Card, Empty, Stat } from "../components/ui";
import { Terminal } from "../components/Terminal";
import { QrCode } from "../components/QrCode";
import { useT } from "../i18n/core";
import { copyText, fmtUptime } from "../utils";
import type { PortState } from "../types";

const SESSIONS = [
  "dashboard.sessionPractice",
  "dashboard.sessionQualify",
  "dashboard.sessionRace",
] as const;

function playersFromLogs(logs: Array<{ message: string }>): string[] {
  const players = new Set<string>();
  for (const line of logs.slice(-400)) {
    const joined = line.message.match(/DRIVER: (.+?) \[\]/);
    if (joined) players.add(joined[1].trim());
    const closed = line.message.match(/Connection is now closed for (.+?) \[\]/);
    if (closed) players.delete(closed[1].trim());
    const terminated = line.message.match(/\[(.+?) \[\]\] terminated/);
    if (terminated) players.delete(terminated[1].trim());
  }
  return Array.from(players);
}

export function Dashboard() {
  const { t } = useT();
  const {
    status,
    config,
    content,
    adapters,
    logs,
    run,
    refreshStatus,
    notify,
    info,
  } = useApp();
  const [selectedIp, setSelectedIp] = useState("");
  const [serverInfo, setServerInfo] = useState<Record<string, unknown> | null>(null);
  const [ports, setPorts] = useState<PortState[]>([]);
  const [external, setExternal] = useState(false);

  const running = status?.running ?? false;
  const httpPort = useMemo(() => {
    const entry = config?.sections
      .find((section) => section.name === "SERVER")
      ?.entries.find((item) => item.key === "HTTP_PORT");
    return Number.parseInt(entry?.value ?? "8081", 10) || 8081;
  }, [config]);

  const gamePort = useMemo(() => {
    const entry = config?.sections
      .find((section) => section.name === "SERVER")
      ?.entries.find((item) => item.key === "UDP_PORT");
    return Number.parseInt(entry?.value ?? "9600", 10) || 9600;
  }, [config]);

  const serverName = value("SERVER", "NAME") || "—";
  const track = value("SERVER", "TRACK") || "—";
  const cars = (value("SERVER", "CARS") || "").split(";").filter(Boolean);
  const maxClients = value("SERVER", "MAX_CLIENTS") || "—";
  const hasPassword = Boolean(value("SERVER", "PASSWORD"));
  const ip = selectedIp || adapters[0]?.ip || "";

  const link = ip ? `acmanager://race/online/join?ip=${ip}&httpPort=${httpPort}` : "";

  function value(section: string, key: string): string {
    return (
      config?.sections
        .find((item) => item.name.toLowerCase() === section.toLowerCase())
        ?.entries.find((entry) => entry.key.toUpperCase() === key.toUpperCase())?.value ?? ""
    );
  }

  useEffect(() => {
    if (!selectedIp && adapters.length > 0) setSelectedIp(adapters[0].ip);
  }, [adapters, selectedIp]);

  useEffect(() => {
    let cancelled = false;
    const load = async () => {
      if (!running) {
        if (!cancelled) setServerInfo(null);
        return;
      }
      try {
        const result = await api.serverInfo(httpPort);
        if (!cancelled) setServerInfo(result);
      } catch {
        if (!cancelled) setServerInfo(null);
      }
    };
    void load();
    const timer = window.setInterval(load, 5000);
    return () => {
      cancelled = true;
      window.clearInterval(timer);
    };
  }, [running, httpPort]);

  useEffect(() => {
    const check = async () => {
      try {
        setPorts(await api.checkPorts([gamePort, httpPort]));
      } catch {
        /* ignore */
      }
    };
    void check();
    const timer = window.setInterval(check, 30000);
    return () => window.clearInterval(timer);
  }, [gamePort, httpPort, running]);

  const players = playersFromLogs(logs);
  const clientsOnline = players.length;
  const clientsMax = typeof serverInfo?.maxclients === "number" ? serverInfo.maxclients : maxClients;
  const sessionLabel =
    typeof serverInfo?.session === "number"
      ? SESSIONS[serverInfo.session]
        ? t(SESSIONS[serverInfo.session])
        : t("dashboard.sessionFallback", { n: serverInfo.session })
      : null;
  const timeLeft = typeof serverInfo?.timeleft === "number" ? serverInfo.timeleft : null;
  const warnings = content
    ? [
        ...content.missing_cars.map((id) => t("dashboard.missingCar", { id })),
        ...content.missing_tracks.map((id) => t("dashboard.missingTrack", { id })),
        ...content.missing_weather.map((id) => t("dashboard.missingWeather", { id })),
      ]
    : [];

  return (
    <div className="page">
      <section className={`hero ${running ? "hero-on" : ""}`}>
        <div className="hero-main">
          <span className={`hero-dot ${running ? "on" : ""}`} />
          <div>
            <h2>{running ? t("dashboard.serverRunning") : t("dashboard.serverStopped")}</h2>
            <p>
              {running
                ? t("dashboard.pidOnline", {
                    pid: status?.pid ?? "?",
                    time: fmtUptime(status?.uptime_secs ?? 0),
                  })
                : t("dashboard.noProcess")}
            </p>
          </div>
        </div>
        <div className="hero-actions">
          {running ? (
            <Button
              variant="danger"
              busy={false}
              onClick={() =>
                void run("stop", async () => {
                  await api.stop();
                  await refreshStatus();
                  notify(t("dashboard.stopped"), "success");
                })
              }
            >
              {t("dashboard.stopServer")}
            </Button>
          ) : (
            <Button
              variant="success"
              onClick={() =>
                void run("start", async () => {
                  try {
                    await api.start();
                  } catch (problem) {
                    if (
                      String(problem).includes("già un server AC") ||
                      String(problem).toLowerCase().includes("already")
                    ) {
                      setExternal(true);
                    }
                    throw problem;
                  }
                  setExternal(false);
                  await refreshStatus();
                  notify(t("dashboard.started"), "success");
                })
              }
            >
              {t("dashboard.startServer")}
            </Button>
          )}
          <Button
            disabled={!running}
            onClick={() =>
              void run("restart", async () => {
                await api.restart();
                await refreshStatus();
                notify(t("dashboard.restarted"), "success");
              })
            }
          >
            {t("dashboard.restart")}
          </Button>
        </div>
      </section>

      {external && !running && (
        <div className="banner banner-amber">
          <strong>{t("dashboard.externalTitle")}</strong>
          <span>{t("dashboard.externalText")}</span>
          <Button
            variant="danger"
            onClick={() =>
              void run("foreign", async () => {
                const count = await api.stopExternal();
                setExternal(false);
                await refreshStatus();
                notify(t("dashboard.externalStopped", { n: count }), "success");
              })
            }
          >
            {t("dashboard.stopExternal")}
          </Button>
        </div>
      )}

      {warnings.length > 0 && (
        <div className="banner banner-amber">
          <strong>{t("dashboard.missingTitle")}</strong>
          <span>{warnings.join(" · ")}</span>
          <Badge tone="amber">{warnings.length}</Badge>
        </div>
      )}

      {!info?.has_server && (
        <div className="banner banner-red">
          <strong>{t("dashboard.noServerTitle")}</strong>
          <span>{t("dashboard.noServerText")}</span>
        </div>
      )}

      <div className="grid grid-3">
        <Card title={t("dashboard.identity")}>
          <div className="stat-grid">
            <Stat label={t("dashboard.name")} value={serverName} />
            <Stat label={t("dashboard.track")} value={track} />
            <Stat label={t("dashboard.maxClients")} value={maxClients} />
            <Stat
              label={t("dashboard.password")}
              value={hasPassword ? t("dashboard.passwordOn") : t("dashboard.passwordNone")}
              tone={hasPassword ? "green" : "red"}
            />
          </div>
          <div className="chip-row">
            {cars.length === 0 && <span className="muted">{t("dashboard.noCars")}</span>}
            {cars.map((car) => (
              <span className="chip" key={car}>{car}</span>
            ))}
          </div>
        </Card>

        <Card
          title={t("dashboard.network")}
          subtitle={ip ? t("dashboard.ipSelected", { ip }) : t("dashboard.noIpDetected")}
        >
          <div className="field">
            <span className="field-label">{t("dashboard.adapter")}</span>
            <select value={ip} onChange={(event) => setSelectedIp(event.target.value)}>
              {adapters.length === 0 && <option value="">{t("dashboard.noIp")}</option>}
              {adapters.map((adapter) => (
                <option key={adapter.ip} value={adapter.ip}>
                  {adapter.kind} · {adapter.ip} ({adapter.name})
                </option>
              ))}
            </select>
          </div>
          <div className="link-box">
            <code>{link || "—"}</code>
          </div>
          <div className="row gap">
            <Button
              disabled={!link}
              onClick={() =>
                void copyText(link).then((ok) =>
                  notify(
                    ok ? t("dashboard.linkCopied") : t("dashboard.copyFailed"),
                    ok ? "success" : "error",
                  ),
                )
              }
            >
              {t("dashboard.copyLink")}
            </Button>
            <Button
              disabled={!ip}
              onClick={() =>
                void run("share", async () => {
                  const files = await api.writeShare(ip, httpPort);
                  notify(t("dashboard.filesUpdated", { n: files.length }), "success");
                })
              }
            >
              {t("dashboard.generateFiles")}
            </Button>
          </div>
        </Card>

        <Card
          title={t("dashboard.players")}
          subtitle={running ? t("dashboard.liveFrom", { port: httpPort }) : t("dashboard.serverStopped")}
        >
          {!running && <Empty>{t("dashboard.playersEmpty")}</Empty>}
          {running && (
            <div className="stat-grid">
              <Stat
                label={t("dashboard.connected")}
                value={`${clientsOnline} / ${clientsMax}`}
                tone={clientsOnline > 0 ? "green" : undefined}
              />
              <Stat label={t("dashboard.session")} value={sessionLabel ?? "—"} />
              <Stat
                label={t("dashboard.timeLeft")}
                value={timeLeft !== null ? fmtUptime(timeLeft) : "—"}
              />
              <Stat
                label={t("dashboard.pickup")}
                value={serverInfo?.pickup ? t("dashboard.pickupOn") : t("dashboard.pickupOff")}
              />
            </div>
          )}
          {running && players.length === 0 && (
            <Empty>
              {serverInfo ? t("dashboard.noPlayerNames") : t("dashboard.waitingServer")}
            </Empty>
          )}
          {players.length > 0 && (
            <ul className="player-list">
              {players.map((player, index) => (
                <li key={player}>
                  <span className="player-index">{index + 1}</span>
                  <div>
                    <strong>{player}</strong>
                  </div>
                </li>
              ))}
            </ul>
          )}
          {ip && running && (
            <div className="qr-row">
              <QrCode value={link} size={104} />
              <div className="qr-side">
                <Badge tone="green">{gamePort}/UDP</Badge>
                <Badge tone="blue">{httpPort}/TCP</Badge>
                <span className="muted small">{t("dashboard.scanQr")}</span>
              </div>
            </div>
          )}
        </Card>
      </div>

      <div className="grid grid-2">
        <Card
          title={t("dashboard.ports")}
          subtitle={t("dashboard.portsSubtitle")}
          actions={
            <Button
              variant="ghost"
              onClick={() =>
                void run("fw", async () => {
                  await api.openFirewall();
                  notify(t("dashboard.firewallRequested"), "info");
                })
              }
            >
              {t("dashboard.openFirewall")}
            </Button>
          }
        >
          <table className="table compact">
            <thead>
              <tr>
                <th>{t("dashboard.port")}</th>
                <th>TCP</th>
                <th>UDP</th>
              </tr>
            </thead>
            <tbody>
              {ports.map((port) => (
                <tr key={port.port}>
                  <td className="mono">{port.port}</td>
                  <td>
                    {port.tcp_free ? (
                      <Badge tone="green">{t("dashboard.free")}</Badge>
                    ) : (
                      <Badge tone="red">{t("dashboard.inUse")}</Badge>
                    )}
                  </td>
                  <td>
                    {port.udp_free ? (
                      <Badge tone="green">{t("dashboard.free")}</Badge>
                    ) : (
                      <Badge tone="amber">{t("dashboard.inUse")}</Badge>
                    )}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </Card>

        <Card title={t("dashboard.liveLog")}>
          <Terminal lines={logs.slice(-14)} height={190} />
        </Card>
      </div>
    </div>
  );
}
