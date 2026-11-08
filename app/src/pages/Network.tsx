import { useEffect, useMemo, useState } from "react";
import { api } from "../api";
import { useApp } from "../store";
import { Badge, Button, Card, Stat } from "../components/ui";
import { QrCode } from "../components/QrCode";
import { copyText } from "../utils";
import { useT } from "../i18n/core";
import type { PortState } from "../types";

export function Network() {
  const { t } = useT();
  const { config, adapters, run, notify, refreshAdapters } = useApp();
  const [selectedIp, setSelectedIp] = useState("");
  const [ports, setPorts] = useState<PortState[]>([]);

  const httpPort = Number.parseInt(
    config?.sections.find((section) => section.name === "SERVER")
      ?.entries.find((entry) => entry.key === "HTTP_PORT")?.value ?? "8081",
    10,
  );
  const udpPort = Number.parseInt(
    config?.sections.find((section) => section.name === "SERVER")
      ?.entries.find((entry) => entry.key === "UDP_PORT")?.value ?? "9600",
    10,
  );

  const ip = selectedIp || adapters[0]?.ip || "";
  const link = ip ? `acmanager://race/online/join?ip=${ip}&httpPort=${httpPort}` : "";

  useEffect(() => {
    if (!selectedIp && adapters.length > 0) setSelectedIp(adapters[0].ip);
  }, [adapters, selectedIp]);

  useEffect(() => {
    let cancelled = false;
    const check = async () => {
      try {
        const result = await api.checkPorts([udpPort, httpPort]);
        if (!cancelled) setPorts(result);
      } catch {
        /* ignore */
      }
    };
    void check();
    const timer = window.setInterval(check, 30000);
    return () => {
      cancelled = true;
      window.clearInterval(timer);
    };
  }, [udpPort, httpPort]);

  const grouped = useMemo(() => {
    const map = new Map<string, typeof adapters>();
    for (const adapter of adapters) {
      map.set(adapter.kind, [...(map.get(adapter.kind) ?? []), adapter]);
    }
    return Array.from(map.entries());
  }, [adapters]);

  return (
    <div className="page">
      <div className="grid grid-2">
        <Card
          title={t("network.addressesTitle")}
          subtitle={t("network.addressesSubtitle")}
          actions={<Button variant="ghost" onClick={() => void refreshAdapters()}>{t("network.refresh")}</Button>}
        >
          {grouped.length === 0 && <p className="muted">{t("network.noAdapters")}</p>}
          <div className="stack-sm">
            {grouped.map(([kind, list]) => (
              <div key={kind}>
                <span className="group-label">{kind}</span>
                {list.map((adapter) => (
                  <button
                    key={adapter.ip}
                    className={adapter.ip === ip ? "adapter active" : "adapter"}
                    onClick={() => setSelectedIp(adapter.ip)}
                  >
                    <span className="adapter-ip mono">{adapter.ip}</span>
                    <span className="adapter-name">{adapter.name}</span>
                    {adapter.ip === ip && <Badge tone="accent">{t("network.selected")}</Badge>}
                  </button>
                ))}
              </div>
            ))}
          </div>
        </Card>

        <Card title={t("network.linkTitle")} subtitle={t("network.linkSubtitle")}>
          <div className="qr-big">
            <QrCode value={link} size={168} />
            <div className="qr-info">
              <span className="qr-label">{t("network.directLink")}</span>
              <code className="link-box">{link || "—"}</code>
              <div className="row gap">
                <Button
                  disabled={!link}
                  onClick={() =>
                    void copyText(link).then((ok) =>
                      notify(
                        ok ? t("network.linkCopied") : t("network.copyFailed"),
                        ok ? "success" : "error",
                      ),
                    )
                  }
                >
                  {t("network.copy")}
                </Button>
                <Button
                  variant="primary"
                  disabled={!ip}
                  onClick={() =>
                    void run("share", async () => {
                      const files = await api.writeShare(ip, httpPort);
                      notify(t("network.filesGenerated", { n: files.length }), "success");
                    })
                  }
                >
                  {t("network.generateFiles")}
                </Button>
              </div>
            </div>
          </div>
          <p className="muted small">
            {t("network.generateHelpBefore")}
            <code>entra.url</code>
            {t("network.generateHelpComma")}
            <code>entra-locale.url</code>
            {t("network.generateHelpAnd")}
            <code>server-info.txt</code>
            {t("network.generateHelpAfter")}
          </p>
        </Card>
      </div>

      <div className="grid grid-2">
        <Card title={t("network.portsTitle")} subtitle={t("network.portsSubtitle")}>
          <table className="table compact">
            <thead>
              <tr><th>{t("network.colPort")}</th><th>{t("network.colProto")}</th><th>{t("network.colStatus")}</th></tr>
            </thead>
            <tbody>
              {ports.map((port) => (
                <tr key={port.port}>
                  <td className="mono">{port.port}</td>
                  <td className="mono">TCP</td>
                  <td>{port.tcp_free ? <Badge tone="green">{t("network.free")}</Badge> : <Badge tone="amber">{t("network.listening")}</Badge>}</td>
                </tr>
              ))}
              {ports.map((port) => (
                <tr key={`${port.port}-udp`}>
                  <td className="mono">{port.port}</td>
                  <td className="mono">UDP</td>
                  <td>{port.udp_free ? <Badge tone="green">{t("network.free")}</Badge> : <Badge tone="amber">{t("network.listening")}</Badge>}</td>
                </tr>
              ))}
            </tbody>
          </table>
          <p className="muted small">{t("network.portsNote")}</p>
        </Card>

        <Card title={t("network.firewallTitle")} subtitle={t("network.firewallSubtitle")}>
          <p className="muted">
            {t("network.firewallHelpBefore")}
            <code>apri-porte-firewall.ps1</code>
            {t("network.firewallHelpAfter")}
          </p>
          <Button
            variant="primary"
            onClick={() =>
              void run("firewall", async () => {
                await api.openFirewall();
                notify(t("network.uacSent"), "info");
              })
            }
          >
            {t("network.openPorts")}
          </Button>
        </Card>
      </div>

      <div className="grid grid-4">
        <Stat
          label={t("network.statIp")}
          value={<span className="mono">{ip || "—"}</span>}
        />
        <Stat label={t("network.statGamePort")} value={udpPort} hint="UDP" />
        <Stat label={t("network.statHttpPort")} value={httpPort} hint="TCP" />
        <Stat label={t("network.statAdapters")} value={adapters.length} />
      </div>
    </div>
  );
}
