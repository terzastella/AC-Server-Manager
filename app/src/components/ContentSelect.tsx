import { useEffect, useMemo, useState } from "react";
import { Badge, Button, Empty } from "./ui";
import { Thumb } from "./Thumb";
import { useT } from "../i18n/core";
import type { CarEntry, ContentLibrary } from "../types";

type Source = "all" | "server" | "game";

function splitIds(value: string): string[] {
  return value
    .split(";")
    .map((item) => item.trim())
    .filter(Boolean);
}

export function CarsField({
  value,
  onChange,
  library,
  onOpen,
}: {
  value: string;
  onChange: (value: string) => void;
  library: ContentLibrary | null;
  onOpen: () => void;
}) {
  const { t } = useT();
  const ids = splitIds(value);

  return (
    <div className="select-field">
      <div className="select-chips">
        {ids.length === 0 && <span className="muted small">{t("contentSelect.noCarsSelected")}</span>}
        {ids.map((id) => {
          const car = library?.cars.find((item) => item.id.toLowerCase() === id.toLowerCase());
          return (
            <div className="select-chip" key={id}>
              <Thumb src={car?.preview ?? car?.badge} alt={id} fallback={id} width={160} />
              <div className="select-chip-text">
                <strong>{car?.name || id}</strong>
                <span>{id}</span>
              </div>
              {car && !car.in_server && <Badge tone="amber">{t("contentSelect.gameBadge")}</Badge>}
              <button
                className="select-chip-x"
                title={t("contentSelect.remove")}
                onClick={() => onChange(ids.filter((item) => item !== id).join(";"))}
              >
                ✕
              </button>
            </div>
          );
        })}
      </div>
      <div className="row gap">
        <Button onClick={onOpen} disabled={!library}>
          {library ? t("contentSelect.chooseCars") : t("contentSelect.loadingContent")}
        </Button>
        <span className="muted small">{t("contentSelect.selectedCount", { n: ids.length })}</span>
      </div>
      <details className="manual">
        <summary>{t("contentSelect.manualEntry")}</summary>
        <input value={value} spellCheck={false} onChange={(event) => onChange(event.target.value)} />
      </details>
    </div>
  );
}

export function TrackField({
  value,
  onChange,
  library,
  onOpen,
}: {
  value: string;
  onChange: (value: string) => void;
  library: ContentLibrary | null;
  onOpen: () => void;
}) {
  const { t } = useT();
  const track = library?.tracks.find((item) => item.id.toLowerCase() === value.toLowerCase());

  return (
    <div className="select-field">
      <div className="select-track">
        <Thumb src={track?.preview} alt={value || t("contentSelect.trackAlt")} fallback={value || "—"} width={480} />
        <div className="select-chip-text">
          <strong>{track?.name || value || t("contentSelect.noTrackSelected")}</strong>
          <span>{track ? [track.city, track.country].filter(Boolean).join(", ") : value}</span>
          {track && !track.in_server && <Badge tone="amber">{t("contentSelect.gameOnly")}</Badge>}
        </div>
        <Button onClick={onOpen} disabled={!library}>
          {library ? t("contentSelect.chooseTrack") : t("contentSelect.loadingContent")}
        </Button>
      </div>
      <details className="manual">
        <summary>{t("contentSelect.manualEntry")}</summary>
        <input value={value} spellCheck={false} onChange={(event) => onChange(event.target.value)} />
      </details>
    </div>
  );
}

