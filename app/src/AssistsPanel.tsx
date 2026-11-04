import { useCallback, useEffect, useRef, useState } from "react";
import { api } from "./api";
import { Button, Toggle } from "./components/ui";
import { useT, type TKey } from "./i18n/core";
import type { Assists } from "./types";

const DEFAULT_ASSISTS: Assists = {
  ideal_line: false,
  auto_blip: false,
  auto_clutch: false,
  auto_shifter: false,
  traction_control: 0,
  abs: 1,
  stability_control: 0,
  damage: 0,
  tyre_wear: 1,
  slipstream: 1,
  fuel_rate: false,
  tyre_blankets: false,
};

type BoolKey = "ideal_line" | "auto_blip" | "auto_clutch" | "auto_shifter" | "fuel_rate" | "tyre_blankets";

const TOGGLES: Array<{ key: BoolKey; labelKey: TKey; hintKey: TKey }> = [
  {
    key: "auto_shifter",
    labelKey: "assists.autoShifter",
    hintKey: "assists.autoShifterHint",
  },
  {
    key: "auto_clutch",
    labelKey: "assists.autoClutch",
    hintKey: "assists.autoClutchHint",
  },
  {
    key: "auto_blip",
    labelKey: "assists.autoBlip",
    hintKey: "assists.autoBlipHint",
  },
  {
    key: "ideal_line",
    labelKey: "assists.idealLine",
    hintKey: "assists.idealLineHint",
  },
  {
    key: "fuel_rate",
    labelKey: "assists.fuelRate",
    hintKey: "assists.fuelRateHint",
  },
  {
    key: "tyre_blankets",
    labelKey: "assists.tyreBlankets",
    hintKey: "assists.tyreBlanketsHint",
  },
];

type NumberKey =
  | "traction_control"
  | "abs"
  | "stability_control"
  | "damage"
  | "tyre_wear"
  | "slipstream";

const SLIDERS: Array<{
  key: NumberKey;
  labelKey: TKey;
  hintKey: TKey;
  min: number;
  max: number;
  step: number;
  text: (value: number, t: (key: TKey) => string) => string;
}> = [
  {
    key: "traction_control",
    labelKey: "assists.tractionControl",
    hintKey: "assists.tractionControlHint",
    min: 0,
    max: 2,
    step: 1,
    text: (value, t) =>
      [t("assists.levelOff"), t("assists.levelFactory"), t("assists.levelOn")][value] ??
      t("assists.levelOff"),
  },
  {
    key: "abs",
    labelKey: "assists.abs",
    hintKey: "assists.absHint",
    min: 0,
    max: 2,
    step: 1,
    text: (value, t) =>
      [t("assists.levelOff"), t("assists.levelFactory"), t("assists.levelOn")][value] ??
      t("assists.levelOff"),
  },
  {
    key: "stability_control",
    labelKey: "assists.stabilityControl",
    hintKey: "assists.stabilityControlHint",
    min: 0,
    max: 100,
    step: 5,
    text: (value, t) => (value === 0 ? t("assists.levelOff") : `${value}%`),
  },
  {
    key: "damage",
    labelKey: "assists.damage",
    hintKey: "assists.damageHint",
    min: 0,
    max: 100,
    step: 10,
    text: (value, t) => (value === 0 ? t("assists.levelOff") : `${value}%`),
  },
  {
    key: "tyre_wear",
    labelKey: "assists.tyreWear",
    hintKey: "assists.tyreWearHint",
    min: 0,
    max: 3,
    step: 1,
    text: (value, t) => (value === 0 ? t("assists.levelOff") : `${value}x`),
  },
  {
    key: "slipstream",
    labelKey: "assists.slipstream",
    hintKey: "assists.slipstreamHint",
    min: 1,
    max: 5,
    step: 1,
    text: (value) => `${value}x`,
  },
];

export function AssistsPanel({ compact = false }: { compact?: boolean }) {
  const { t } = useT();
  const [values, setValues] = useState<Assists | null>(null);
  const [exists, setExists] = useState(false);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState("");
  const pending = useRef<Assists | null>(null);
  const timer = useRef<number | null>(null);

  useEffect(() => {
    let cancelled = false;
    api
      .assistsState()
      .then((value) => {
        if (cancelled) return;
        setValues(value.values);
        setExists(value.exists);
      })
      .catch((problem) => {
        if (!cancelled) setError(String(problem));
      });
    return () => {
      cancelled = true;
    };
  }, []);

  const flush = useCallback(async () => {
    const next = pending.current;
    if (!next) {
      setSaving(false);
      return;
    }
    pending.current = null;
    try {
      const saved = await api.saveAssists(next);
      setExists(saved.exists);
      setError("");
    } catch (problem) {
      setError(String(problem));
    } finally {
      setSaving(false);
    }
  }, []);

  // Salva anche se il pannello viene smontato (es. passaggio alla finestra Entra).
  useEffect(
    () => () => {
      if (timer.current) window.clearTimeout(timer.current);
      if (pending.current) void api.saveAssists(pending.current);
    },
    [],
  );

  const update = (next: Assists) => {
    setValues(next);
    pending.current = next;
    setSaving(true);
    if (timer.current) window.clearTimeout(timer.current);
    timer.current = window.setTimeout(() => {
      timer.current = null;
      void flush();
    }, 350);
  };

  if (!values) {
    return <p className="muted">{t("assists.loading")}</p>;
  }

  return (
    <div className={compact ? "assists compact" : "assists"}>
      <div className="assists-groups">
        <div className="assists-group">
          {!compact && <h3 className="assists-group-title">{t("assists.toggles")}</h3>}
          <div className="assists-list">
            {TOGGLES.map((row) => (
              <div className="assists-row" key={row.key}>
                <div>
                  <strong>{t(row.labelKey)}</strong>
                  {!compact && <span>{t(row.hintKey)}</span>}
                </div>
                <Toggle
                  checked={values[row.key]}
                  ariaLabel={t(row.labelKey)}
                  onChange={(value) => update({ ...values, [row.key]: value })}
                />
              </div>
            ))}
          </div>
        </div>

        <div className="assists-group">
          {!compact && <h3 className="assists-group-title">{t("assists.levels")}</h3>}
          <div className="assists-sliders">
            {SLIDERS.map((row) => (
              <label className="assist-slider" key={row.key}>
                <span className="assist-slider-head">
                  <strong>{t(row.labelKey)}</strong>
                  <b>{row.text(values[row.key], t)}</b>
                </span>
                {!compact && <span className="assist-slider-hint">{t(row.hintKey)}</span>}
                <input
                  type="range"
                  min={row.min}
                  max={row.max}
                  step={row.step}
                  value={values[row.key]}
                  aria-label={t(row.labelKey)}
                  onChange={(event) => update({ ...values, [row.key]: Number(event.target.value) })}
                />
              </label>
            ))}
          </div>
        </div>
      </div>

      {!compact && (
        <div className="assists-foot">
          <span className="assists-status">
            {saving
              ? t("assists.saving")
              : exists
                ? t("assists.saved")
                : t("assists.willCreate")}
          </span>
          <Button variant="ghost" onClick={() => update(DEFAULT_ASSISTS)}>
            {t("assists.defaults")}
          </Button>
        </div>
      )}

      {!compact && <p className="assists-note">{t("assists.note")}</p>}
      {error && <p className="launcher-error">{error}</p>}
    </div>
  );
}
