import { useEffect, useState } from "react";
import { api } from "./api";
import { AssistsPanel } from "./AssistsPanel";
import { JoinWindow } from "./JoinWindow";
import { Button } from "./components/ui";
import { useT } from "./i18n/core";
import type { JoinArgs, LauncherState } from "./types";

function parseLink(link: string): JoinArgs | null {
  const query = link.split("?")[1];
  if (!query) return null;
  const params = new Map<string, string>();
  for (const pair of query.split("&")) {
    const [key, value = ""] = pair.split("=");
    let decoded = value;
    try {
      decoded = decodeURIComponent(value);
    } catch {
      // link incollato male o troncato: meglio il valore grezzo che un crash.
      decoded = value;
    }
    params.set(key.toLowerCase(), decoded);
  }
  const ip = params.get("ip") ?? "";
  const httpPort = Number.parseInt(params.get("httpport") ?? "", 10);
  const apiPort = Number.parseInt(params.get("apiport") ?? "", 10);
  const token = params.get("token") ?? "";
  if (!ip || !httpPort || !apiPort || !token) return null;
  return {
    ip,
    http_port: httpPort,
    api_port: apiPort,
    token,
    name: params.get("n") ?? "",
    accent: params.get("c") ?? "#ff5a1f",
    banner: params.get("b") ?? "",
  };
}

type CardInfo = { track: string; free: number; max: number; online: boolean };

export function LauncherHome() {
  const { t, lang, setLang } = useT();
  const [state, setState] = useState<LauncherState | null>(null);
  const [input, setInput] = useState("");
  const [error, setError] = useState("");
  const [driver, setDriver] = useState("");
  const [selected, setSelected] = useState<JoinArgs | null>(null);
  const [cards, setCards] = useState<Record<string, CardInfo>>({});
  const [refresh, setRefresh] = useState(0);

  useEffect(() => {
    void api.launcherState().then((value) => {
      setState(value);
      setDriver(value.driver_name);
    });
  }, []);

  useEffect(() => {
    if (!state) return;
    let cancelled = false;
    state.invites.slice(0, 8).forEach((invite) => {
      const args = parseLink(invite.link);
      if (!args) return;
      api
        .remoteServerInfo(args.ip, args.api_port, args.token)
        .then((info) => {
          if (cancelled) return;
          setCards((current) => ({
            ...current,
            [invite.link]: {
              track: info.track,
              free: info.free_slots,
              max: info.max_clients,
              online: true,
            },
          }));
        })
        .catch(() => {
          if (cancelled) return;
          setCards((current) => ({
            ...current,
            [invite.link]: { track: "", free: 0, max: 0, online: false },
          }));
        });
    });
    return () => {
      cancelled = true;
    };
  }, [state, refresh]);

  if (selected) {
    return (
      <JoinWindow
        initialArgs={selected}
        initialDriver={driver}
        onBack={() => setSelected(null)}
      />
    );
  }

  const add = async () => {
    setError("");
    try {
      const value = await api.addInvite(input.trim());
      setState(value);
      setInput("");
    } catch (problem) {
      setError(String(problem));
    }
  };

  return (
    <div className="launcher">
      <header className="launcher-head">
        <div className="launcher-brand">
          <img className="launcher-logo" src="/logo-launcher-badge.svg" alt="AC Launcher" />
          <div>
            <strong>ASSETTO CORSA</strong>
            <span>{t("launcher.subtitle")}</span>
          </div>
        </div>
        <div className="launcher-head-tools">
          <div className="lang-switch" role="group" aria-label={t("common.language")}>
            <button
              type="button"
              className={`lang-btn${lang === "it" ? " active" : ""}`}
              onClick={() => setLang("it")}
            >
              IT
            </button>
            <button
              type="button"
              className={`lang-btn${lang === "en" ? " active" : ""}`}
              onClick={() => setLang("en")}
            >
              EN
            </button>
          </div>
          <label className="launcher-driver">
            <span>{t("launcher.yourName")}</span>
            <input
              value={driver}
              placeholder={t("launcher.driverPlaceholder")}
              onChange={(event) => setDriver(event.target.value)}
              onBlur={() => void api.setDriverName(driver)}
            />
          </label>
        </div>
      </header>

      <main className="launcher-body">
        <section className="launcher-invites">
          <div className="launcher-invites-head">
            <h1>{t("launcher.serversTitle")}</h1>
            <Button
              variant="ghost"
              onClick={() => {
                setCards({});
                setRefresh((value) => value + 1);
              }}
            >
              {t("launcher.refresh")}
            </Button>
          </div>
          {state && state.invites.length === 0 && (
            <p className="muted">{t("launcher.noInvites")}</p>
          )}
          <div className="launcher-grid">
            {(state?.invites ?? []).map((invite) => {
              const args = parseLink(invite.link);
              const info = cards[invite.link];
              return (
                <article key={invite.link} className="launcher-card">
                  <div className="launcher-card-top">
                    <strong>{args?.name || t("launcher.serverFallback")}</strong>
                    <span
                      className={
                        info ? (info.online ? "launcher-dot online" : "launcher-dot off") : "launcher-dot"
                      }
                    />
                  </div>
                  <p>
                    {info?.online
                      ? t("launcher.slotsFree", {
                          track: info.track || "—",
                          free: info.free,
                          max: info.max,
                        })
                      : info
                        ? t("launcher.unreachable")
                        : t("launcher.checking")}
                  </p>
                  <div className="row gap">
                    <Button
                      variant="primary"
                      disabled={!args}
                      onClick={() => args && setSelected(args)}
                    >
                      {t("launcher.enter")}
                    </Button>
                    <Button
                      variant="ghost"
                      onClick={() => void api.removeInvite(invite.link).then(setState)}
                    >
                      {t("launcher.remove")}
                    </Button>
                  </div>
                </article>
              );
            })}
          </div>
        </section>

        <div className="launcher-side">
          <section className="launcher-add">
            <h2>{t("launcher.addTitle")}</h2>
            <p className="muted">
              {t("launcher.addHelpBefore")}
              <code>acservercontrol://…</code>
              {t("launcher.addHelpAfter")}
            </p>
            <input
              value={input}
              placeholder="acservercontrol://join?ip=…"
              onChange={(event) => setInput(event.target.value)}
            />
            <div className="row gap">
              <Button variant="primary" disabled={!input.trim()} onClick={() => void add()}>
                {t("launcher.add")}
              </Button>
            </div>
            {error && <p className="launcher-error">{error}</p>}
          </section>

        </div>

        <section className="launcher-add launcher-assists">
          <h2>{t("launcher.assistsTitle")}</h2>
          <p className="muted">{t("launcher.assistsHelp")}</p>
          <AssistsPanel />
        </section>
      </main>
    </div>
  );
}