export function SkinPicker({
  car,
  current,
  onClose,
  onApply,
}: {
  car: CarEntry;
  current: string;
  onClose: () => void;
  onApply: (skin: string) => void;
}) {
  const { t } = useT();
  const [selected, setSelected] = useState(current);
  const [search, setSearch] = useState("");

  const skins = useMemo(() => {
    const query = search.trim().toLowerCase();
    return car.skins.filter((skin) => {
      if (!query) return true;
      return `${skin.display} ${skin.name} ${skin.team} ${skin.country} ${skin.number}`
        .toLowerCase()
        .includes(query);
    });
  }, [car, search]);

  const PAGE_SIZE = 36;
  const [page, setPage] = useState(0);
  useEffect(() => {
    setPage(0);
  }, [search, car.id]);

  const pageCount = Math.max(1, Math.ceil(skins.length / PAGE_SIZE));
  const safePage = Math.min(page, pageCount - 1);
  const pageSkins = skins.slice(safePage * PAGE_SIZE, safePage * PAGE_SIZE + PAGE_SIZE);

  return (
    <div className="detail-overlay" onClick={onClose}>
      <div className="picker-panel" onClick={(event) => event.stopPropagation()}>
        <header className="picker-head">
          <div>
            <h2>{t("contentSelect.skinsTitle", { car: car.name })}</h2>
            <p>{t("contentSelect.skinsSubtitle", { n: car.skins.length })}</p>
          </div>
          <button className="detail-close" onClick={onClose}>✕</button>
        </header>

        <div className="picker-toolbar">
          <input
            className="search"
            placeholder={t("contentSelect.searchSkinPlaceholder")}
            value={search}
            onChange={(event) => setSearch(event.target.value)}
            autoFocus
          />
          <Badge tone="muted">{skins.length}</Badge>
        </div>

        <div className="picker-grid">
          {pageSkins.map((skin) => {
            const active = selected === skin.name;
            return (
              <button
                key={skin.name}
                className={active ? "picker-card active" : "picker-card"}
                onClick={() => setSelected(skin.name)}
                title={skin.display}
              >
                <Thumb src={skin.preview} alt={skin.display} fallback={skin.name} width={320} eager />
                <div className="picker-card-text">
                  <strong>{skin.display}</strong>
                  <span>{skin.name}</span>
                </div>
                <div className="picker-card-badges">
                  {skin.number && <Badge tone="accent">#{skin.number}</Badge>}
                  {skin.team && <Badge tone="muted">{skin.team}</Badge>}
                  {skin.country && <Badge tone="muted">{skin.country}</Badge>}
                </div>
                {active && <span className="picker-check">✓</span>}
              </button>
            );
          })}
          {skins.length === 0 && <Empty>{t("contentSelect.noSkins")}</Empty>}
        </div>

        <footer className="picker-foot">
          <span>{selected ? t("contentSelect.selectedSkin", { name: selected }) : t("contentSelect.noSkin")}</span>
          <div className="picker-pager">
            <Button variant="ghost" disabled={safePage === 0} onClick={() => setPage(safePage - 1)}>‹</Button>
            <span>{t("contentSelect.page", { page: safePage + 1, pages: pageCount })}</span>
            <Button variant="ghost" disabled={safePage >= pageCount - 1} onClick={() => setPage(safePage + 1)}>›</Button>
          </div>
          <div className="row gap">
            <Button variant="ghost" onClick={onClose}>{t("contentSelect.cancel")}</Button>
            <Button variant="primary" disabled={!selected} onClick={() => onApply(selected)}>
              {t("contentSelect.applySkin")}
            </Button>
          </div>
        </footer>
      </div>
    </div>
  );
}

export function ContentPicker({
  kind,
  library,
  initial,
  onClose,
  onApply,
  multiple: multipleOverride,
}: {
  kind: "cars" | "tracks";
  library: ContentLibrary;
  initial: string[];
  onClose: () => void;
  onApply: (ids: string[]) => void;
  multiple?: boolean;
}) {
  const [selected, setSelected] = useState<string[]>(initial);
  const { t } = useT();
  const [search, setSearch] = useState("");
  const [source, setSource] = useState<Source>("all");
  const multiple = multipleOverride ?? kind === "cars";

  const items = useMemo(() => {
    const query = search.trim().toLowerCase();
    const list = kind === "cars" ? library.cars : library.tracks;
    return list
      .filter((item) => {
        if (source === "server" && !item.in_server) return false;
        if (source === "game" && !item.in_game) return false;
        if (!query) return true;
        const haystack = `${item.name} ${item.id} ${"brand" in item ? item.brand : item.country}`.toLowerCase();
        return haystack.includes(query);
      })
      .sort((a, b) => a.name.localeCompare(b.name));
  }, [kind, library, search, source]);

  const PAGE_SIZE = 48;
  const [page, setPage] = useState(0);
  useEffect(() => {
    setPage(0);
  }, [search, source, kind]);

  const pageCount = Math.max(1, Math.ceil(items.length / PAGE_SIZE));
  const safePage = Math.min(page, pageCount - 1);
  const pageItems = items.slice(safePage * PAGE_SIZE, safePage * PAGE_SIZE + PAGE_SIZE);

  const toggle = (id: string) => {
    setSelected((current) => {
      if (!multiple) return current[0] === id ? [] : [id];
      return current.includes(id)
        ? current.filter((item) => item !== id)
        : [...current, id];
    });
  };

  const missingOnServer = selected.filter((id) => {
    const item =
      kind === "cars"
        ? library.cars.find((car) => car.id === id)
        : library.tracks.find((track) => track.id === id);
    return item ? !item.in_server : false;
  });

  return (
    <div className="detail-overlay" onClick={onClose}>
      <div className="picker-panel" onClick={(event) => event.stopPropagation()}>
        <header className="picker-head">
          <div>
            <h2>
              {multiple
                ? t("contentSelect.chooseCarsTitle")
                : kind === "tracks"
                  ? t("contentSelect.chooseTrackTitle")
                  : t("contentSelect.chooseCarTitle")}
            </h2>
            <p>
              {multiple
                ? t("contentSelect.multiSubtitle")
                : kind === "tracks"
                  ? t("contentSelect.trackSubtitle")
                  : t("contentSelect.modelSubtitle")}
            </p>
          </div>
          <button className="detail-close" onClick={onClose}>✕</button>
        </header>

        <div className="picker-toolbar">
          <input
            className="search"
            placeholder={t("contentSelect.searchPlaceholder")}
            value={search}
            onChange={(event) => setSearch(event.target.value)}
            autoFocus
          />
          <select value={source} onChange={(event) => setSource(event.target.value as Source)}>
            <option value="all">{t("contentSelect.sourceAll")}</option>
            <option value="server">{t("contentSelect.sourceServer")}</option>
            <option value="game">{t("contentSelect.sourceGame")}</option>
          </select>
          <Badge tone="muted">{items.length}</Badge>
        </div>

        {missingOnServer.length > 0 && (
          <div className="banner banner-amber picker-warning">
            <strong>{t("contentSelect.missingTitle")}</strong>
            <span>{t("contentSelect.missingText", { ids: missingOnServer.join(", ") })}</span>
          </div>
        )}

        <div className="picker-grid">
          {pageItems.map((item) => {
            const id = item.id;
            const active = selected.includes(id);
            return (
              <button
                key={id}
                className={active ? "picker-card active" : "picker-card"}
                onClick={() => toggle(id)}
                title={item.name}
              >
                <Thumb src={item.preview} alt={item.name} fallback={item.name} width={320} eager />
                <div className="picker-card-text">
                  <strong>{item.name}</strong>
                  <span>{"brand" in item ? item.brand || id : item.country || id}</span>
                </div>
                <div className="picker-card-badges">
                  {item.used && <Badge tone="green">{t("contentSelect.inUse")}</Badge>}
                  {!item.in_server && <Badge tone="amber">{t("contentSelect.gameBadge")}</Badge>}
                </div>
                {active && <span className="picker-check">✓</span>}
              </button>
            );
          })}
          {items.length === 0 && <Empty>{t("contentSelect.noResults")}</Empty>}
        </div>

        <footer className="picker-foot">
          <span>{t("contentSelect.selectedCountPlural", { n: selected.length })}</span>
          <div className="picker-pager">
            <Button variant="ghost" disabled={safePage === 0} onClick={() => setPage(safePage - 1)}>{t("contentSelect.prev")}</Button>
            <span>{t("contentSelect.page", { page: safePage + 1, pages: pageCount })}</span>
            <Button variant="ghost" disabled={safePage >= pageCount - 1} onClick={() => setPage(safePage + 1)}>{t("contentSelect.next")}</Button>
          </div>
          <div className="row gap">
            <Button variant="ghost" onClick={() => setSelected([])}>{t("contentSelect.clear")}</Button>
            <Button variant="ghost" onClick={onClose}>{t("contentSelect.cancel")}</Button>
            <Button variant="primary" disabled={selected.length === 0} onClick={() => onApply(selected)}>
              {t("contentSelect.apply")}
            </Button>
          </div>
        </footer>
      </div>
    </div>
  );
}
