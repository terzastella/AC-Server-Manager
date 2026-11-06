import { useEffect, useMemo, useRef, useState } from "react";
import { api } from "../api";
import { useApp } from "../store";
import { Badge, Button, Card, Empty } from "../components/ui";
import { Thumb } from "../components/Thumb";
import { ContentPicker, SkinPicker } from "../components/ContentSelect";
import { useT } from "../i18n/core";
import type { CarSlot, ContentLibrary } from "../types";
import { num } from "../utils";

const EMPTY_SLOT: CarSlot = {
  driver_name: "",
  team: "",
  model: "",
  skin: "",
  spectator_mode: 0,
  guid: "",
  ballast: 0,
  restrictor: 0,
};

export function Slots() {
  const { t } = useT();
  const { slots, slotsRevision, config, content, settings, refreshSlots, refreshContent, refreshConfig, run, notify } = useApp();
  const [draft, setDraft] = useState<CarSlot[]>([]);
  // ultima versione ricevuta dal server (non modificata dall'utente)
  const syncedRef = useRef<CarSlot[]>([]);
  const syncedRevisionRef = useRef(0);
  const draftRef = useRef<CarSlot[]>([]);
  const [pendingRemote, setPendingRemote] = useState(false);
  const [library, setLibrary] = useState<ContentLibrary | null>(null);
  const [modelPicker, setModelPicker] = useState<number | null>(null);
  const [skinPicker, setSkinPicker] = useState<number | null>(null);
  const [addCars, setAddCars] = useState(false);
  const [addTrack, setAddTrack] = useState(false);

  const sameSlots = (left: CarSlot[], right: CarSlot[]) =>
    JSON.stringify(left) === JSON.stringify(right);

  useEffect(() => {
    draftRef.current = draft;
  }, [draft]);

  useEffect(() => {
    const next = slots.map((slot) => ({ ...slot }));
    if (sameSlots(draftRef.current, syncedRef.current)) {
      syncedRef.current = next;
      syncedRevisionRef.current = slotsRevision;
      setDraft(next);
      setPendingRemote(false);
    } else if (!sameSlots(next, syncedRef.current)) {
      // prenotazione arrivata mentre stai modificando: non cancellare il draft
      setPendingRemote(true);
    }
  }, [slots, slotsRevision]);

  const reloadFromServer = () => {
    // scarta le modifiche locali e adotta quello che c'e' su disco
    syncedRef.current = draftRef.current;
    setPendingRemote(false);
    void refreshSlots();
  };

  useEffect(() => {
    void api.contentLibrary().then(setLibrary).catch(() => setLibrary(null));
  }, []);

  const carsFromConfig = (value("SERVER", "CARS") || "")
    .split(";")
    .map((item) => item.trim())
    .filter(Boolean);

  function value(section: string, key: string): string {
    return (
      config?.sections
        .find((item) => item.name === section)
        ?.entries.find((entry) => entry.key.toUpperCase() === key.toUpperCase())?.value ?? ""
    );
  }

  const maxClients = num(value("SERVER", "MAX_CLIENTS"), 0);
  const pickup = value("SERVER", "PICKUP_MODE_ENABLED") === "1";

  const knownModels = useMemo(
    () => new Set(carsFromConfig.map((item) => item.toLowerCase())),
    [carsFromConfig],
  );

  const carFor = (model: string) =>
    library?.cars.find((car) => car.id.toLowerCase() === model.toLowerCase());

  const skinsFor = (model: string) => carFor(model)?.skins ?? [];

  const update = (index: number, patch: Partial<CarSlot>) => {
    setDraft((current) =>
      current.map((slot, position) => (position === index ? { ...slot, ...patch } : slot)),
    );
  };

  const applyModel = (index: number, model: string) => {
    const skins = skinsFor(model);
    const current = draft[index]?.skin ?? "";
    const skin = skins.some((item) => item.name === current) ? current : skins[0]?.name ?? "";
    update(index, { model, skin });
  };

  const addSlot = () => {
    const model = carsFromConfig[0] ?? "";
    setDraft((current) => [
      ...current,
      { ...EMPTY_SLOT, model, skin: skinsFor(model)[0]?.name ?? "" },
    ]);
  };

  const duplicate = (index: number) => {
    setDraft((current) => {
      const copy = { ...current[index] };
      const next = [...current];
      next.splice(index + 1, 0, copy);
      return next;
    });
  };

  const remove = (index: number) => {
    setDraft((current) => current.filter((_, position) => position !== index));
  };

  const problems = useMemo(() => {
    const list: string[] = [];
    if (draft.length < maxClients) {
      list.push(t("slots.problemSlotCount", { n: draft.length, max: maxClients }));
    }
    const missingModels = draft
      .map((slot) => slot.model)
      .filter((model) => model && !knownModels.has(model.toLowerCase()));
    if (missingModels.length > 0) {
      list.push(
        t("slots.problemUnknownModels", {
          models: Array.from(new Set(missingModels)).join(", "),
        }),
      );
    }
    const absent = draft
      .map((slot) => slot.model)
      .filter(
        (model) =>
          model &&
          content &&
          !content.cars.some((car) => car.id.toLowerCase() === model.toLowerCase()),
      );
    if (absent.length > 0) {
      list.push(
        t("slots.problemMissingContent", { models: Array.from(new Set(absent)).join(", ") }),
      );
    }
    return list;
  }, [draft, maxClients, knownModels, content, t]);

  const skinCar =
    skinPicker !== null ? carFor(draft[skinPicker]?.model ?? "") ?? null : null;

  const gameDir = library?.game_dir ?? settings?.game_dir ?? "";

  const addCarsToServer = (ids: string[]) =>
    void run("add-cars", async () => {
      const selected = ids.map((id) => id.trim()).filter(Boolean);
      if (selected.length === 0) return;
      if (!config) throw new Error(t("slots.configMissing"));

      const currentCars = (value("SERVER", "CARS") || "")
        .split(";")
        .map((item) => item.trim())
        .filter(Boolean);
      const mergedCars = [...currentCars];
      for (const id of selected) {
        if (!mergedCars.some((item) => item.toLowerCase() === id.toLowerCase())) {
          mergedCars.push(id);
        }
      }

      const sections = config.sections.map((section) => {
        if (section.name !== "SERVER") return section;
        const hasCars = section.entries.some((entry) => entry.key.toUpperCase() === "CARS");
        const entries = hasCars
          ? section.entries.map((entry) =>
              entry.key.toUpperCase() === "CARS" ? { ...entry, value: mergedCars.join(";") } : entry,
            )
          : [...section.entries, { key: "CARS", value: mergedCars.join(";") }];
        return { ...section, entries };
      });
      await api.saveConfig(sections);
      await refreshConfig();

      const existingModels = new Set(draft.map((slot) => slot.model.toLowerCase()));
      const chosen = selected.filter((id) => !existingModels.has(id.toLowerCase()));
      const cap = maxClients > 0 ? maxClients : Number.POSITIVE_INFINITY;
      const nextSlots = [...draft];
      for (const id of chosen) {
        if (nextSlots.length >= cap) break;
        existingModels.add(id.toLowerCase());
        nextSlots.push({ ...EMPTY_SLOT, model: id, skin: skinsFor(id)[0]?.name ?? "" });
      }
      if (maxClients > 0 && nextSlots.length < maxClients && chosen.length > 0) {
        let index = 0;
        while (nextSlots.length < maxClients) {
          const id = chosen[index % chosen.length];
          nextSlots.push({ ...EMPTY_SLOT, model: id, skin: skinsFor(id)[0]?.name ?? "" });
          index += 1;
        }
      }
      const addedSlots = nextSlots.length - draft.length;
      if (addedSlots > 0) {
        await api.saveSlots(nextSlots, slotsRevision);
        syncedRef.current = nextSlots.map((slot) => ({ ...slot }));
        setPendingRemote(false);
        await refreshSlots();
      }

      let copied = 0;
      if (gameDir) {
        const report = await api.copyContent(gameDir);
        copied = report.copied.length;
        if (report.errors.length > 0) notify(report.errors.join(" · "), "error");
      }
      await refreshContent();

      const unchanged = addedSlots === 0 && maxClients > 0 && draft.length >= maxClients;
      notify(
        t(unchanged ? "slots.notifyAddCarsFull" : "slots.notifyAddCars", {
          cars: mergedCars.length,
          slots: addedSlots,
          copied,
        }),
        "success",
      );
    });

  const addTrackToServer = (ids: string[]) =>
    void run("add-track", async () => {
      const track = ids[0];
      if (!track) return;
      if (!config) throw new Error(t("slots.configMissing"));

      const sections = config.sections.map((section) => {
        if (section.name !== "SERVER") return section;
        const hasTrack = section.entries.some((entry) => entry.key.toUpperCase() === "TRACK");
        const entries = hasTrack
          ? section.entries.map((entry) =>
              entry.key.toUpperCase() === "TRACK" ? { ...entry, value: track } : entry,
            )
          : [...section.entries, { key: "TRACK", value: track }];
        return { ...section, entries };
      });
      await api.saveConfig(sections);
      await refreshConfig();

      let copied = 0;
      if (gameDir) {
        const report = await api.copyContent(gameDir);
        copied = report.copied.length;
        if (report.errors.length > 0) notify(report.errors.join(" · "), "error");
      }
      await refreshContent();
      notify(t("slots.notifyTrack", { track, copied }), "success");
    });

  return (
    <div className="page">
      <div className="banner banner-blue">
        <strong>
          {t("slots.mode", { mode: pickup ? t("slots.modePickup") : t("slots.modeEntryList") })}
        </strong>
        <span>{pickup ? t("slots.helpPickup") : t("slots.helpEntryList")}</span>
        {!library && <Badge tone="amber">{t("slots.loadingPreviews")}</Badge>}
      </div>

      {problems.length > 0 && (
        <div className="banner banner-amber">
          <strong>{t("slots.checks")}</strong>
          <span>{problems.join(" ")}</span>
        </div>
      )}

      <div className="slots-toolbar">
        <Button
          variant="primary"
          disabled={!library || !config}
          onClick={() => setAddCars(true)}
        >
          {t("slots.addCars")}
        </Button>
        <Button disabled={!library || !config} onClick={() => setAddTrack(true)}>
          {t("slots.addTrack")}
        </Button>
        <span className="muted slots-toolbar-hint">
          {t("slots.hintAddCars")}<code>CARS=</code>{t("slots.hintUpTo")}
          <code>MAX_CLIENTS</code>{t("slots.hintTrack")}<code>TRACK=</code>
          {t("slots.hintAfter")}
        </span>
      </div>

      {pendingRemote && (
        <div className="banner banner-amber">
          <strong>{t("slots.pendingTitle")}</strong>
          <span>{t("slots.pendingText")}</span>
          <Button variant="ghost" onClick={reloadFromServer}>
            {t("slots.pendingReload")}
          </Button>
        </div>
      )}

      <Card
        title={t("slots.title", { n: draft.length })}
        subtitle={t("slots.subtitle", { max: maxClients || "—", cars: carsFromConfig.length })}
        actions={
          <div className="row gap">
            <Button onClick={addSlot}>{t("slots.addSlot")}</Button>
            <Button variant="ghost" onClick={reloadFromServer}>
              {t("slots.reload")}
            </Button>
            <Button
              variant="primary"
              disabled={pendingRemote}
              title={pendingRemote ? t("slots.pendingText") : undefined}
              onClick={() =>
                void run("save-slots", async () => {
                  await api.saveSlots(draft, syncedRevisionRef.current);
                  syncedRef.current = draft.map((slot) => ({ ...slot }));
                  setPendingRemote(false);
                  await refreshSlots();
                  await refreshContent();
                  notify(t("slots.saved"), "success");
                })
              }
            >
              {t("slots.save")}
            </Button>
          </div>
        }
      >
        {draft.length === 0 ? (
          <Empty>{t("slots.empty")}</Empty>
        ) : (
          <div className="table-wrap">
            <table className="table slots-table">
              <thead>
                <tr>
                  <th>#</th>
                  <th>{t("slots.colCar")}</th>
                  <th>{t("slots.colSkin")}</th>
                  <th>{t("slots.colDriver")}</th>
                  <th>{t("slots.colTeam")}</th>
                  <th>{t("slots.colBallast")}</th>
                  <th>{t("slots.colRestrictor")}</th>
                  <th>{t("slots.colSpectator")}</th>
                  <th />
                </tr>
              </thead>
              <tbody>
                {draft.map((slot, index) => {
                  const car = carFor(slot.model);
                  const skin = skinsFor(slot.model).find((item) => item.name === slot.skin);
                  const missing = slot.model && !car;
                  return (
                    <tr key={index} className={missing ? "row-warn" : ""}>
                      <td className="mono">{index}</td>
                      <td className="slot-pick-cell">
                        <div className="slot-car">
                          <Thumb
                            src={car?.preview ?? car?.badge}
                            alt={slot.model}
                            fallback={slot.model || "—"}
                            width={220}
                          />
                          <div className="slot-car-text">
                            <strong>{car?.name || slot.model || "—"}</strong>
                            <button className="link-btn" onClick={() => setModelPicker(index)}>
                              {t("slots.changeCar")}
                            </button>
                          </div>
                        </div>
                        {missing && <Badge tone="red">{t("slots.missing")}</Badge>}
                      </td>
                      <td className="slot-pick-cell">
                        <div className="slot-car">
                          <Thumb
                            src={skin?.preview}
                            alt={slot.skin}
                            fallback={slot.skin || "—"}
                            width={220}
                          />
                          <div className="slot-car-text">
                            <strong>{skin?.display || slot.skin || t("slots.noSkin")}</strong>
                            <span className="slot-skin-meta">
                              {skin?.number ? `#${skin.number}` : ""}
                              {skin?.team ? ` · ${skin.team}` : ""}
                              {!skin?.team && skin?.country ? skin.country : ""}
                            </span>
                            <button
                              className="link-btn"
                              disabled={!car}
                              onClick={() => setSkinPicker(index)}
                            >
                              {t("slots.chooseSkin")}
                            </button>
                          </div>
                        </div>
                      </td>
                      <td>
                        <input
                          value={slot.driver_name}
                          placeholder={t("slots.freeDriver")}
                          onChange={(event) => update(index, { driver_name: event.target.value })}
                        />
                      </td>
                      <td>
                        <input
                          value={slot.team}
                          placeholder="—"
                          onChange={(event) => update(index, { team: event.target.value })}
                        />
                      </td>
                      <td>
                        <input
                          className="narrow"
                          type="number"
                          value={slot.ballast}
                          onChange={(event) => update(index, { ballast: num(event.target.value) })}
                        />
                      </td>
                      <td>
                        <input
                          className="narrow"
                          type="number"
                          value={slot.restrictor}
                          onChange={(event) => update(index, { restrictor: num(event.target.value) })}
                        />
                      </td>
                      <td>
                        <input
                          type="checkbox"
                          checked={slot.spectator_mode === 1}
                          onChange={(event) =>
                            update(index, { spectator_mode: event.target.checked ? 1 : 0 })
                          }
                        />
                      </td>
                      <td>
                        <div className="row gap-sm">
                          <button className="icon-btn" title={t("slots.duplicate")} onClick={() => duplicate(index)}>⧉</button>
                          <button className="icon-btn danger" title={t("slots.remove")} onClick={() => remove(index)}>✕</button>
                        </div>
                      </td>
                    </tr>
                  );
                })}
              </tbody>
            </table>
          </div>
        )}
      </Card>

      {modelPicker !== null && library && (
        <ContentPicker
          kind="cars"
          multiple={false}
          library={library}
          initial={[draft[modelPicker]?.model ?? ""].filter(Boolean)}
          onClose={() => setModelPicker(null)}
          onApply={(ids) => {
            if (modelPicker === null) return;
            applyModel(modelPicker, ids[0] ?? "");
            setModelPicker(null);
          }}
        />
      )}

      {addCars && library && (
        <ContentPicker
          kind="cars"
          multiple
          library={library}
          initial={carsFromConfig}
          onClose={() => setAddCars(false)}
          onApply={(ids) => {
            setAddCars(false);
            addCarsToServer(ids);
          }}
        />
      )}

      {addTrack && library && (
        <ContentPicker
          kind="tracks"
          multiple={false}
          library={library}
          initial={[value("SERVER", "TRACK")].filter(Boolean)}
          onClose={() => setAddTrack(false)}
          onApply={(ids) => {
            setAddTrack(false);
            addTrackToServer(ids);
          }}
        />
      )}

      {skinPicker !== null && skinCar && (
        <SkinPicker
          car={skinCar}
          current={draft[skinPicker]?.skin ?? ""}
          onClose={() => setSkinPicker(null)}
          onApply={(skin) => {
            if (skinPicker !== null) update(skinPicker, { skin });
            setSkinPicker(null);
          }}
        />
      )}
    </div>
  );
}
