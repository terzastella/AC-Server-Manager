import { useState } from "react";
import { AppProvider, useApp } from "./store";
import { cx } from "./components/ui";
import { Dashboard } from "./pages/Dashboard";
import { Config } from "./pages/Config";
import { Slots } from "./pages/Slots";
import { Content } from "./pages/Content";
import { Cm } from "./pages/Cm";
import { Network } from "./pages/Network";
import { Invite } from "./pages/Invite";
import { Presets } from "./pages/Presets";
import { Logs } from "./pages/Logs";
import { Settings } from "./pages/Settings";
import { Servers } from "./pages/Servers";
import { useT, type TKey } from "./i18n/core";
import { profileName } from "./profileName";
import { fmtUptime } from "./utils";

type PageId =
  | "dashboard"
  | "config"
  | "slots"
  | "content"
  | "cm"
  | "network"
  | "invite"
  | "servers"
  | "presets"
  | "logs"
  | "settings";

const NAV: Array<{ id: PageId; icon: React.ReactNode }> = [
  { id: "dashboard", icon: <IconDashboard /> },
  { id: "config", icon: <IconSliders /> },
  { id: "slots", icon: <IconCar /> },
  { id: "content", icon: <IconBox /> },
  { id: "cm", icon: <IconImport /> },
  { id: "network", icon: <IconGlobe /> },
  { id: "invite", icon: <IconShare /> },
  { id: "servers", icon: <IconLayers /> },
  { id: "presets", icon: <IconLayers /> },
  { id: "logs", icon: <IconTerminal /> },
  { id: "settings", icon: <IconGear /> },
];

const TITLES: Record<PageId, { title: TKey; subtitle: TKey }> = {
  dashboard: { title: "page.dashboard.title", subtitle: "page.dashboard.subtitle" },
  config: { title: "page.config.title", subtitle: "page.config.subtitle" },
  slots: { title: "page.slots.title", subtitle: "page.slots.subtitle" },
  content: { title: "page.content.title", subtitle: "page.content.subtitle" },
  cm: { title: "page.cm.title", subtitle: "page.cm.subtitle" },
  network: { title: "page.network.title", subtitle: "page.network.subtitle" },
  invite: { title: "page.invite.title", subtitle: "page.invite.subtitle" },
  servers: { title: "page.servers.title", subtitle: "page.servers.subtitle" },
  presets: { title: "page.presets.title", subtitle: "page.presets.subtitle" },
  logs: { title: "page.logs.title", subtitle: "page.logs.subtitle" },
  settings: { title: "page.settings.title", subtitle: "page.settings.subtitle" },
};

function Shell() {
  const [page, setPage] = useState<PageId>("dashboard");
  const { status, info, logs, profiles, switchProfile } = useApp();
  const { t, lang, setLang } = useT();
  const running = status?.running ?? false;
  const lastLog = logs.length ? logs[logs.length - 1] : null;

  return (
    <div className="app">
      <aside className="sidebar">
        <div className="brand">
          <img className="brand-mark" src="/logo-manager.png" alt="AC Server Control" />
          <div className="brand-text">
            <strong>SERVER</strong>
            <span>CONTROL</span>
          </div>
        </div>

        <div className="profile-switch">
          <label htmlFor="active-profile">{t("shell.activeProfile")}</label>
          <select
            id="active-profile"
            value={profiles?.active ?? ""}
            disabled={running}
            onChange={(event) => void switchProfile(event.target.value)}
          >
            {(profiles?.profiles ?? []).map((profile) => (
              <option key={profile.id} value={profile.id}>
                {profileName(profile, t)}
              </option>
            ))}
          </select>
          {running && <span>{t("shell.stopToSwitch")}</span>}
        </div>

        <nav className="nav">
          {NAV.map((item) => (
            <button
              key={item.id}
              className={cx("nav-item", page === item.id && "active")}
              onClick={() => setPage(item.id)}
            >
              {item.icon}
              <span>{t(`nav.${item.id}` as TKey)}</span>
            </button>
          ))}
        </nav>

        <div className="sidebar-foot">
          <div className={cx("pill", running ? "pill-green" : "pill-muted")}>
            <span className="dot" />
            {running
              ? t("shell.running", { time: fmtUptime(status?.uptime_secs ?? 0) })
              : t("shell.stopped")}
          </div>
          <div className="lang-switch" role="group" aria-label={t("common.language")}>
            <button
              type="button"
              className={cx("lang-btn", lang === "it" && "active")}
              onClick={() => setLang("it")}
            >
              IT
            </button>
            <button
              type="button"
              className={cx("lang-btn", lang === "en" && "active")}
              onClick={() => setLang("en")}
            >
              EN
            </button>
          </div>
          <div className="path" title={info?.server_dir ?? ""}>
            {info?.server_dir || t("shell.noFolder")}
          </div>
        </div>
      </aside>

      <main className="main">
        <header className="topbar">
          <div>
            <h1>{t(TITLES[page].title)}</h1>
            <p>{t(TITLES[page].subtitle)}</p>
          </div>
          <div className="topbar-status">
            {lastLog ? (
              <span className="last-event" title={lastLog.message}>
                <span className="event-dot" /> {lastLog.message}
              </span>
            ) : (
              <span className="last-event muted">{t("shell.noEvent")}</span>
            )}
          </div>
        </header>

        <div className="content">
          {page === "dashboard" && <Dashboard />}
          {page === "config" && <Config />}
          {page === "slots" && <Slots />}
          {page === "content" && <Content />}
          {page === "cm" && <Cm />}
          {page === "network" && <Network />}
          {page === "invite" && <Invite />}
          {page === "servers" && <Servers />}
          {page === "presets" && <Presets />}
          {page === "logs" && <Logs />}
          {page === "settings" && <Settings />}
        </div>
      </main>

      <Toasts />
    </div>
  );
}

