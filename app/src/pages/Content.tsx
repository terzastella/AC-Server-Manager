import { useEffect, useMemo, useState } from "react";
import { api, openPath } from "../api";
import { useApp } from "../store";
import { Badge, Button, Card, Empty, Stat } from "../components/ui";
import { Thumb } from "../components/Thumb";
import { useT } from "../i18n/core";
import type { CarEntry, ContentLibrary, TrackEntry } from "../types";

type Kind = "cars" | "tracks";
type Source = "all" | "server" | "game";
type Selection = { kind: "car"; car: CarEntry } | { kind: "track"; track: TrackEntry } | null;

export function Content() {
  const { t } = useT();
  const { settings, info, run, refreshContent, notify } = useApp();
  const [library, setLibrary] = useState<ContentLibrary | null>(null);
  const [kind, setKind] = useState<Kind>("cars");
  const [source, setSource] = useState<Source>("all");
  const [search, setSearch] = useState("");
  const [onlyUsed, setOnlyUsed] = useState(false);
  const [selection, setSelection] = useState<Selection>(null);
  const [skin, setSkin] = useState<string | null>(null);
  const [gameDir, setGameDir] = useState("");
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const load = async () => {
    setLoading(true);
    setError(null);
    try {
      const next = await api.contentLibrary();
      setLibrary(next);
      setGameDir(next.game_dir ?? settings?.game_dir ?? "");
    } catch (failure) {
      setError(String(failure));
      notify(String(failure), "error");
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    void load();
  }, []);

  useEffect(() => {
    if (selection?.kind === "car") {
      setSkin(selection.car.skins[0]?.preview ?? null);
    }
  }, [selection]);

  const query = search.trim().toLowerCase();

  const cars = useMemo(() => {
    return (library?.cars ?? [])
      .filter((car) => {
        if (onlyUsed && !car.used) return false;
        if (source === "server" && !car.in_server) return false;
        if (source === "game" && !car.in_game) return false;
        if (!query) return true;
        return (
          car.name.toLowerCase().includes(query) ||
          car.brand.toLowerCase().includes(query) ||
          car.id.toLowerCase().includes(query) ||
          car.tags.some((tag) => tag.toLowerCase().includes(query))
        );
      })
      .sort((a, b) => Number(b.used) - Number(a.used) || a.name.localeCompare(b.name));
  }, [library, query, onlyUsed, source]);

  const tracks = useMemo(() => {
    return (library?.tracks ?? [])
      .filter((track) => {
        if (onlyUsed && !track.used) return false;
        if (source === "server" && !track.in_server) return false;
        if (source === "game" && !track.in_game) return false;
        if (!query) return true;
        return (
          track.name.toLowerCase().includes(query) ||
          track.country.toLowerCase().includes(query) ||
          track.id.toLowerCase().includes(query)
        );
      })
      .sort((a, b) => Number(b.used) - Number(a.used) || a.name.localeCompare(b.name));
  }, [library, query, onlyUsed, source]);

  const selectedCar = selection?.kind === "car" ? selection.car : null;
  const selectedTrack = selection?.kind === "track" ? selection.track : null;
  const heroImage =
    selectedCar?.skins.find((item) => item.preview === skin)?.preview ??
    selectedCar?.preview ??
    selectedCar?.badge ??
    null;

  const copyFromGame = () =>
    void run("copy", async () => {
      if (!gameDir) return;
      const report = await api.copyContent(gameDir);
      await load();
      await refreshContent();
      const message = [
        report.copied.length ? t("content.copied", { n: report.copied.length }) : "",
        report.skipped.length ? t("content.alreadyPresent", { n: report.skipped.length }) : "",
        report.errors.length ? t("content.errors", { n: report.errors.length }) : "",
      ]
        .filter(Boolean)
        .join(" · ");
      notify(message || t("content.noOperation"), report.errors.length ? "error" : "success");
    });

  return (
    <div className="page">
      <div className="grid grid-4">
        <Stat label={t("content.cars")} value={library?.cars.length ?? "—"} hint={t("content.onServerHint", { n: library?.cars.filter((car) => car.in_server).length ?? 0 })} />
        <Stat label={t("content.tracks")} value={library?.tracks.length ?? "—"} hint={t("content.onServerHint", { n: library?.tracks.filter((track) => track.in_server).length ?? 0 })} />
        <Stat label={t("content.inUse")} value={(library?.cars.filter((car) => car.used).length ?? 0) + (library?.tracks.filter((track) => track.used).length ?? 0)} hint={t("content.fromServerCfg")} />
        <Stat label={t("content.gameFolder")} value={library?.game_dir ? t("content.linked") : t("content.notFound")} tone={library?.game_dir ? "green" : "red"} />
      </div>

      {error && (
        <div className="banner banner-red">
          <strong>{t("content.scanError")}</strong>
          <span>{error}</span>
        </div>
      )}
      {loading && !library && (
        <div className="banner banner-blue">
          <strong>{t("content.scanning")}</strong>
          <span>{t("content.scanningHint")}</span>
        </div>
      )}

      <Card
        title={t("content.copyFromGame")}
        subtitle={library?.game_dir ? library.game_dir : t("content.setAcFolder")}
        actions={<Button variant="ghost" onClick={() => void load()}>{t("content.rescan")}</Button>}
      >
        <div className="row gap">
          <input
            className="grow mono"
            placeholder="C:\\Program Files (x86)\\Steam\\steamapps\\common\\assettocorsa"
            value={gameDir}
            onChange={(event) => setGameDir(event.target.value)}
          />
          <Button variant="primary" disabled={!gameDir} onClick={copyFromGame} busy={false}>
            {t("content.copyUsed")}
          </Button>
          <Button variant="ghost" disabled={!gameDir} onClick={() => void openPath(gameDir)}>{t("content.open")}</Button>
        </div>
        <p className="muted small">
          {t("content.copyHelp")}
        </p>
      </Card>

      <div className="content-toolbar">
        <div className="segmented">
          <button className={kind === "cars" ? "active" : ""} onClick={() => setKind("cars")}>
            {t("content.cars")} <em>{cars.length}</em>
          </button>
          <button className={kind === "tracks" ? "active" : ""} onClick={() => setKind("tracks")}>
            {t("content.tracks")} <em>{tracks.length}</em>
          </button>
        </div>
        <input
          className="search"
          placeholder={t("content.searchPlaceholder")}
          value={search}
          onChange={(event) => setSearch(event.target.value)}
        />
        <select value={source} onChange={(event) => setSource(event.target.value as Source)}>
          <option value="all">{t("content.sourceAll")}</option>
          <option value="server">{t("content.sourceServer")}</option>
          <option value="game">{t("content.sourceGame")}</option>
        </select>
        <button className={onlyUsed ? "filter-chip active" : "filter-chip"} onClick={() => setOnlyUsed(!onlyUsed)}>
          {t("content.onlyUsed")}
        </button>
      </div>

      {kind === "cars" && (
        <div className="card-grid">
          {cars.map((car) => (
            <button className="content-card" key={car.id} onClick={() => setSelection({ kind: "car", car })}>
              <Thumb src={car.preview ?? car.badge} alt={car.name} fallback={car.brand || car.id} />
              <div className="content-card-body">
                <strong>{car.name}</strong>
                <span>{car.brand || car.id}</span>
                <div className="content-card-badges">
                  {car.used && <Badge tone="green">in CARS=</Badge>}
                  {!car.in_server && <Badge tone="amber">{t("content.gameOnly")}</Badge>}
                  {car.in_server && !car.in_game && <Badge tone="blue">{t("content.serverOnly")}</Badge>}
                  <Badge tone="muted">{t("content.skins", { n: car.skins.length })}</Badge>
                </div>
              </div>
            </button>
          ))}
          {cars.length === 0 && <Empty>{t("content.noCars")}</Empty>}
        </div>
      )}

      {kind === "tracks" && (
        <div className="card-grid">
          {tracks.map((track) => (
            <button className="content-card" key={track.id} onClick={() => setSelection({ kind: "track", track })}>
              <Thumb src={track.preview} alt={track.name} fallback={track.country || track.id} />
              <div className="content-card-body">
                <strong>{track.name}</strong>
                <span>{[track.city, track.country].filter(Boolean).join(", ") || track.id}</span>
                <div className="content-card-badges">
                  {track.used && <Badge tone="green">in TRACK=</Badge>}
                  {!track.in_server && <Badge tone="amber">{t("content.gameOnly")}</Badge>}
                  {track.in_server && !track.in_game && <Badge tone="blue">{t("content.serverOnly")}</Badge>}
                  {track.length && <Badge tone="muted">{t("content.meters", { n: track.length })}</Badge>}
                </div>
              </div>
            </button>
          ))}
          {tracks.length === 0 && <Empty>{t("content.noTracks")}</Empty>}
        </div>
      )}

      {selection && (
        <div className="detail-overlay" onClick={() => setSelection(null)}>
          <div className="detail-panel" onClick={(event) => event.stopPropagation()}>
            <button className="detail-close" onClick={() => setSelection(null)}>✕</button>

            {selectedCar && (
              <>
                <div className="detail-hero">
                  <Thumb src={heroImage} alt={selectedCar.name} fallback={selectedCar.brand || selectedCar.id} className="detail-image" width={1200} />
                  {selectedCar.skins.length > 0 && (
                    <div className="skin-strip">
                      {selectedCar.skins.map((entry) => (
                        <button
                          key={entry.name}
                          className={entry.preview === skin ? "skin active" : "skin"}
                          onClick={() => setSkin(entry.preview)}
                          title={entry.display}
                        >
                          <Thumb src={entry.preview} alt={entry.display} fallback={entry.name} width={200} />
                        </button>
                      ))}
                    </div>
                  )}
                </div>
                <div className="detail-info">
                  <div className="detail-title">
                    <h2>{selectedCar.name}</h2>
                    <span>{selectedCar.brand} · {selectedCar.class || selectedCar.id}</span>
                  </div>
                  <div className="chip-row">
                    {selectedCar.tags.map((tag) => <span className="chip" key={tag}>{tag}</span>)}
                  </div>
                  {Object.keys(selectedCar.specs ?? {}).length > 0 && (
                    <div className="spec-grid">
                      {Object.entries(selectedCar.specs).map(([key, value]) => (
                        <div key={key}><span>{key}</span><b>{String(value)}</b></div>
                      ))}
                    </div>
                  )}
                  <div className="detail-flags">
                    {selectedCar.used && <Badge tone="green">in CARS=</Badge>}
                    {selectedCar.in_server ? <Badge tone="blue">{t("content.onServerBadge")}</Badge> : <Badge tone="amber">{t("content.inGameOnly")}</Badge>}
                    {selectedCar.in_game && <Badge tone="muted">{t("content.installedInGame")}</Badge>}
                    <Badge tone="muted">{t("content.skins", { n: selectedCar.skins.length })}</Badge>
                  </div>
                  <div className="row gap top-gap">
                    <Button variant="ghost" onClick={() => void openPath(selectedCar.server_path ?? selectedCar.game_path ?? "")}>
                      {t("content.openFolder")}
                    </Button>
                    <code className="path-box grow">{selectedCar.server_path ?? selectedCar.game_path ?? "—"}</code>
                  </div>
                </div>
              </>
            )}

            {selectedTrack && (
              <>
                <div className="detail-hero">
                  <Thumb src={selectedTrack.preview} alt={selectedTrack.name} fallback={selectedTrack.country || selectedTrack.id} className="detail-image" width={1200} />
                  {selectedTrack.outline && (
                    <div className="track-outline">
                      <Thumb src={selectedTrack.outline} alt={t("content.layout")} className="outline-thumb" width={420} />
                    </div>
                  )}
                </div>
                <div className="detail-info">
                  <div className="detail-title">
                    <h2>{selectedTrack.name}</h2>
                    <span>{[selectedTrack.city, selectedTrack.country].filter(Boolean).join(", ") || selectedTrack.id}</span>
                  </div>
                  <div className="chip-row">
                    {selectedTrack.tags.map((tag) => <span className="chip" key={tag}>{tag}</span>)}
                  </div>
                  <div className="spec-grid">
                    {selectedTrack.length && <div><span>{t("content.length")}</span><b>{t("content.meters", { n: selectedTrack.length })}</b></div>}
                    {selectedTrack.country && <div><span>{t("content.country")}</span><b>{selectedTrack.country}</b></div>}
                    {selectedTrack.city && <div><span>{t("content.city")}</span><b>{selectedTrack.city}</b></div>}
                  </div>
                  <div className="detail-flags">
                    {selectedTrack.used && <Badge tone="green">in TRACK=</Badge>}
                    {selectedTrack.in_server ? <Badge tone="blue">{t("content.onServerBadge")}</Badge> : <Badge tone="amber">{t("content.inGameOnly")}</Badge>}
                  </div>
                  <div className="row gap top-gap">
                    <Button variant="ghost" onClick={() => void openPath(selectedTrack.server_path ?? selectedTrack.game_path ?? "")}>
                      {t("content.openFolder")}
                    </Button>
                    <code className="path-box grow">{selectedTrack.server_path ?? selectedTrack.game_path ?? "—"}</code>
                  </div>
                </div>
              </>
            )}
          </div>
        </div>
      )}
    </div>
  );
}
