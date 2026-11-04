import { useEffect, useMemo, useState } from "react";
import { api } from "./api";
import { AssistsPanel } from "./AssistsPanel";
import { Thumb } from "./components/Thumb";
import { Button } from "./components/ui";
import { useT } from "./i18n/core";
import type { ContentLibrary, InviteServerInfo, JoinArgs } from "./types";

type Phase = "loading" | "error" | "form" | "done";

function accentStyle(accent: string): React.CSSProperties {
  return { ["--join-accent" as string]: accent || "#ff5a1f" };
}

export function JoinWindow({
  initialArgs,
  initialDriver = "",
  onBack,
}: {
  initialArgs?: JoinArgs;
  initialDriver?: string;
  onBack?: () => void;
}) {
  const { t } = useT();
  const [args, setArgs] = useState<JoinArgs | null>(initialArgs ?? null);
  const [library, setLibrary] = useState<ContentLibrary | null>(null);
  const [info, setInfo] = useState<InviteServerInfo | null>(null);
  const [phase, setPhase] = useState<Phase>("loading");
  const [error, setError] = useState("");
  const [driver, setDriver] = useState(initialDriver);
  const [car, setCar] = useState("");
  const [skin, setSkin] = useState("");
  const [busy, setBusy] = useState(false);
  const [notice, setNotice] = useState("");
  const [guid, setGuid] = useState("");
  const [reload, setReload] = useState(0);

  useEffect(() => {
    api
      .steamGuid()
      .then((value) => setGuid(value ?? ""))
      .catch(() => undefined);
  }, []);

  useEffect(() => {
    let cancelled = false;
    setPhase("loading");
    setError("");
    void (async () => {
      try {
        const context = initialArgs ?? (await api.joinContext());
        if (!context) throw new Error(t("join.noInvite"));
        if (cancelled) return;
        setArgs(context);
        document.title = t("join.documentTitle", { name: context.name || "AC Server" });

        api
          .launcherLibrary()
          .then((value) => {
            if (!cancelled) setLibrary(value);
          })
          .catch(() => undefined);

        const payload = await api.remoteServerInfo(
          context.ip,
          context.api_port,
          context.token,
        );
        if (cancelled) return;
        setInfo(payload);
        setCar(payload.cars[0]?.id ?? "");
        setSkin(payload.cars[0]?.skins[0] ?? "");
        setPhase("form");
      } catch (problem) {
        if (cancelled) return;
        setError(String(problem));
        setPhase("error");
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [reload]);

  const retry = () => {
    setError("");
    setNotice("");
    setPhase("loading");
    setReload((value) => value + 1);
  };

  const localCar = useMemo(
    () => library?.cars.find((item) => item.id === car),
    [library, car],
  );

  const skins = useMemo(() => {
    const declared = info?.cars.find((item) => item.id === car)?.skins ?? [];
    if (localCar) {
      return localCar.skins
        .filter((item) => declared.length === 0 || declared.includes(item.name))
        .map((item) => ({
          name: item.name,
          display: item.display || item.name,
          preview: item.preview,
          local: true,
        }));
    }
    return declared.map((name) => ({ name, display: name, preview: null, local: false }));
  }, [info, localCar, car]);

  const enter = async (skipReservation = false) => {
    if (!args) return;
    if (!skipReservation && !driver.trim()) {
      setNotice(t("join.needName"));
      return;
    }
    setBusy(true);
    setNotice("");
    try {
      if (!skipReservation) {
        const result = await api.remoteReserve(
          args.ip,
          args.api_port,
          args.token,
          driver.trim(),
          car,
          skin,
          guid,
        );
        if (!result.ok) {
          setNotice(result.error || t("join.reserveFailed"));
          setBusy(false);
          return;
        }
        if (info?.running) {
          setNotice(t("join.reserved"));
        }
        await api.setDriverName(driver.trim());
      }
      await api.launchGame(
        args.ip,
        info?.port ?? 9600,
        args.http_port,
        driver.trim(),
        car,
        skin,
        info?.name || args.name || t("join.defaultEventName"),
      );
      setPhase("done");
    } catch (problem) {
      setNotice(String(problem));
    } finally {
      setBusy(false);
    }
  };

  if (phase === "loading") {
    return (
      <div className="join-page">
        <div className="join-loading">{t("join.connecting")}</div>
      </div>
    );
  }

  if (phase === "error" || !args) {
    const unreachable =
      error.toLowerCase().includes("non raggiungibile") ||
      error.toLowerCase().includes("unreachable");
    return (
      <div className="join-page">
        <div className="join-card join-error">
          <h1>{unreachable ? t("join.unreachableTitle") : t("join.invalidTitle")}</h1>
          <p>{error || t("join.invalidText")}</p>
          {unreachable && <p className="muted">{t("join.unreachableHelp")}</p>}
          <div className="row gap">
            <Button variant="primary" onClick={retry}>
              {t("join.retry")}
            </Button>
            {onBack && (
              <Button variant="ghost" onClick={onBack}>
                {t("join.backToServers")}
              </Button>
            )}
          </div>
        </div>
      </div>
    );
  }

  if (phase === "done") {
    const joinLink = `acmanager://race/online/join?ip=${encodeURIComponent(args.ip)}&httpPort=${args.http_port}`;
    return (
      <div className="join-page" style={accentStyle(args.accent)}>
        <div className="join-card">
          <h1>{t("join.doneTitle")}</h1>
          <p>{t("join.doneText")}</p>
          {notice && <p className="join-notice">{notice}</p>}
          <div className="row gap">
            <Button onClick={() => setPhase("form")}>{t("join.back")}</Button>
            <Button
              variant="ghost"
              onClick={() => void api.openUrl(joinLink)}
              title={t("join.openInCmHint")}
            >
              {t("join.openInCm")}
            </Button>
            {onBack && (
              <Button variant="ghost" onClick={onBack}>
                {t("join.backToServers")}
              </Button>
            )}
          </div>
        </div>
      </div>
    );
  }

  return (
    <div className="join-page" style={accentStyle(args.accent)}>
      <header
        className="join-hero"
        style={
          args.banner
            ? {
                backgroundImage: `linear-gradient(180deg, rgba(6,9,12,.35), rgba(6,9,12,.92)), url("${args.banner}")`,
              }
            : undefined
        }
      >
        <div className="join-hero-text">
          <span className="join-kicker">{args.name || t("join.kickerFallback")}</span>
          <h1>{info?.name || t("join.serverFallback")}</h1>
          <p>
            {t("join.trackLabel")}
            <strong>{info?.track || "—"}</strong>
            {t("join.freeSlotsLabel")}
            <strong>{info?.free_slots ?? "—"}</strong>
            {info ? ` / ${info.max_clients}` : ""}
          </p>
        </div>
      </header>

      <main className="join-body">
        <section className="join-cars">
          <div className="join-section-head">
            <h2>{t("join.carsTitle")}</h2>
            <span className="muted">{t("join.carsSubtitle")}</span>
          </div>
          <div className="join-car-grid">
            {(info?.cars ?? []).map((item) => {
              const preview = library?.cars.find((entry) => entry.id === item.id)?.preview;
              return (
                <button
                  key={item.id}
                  className={item.id === car ? "join-car active" : "join-car"}
                  onClick={() => {
                    setCar(item.id);
                    setSkin(item.skins[0] ?? "");
                  }}
                >
                  <Thumb src={preview} alt={item.name} fallback={item.name} width={320} />
                  <strong>{item.name}</strong>
                  <span>{t("join.skinsCount", { n: item.skins.length })}</span>
                </button>
              );
            })}
            {(info?.cars ?? []).length === 0 && (
              <div className="muted">{t("join.noCars")}</div>
            )}
          </div>
        </section>

        <aside className="join-side">
          <h2>{t("join.skinTitle")}</h2>
          <p className="join-skins-note">
            {skins.length === 0
              ? ""
              : skins[0].local
                ? t("join.skinsLocal", { n: skins.length })
                : t("join.skinsRemote", { n: skins.length })}
          </p>
          <div className="join-skins">
            {skins.map((item) => (
              <button
                key={item.name}
                className={item.name === skin ? "join-skin active" : "join-skin"}
                onClick={() => setSkin(item.name)}
                title={item.local ? item.name : t("join.skinRemoteHint", { name: item.name })}
              >
                <Thumb src={item.preview} alt={item.display} fallback={item.name} width={220} />
                <span>{item.display}</span>
                {!item.local && <em className="join-skin-warn">{t("join.skinFromServer")}</em>}
              </button>
            ))}
            {skins.length === 0 && <div className="muted">{t("join.noSkins")}</div>}
          </div>

          <label className="join-field">
            <span>{t("join.yourName")}</span>
            <input
              value={driver}
              placeholder={t("join.driverPlaceholder")}
              onChange={(event) => setDriver(event.target.value)}
            />
          </label>

          <details className="join-assists">
            <summary>{t("join.assistsTitle")}</summary>
            <AssistsPanel compact />
          </details>

          {notice && <p className="join-notice">{notice}</p>}

          <Button variant="primary" disabled={busy || !car || !driver.trim()} onClick={() => void enter()}>
            {busy ? t("join.reserving") : t("join.enter")}
          </Button>
          {notice && (
            <Button variant="ghost" onClick={() => void enter(true)}>
              {t("join.enterAnyway")}
            </Button>
          )}

          <div className="row gap">
            <Button variant="ghost" onClick={retry}>
              {t("join.refresh")}
            </Button>
            {onBack && (
              <Button variant="ghost" onClick={onBack}>
                {t("join.backToServers")}
              </Button>
            )}
          </div>
        </aside>
      </main>
    </div>
  );
}