function Toasts() {
  const { toasts, dismiss } = useApp();
  return (
    <div className="toasts">
      {toasts.map((toast) => (
        <button key={toast.id} className={cx("toast", `toast-${toast.tone}`)} onClick={() => dismiss(toast.id)}>
          {toast.text}
        </button>
      ))}
    </div>
  );
}

export default function App() {
  return (
    <AppProvider>
      <Shell />
    </AppProvider>
  );
}

// ------------------------------------------------------------------ icons

function IconDashboard() {
  return (
    <svg viewBox="0 0 24 24"><rect x="3" y="3" width="8" height="8" rx="2" /><rect x="13" y="3" width="8" height="5" rx="2" /><rect x="13" y="10" width="8" height="11" rx="2" /><rect x="3" y="13" width="8" height="8" rx="2" /></svg>
  );
}
function IconSliders() {
  return (
    <svg viewBox="0 0 24 24"><path d="M4 6h10M18 6h2M4 12h4M12 12h8M4 18h12M20 18h0" /><circle cx="16" cy="6" r="2" /><circle cx="10" cy="12" r="2" /><circle cx="18" cy="18" r="2" /></svg>
  );
}
function IconCar() {
  return (
    <svg viewBox="0 0 24 24"><path d="M5 16l1.5-5h11L19 16M4 16h16v3h-2v-2H6v2H4v-3z" /><circle cx="7.5" cy="16.5" r="1" /><circle cx="16.5" cy="16.5" r="1" /></svg>
  );
}
function IconBox() {
  return (
    <svg viewBox="0 0 24 24"><path d="M12 3l8 4.5v9L12 21l-8-4.5v-9L12 3z" /><path d="M4 7.5l8 4.5 8-4.5M12 12v9" /></svg>
  );
}
function IconImport() {
  return (
    <svg viewBox="0 0 24 24"><path d="M12 3v10M8 9l4 4 4-4" /><path d="M4 16v3a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2v-3" /></svg>
  );
}
function IconGlobe() {
  return (
    <svg viewBox="0 0 24 24"><circle cx="12" cy="12" r="9" /><path d="M3 12h18M12 3c3 3.5 3 14 0 18M12 3c-3 3.5-3 14 0 18" /></svg>
  );
}
function IconShare() {
  return (
    <svg viewBox="0 0 24 24"><circle cx="18" cy="5" r="3" /><circle cx="6" cy="12" r="3" /><circle cx="18" cy="19" r="3" /><path d="M8.6 10.6l6.8-4.2M8.6 13.4l6.8 4.2" /></svg>
  );
}
function IconLayers() {
  return (
    <svg viewBox="0 0 24 24"><path d="M12 3l9 5-9 5-9-5 9-5z" /><path d="M3 13l9 5 9-5" /></svg>
  );
}
function IconTerminal() {
  return (
    <svg viewBox="0 0 24 24"><rect x="3" y="4" width="18" height="16" rx="2" /><path d="M7 9l3 3-3 3M13 15h4" /></svg>
  );
}
function IconGear() {
  return (
    <svg viewBox="0 0 24 24"><circle cx="12" cy="12" r="3" /><path d="M12 2v3M12 19v3M2 12h3M19 12h3M4.9 4.9l2.1 2.1M17 17l2.1 2.1M19.1 4.9L17 7M7 17l-2.1 2.1" /></svg>
  );
}
