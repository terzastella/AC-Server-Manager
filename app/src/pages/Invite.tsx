import { useEffect, useState } from "react";
import { api } from "../api";
import { useApp } from "../store";
import { Badge, Button, Card } from "../components/ui";
import { QrCode } from "../components/QrCode";
import { copyText } from "../utils";
import { useT } from "../i18n/core";
import type { InviteSettings } from "../types";

/** Giorni residui arrotondati, per il selettore della scadenza. */
function expiryDays(expiresAt: number | null): number {
  if (!expiresAt) return 0;
  const days = Math.round((expiresAt - Date.now() / 1000) / 86400);
  if (days <= 1) return 1;
  if (days <= 7) return 7;
  if (days <= 30) return 30;
  return 0;
}

function expiryFromDays(days: number): number | null {
  return days > 0 ? Math.round(Date.now() / 1000) + days * 86400 : null;
}

export function Invite() {
  const { t } = useT();
  const { config, adapters, run, notify, refreshConfig } = useApp();
  const [draft, setDraft] = useState<InviteSettings | null>(null);
  const [ip, setIp] = useState("");
  const [link, setLink] = useState("");
  const [protocol, setProtocol] = useState(false);

  const httpPortValue = Number.parseInt(
    config?.sections
      .find((section) => section.name === "SERVER")
      ?.entries.find((entry) => entry.key === "HTTP_PORT")?.value ?? "8081",
    10,
  );
  const httpPort = Number.isFinite(httpPortValue) && httpPortValue > 0 ? httpPortValue : 8081;
  const pickup =
    config?.sections
      .find((section) => section.name === "SERVER")
      ?.entries.find((entry) => entry.key === "PICKUP_MODE_ENABLED")?.value === "1";

  useEffect(() => {
    void api.inviteSettings().then(setDraft).catch(() => setDraft(null));
    void api.inviteProtocolStatus().then(setProtocol).catch(() => setProtocol(false));
  }, []);

  useEffect(() => {
    if (!ip && adapters.length > 0) setIp(adapters[0].ip);
  }, [adapters, ip]);

  useEffect(() => {
    if (!ip) {
      setLink("");
      return;
    }
    void api
      .inviteLink(ip, httpPort)
      .then(setLink)
      .catch(() => setLink(""));
  }, [ip, httpPort, draft]);

  if (!draft) {
    return (
      <div className="page">
        <Card title={t("invite.title")}>
          <p className="muted">{t("invite.loading")}</p>
        </Card>
      </div>
    );
  }

  const update = (patch: Partial<InviteSettings>) =>
    setDraft((current) => (current ? { ...current, ...patch } : current));

  return (
    <div className="page">
      <div className="grid grid-2">
        <Card title={t("invite.graphicsTitle")} subtitle={t("invite.graphicsSubtitle")}>
          <label className="field">
            <span className="field-label">{t("invite.nameLabel")}</span>
            <input
              value={draft.name}
              placeholder={t("invite.namePlaceholder")}
              onChange={(event) => update({ name: event.target.value })}
            />
          </label>
          <label className="field">
            <span className="field-label">{t("invite.accentLabel")}</span>
            <div className="row gap">
              <input
                type="color"
                className="color-input"
                value={draft.accent}
                onChange={(event) => update({ accent: event.target.value })}
              />
              <input
                value={draft.accent}
                onChange={(event) => update({ accent: event.target.value })}
              />
            </div>
          </label>
          <label className="field">
            <span className="field-label">{t("invite.bannerLabel")}</span>
            <input
              value={draft.banner}
              placeholder="https://…/banner.jpg"
              onChange={(event) => update({ banner: event.target.value })}
            />
          </label>
          <label className="field">
            <span className="field-label">{t("invite.apiPortLabel")}</span>
            <input
              type="number"
              className="narrow"
              value={draft.api_port}
              readOnly
              title={t("invite.apiPortHint")}
            />
          </label>
          <label className="field">
            <span className="field-label">{t("invite.bindLabel")}</span>
            <select
              value={draft.api_bind}
              onChange={(event) => update({ api_bind: event.target.value })}
              title={t("invite.bindHint")}
            >
              <option value="127.0.0.1">{t("invite.bindLocal")}</option>
              <option value="0.0.0.0">{t("invite.bindAll")}</option>
              {adapters.map((adapter) => (
                <option key={adapter.ip} value={adapter.ip}>
                  {adapter.ip} ({adapter.name})
                </option>
              ))}
            </select>
            {draft.api_bind === "0.0.0.0" && (
              <span className="field-warn">{t("invite.bindWarning")}</span>
            )}
            {draft.api_bind === "127.0.0.1" && (
              <span className="field-warn">{t("invite.bindLocalWarning")}</span>
            )}
          </label>
          <label className="field">
            <span className="field-label">{t("invite.expiryLabel")}</span>
            <select
              value={expiryDays(draft.expires_at)}
              onChange={(event) => update({ expires_at: expiryFromDays(Number(event.target.value)) })}
              title={t("invite.expiryHint")}
            >
              <option value={0}>{t("invite.expiryNever")}</option>
              <option value={1}>{t("invite.expiry1")}</option>
              <option value={7}>{t("invite.expiry7")}</option>
              <option value={30}>{t("invite.expiry30")}</option>
            </select>
          </label>

          <div className="row gap">
            <Button
              variant="primary"
              onClick={() =>
                void run("save-invite", async () => {
                  const saved = await api.saveInviteSettings(draft);
                  setDraft(saved);
                  notify(t("invite.saved"), "success");
                })
              }
            >
              {t("invite.save")}
            </Button>
            <Button
              onClick={() =>
                void run("regen-token", async () => {
                  const saved = await api.regenerateInviteToken();
                  setDraft(saved);
                  notify(t("invite.tokenRegenerated"), "success");
                })
              }
            >
              {t("invite.regenToken")}
            </Button>
          </div>
        </Card>

        <Card title={t("invite.linkTitle")} subtitle={t("invite.linkSubtitle")}>
          <label className="field">
            <span className="field-label">{t("invite.ipLabel")}</span>
            <input value={ip} onChange={(event) => setIp(event.target.value)} />
          </label>

          <div className="invite-link">{link || t("invite.ipUnavailable")}</div>

          <div className="row gap">
            <Button
              disabled={!link}
              onClick={() =>
                void copyText(link).then((ok) =>
                  notify(ok ? t("invite.linkCopied") : t("invite.copyFailed"), ok ? "success" : "error"),
                )
              }
            >
              {t("invite.copyLink")}
            </Button>
            <Button
              variant={protocol ? "ghost" : "primary"}
              onClick={() =>
                void run("register-protocol", async () => {
                  const ok = await api.registerInviteProtocol();
                  setProtocol(ok);
                  notify(
                    ok
                      ? t("invite.protocolRegistered")
                      : t("invite.protocolFailed"),
                    ok ? "success" : "error",
                  );
                })
              }
            >
              {protocol ? t("invite.protocolActive") : t("invite.registerProtocol")}
            </Button>
          </div>

          {link && (
            <div className="qr-small">
              <QrCode value={link} size={148} />
            </div>
          )}

          <div className="invite-facts">
            <div>
              <span>Token</span>
              <code>{draft.token}</code>
            </div>
            <div>
              <span>{t("invite.apiPort")}</span>
              <code>{draft.api_port}</code>
            </div>
            <div>
              <span>{t("invite.status")}</span>
              <Badge tone={protocol ? "green" : "amber"}>
                {protocol ? t("invite.protocolActiveLower") : t("invite.protocolMissing")}
              </Badge>
            </div>
          </div>
        </Card>
      </div>

      <div className="banner banner-blue">
        <strong>{t("invite.howTitle")}</strong>
        <span>
          {t("invite.howTextBefore")}
          <code>entry_list.ini</code>
          {t("invite.howTextAfter")}
        </span>
      </div>

      {pickup && (
        <div className="banner banner-amber">
          <strong>{t("invite.pickupTitle")}</strong>
          <span>{t("invite.pickupText")}</span>
          <Button
            variant="ghost"
            onClick={() =>
              void run("disable-pickup", async () => {
                if (!config) return;
                const sections = config.sections.map((section) =>
                  section.name === "SERVER"
                    ? {
                        ...section,
                        entries: section.entries.map((entry) =>
                          entry.key.toUpperCase() === "PICKUP_MODE_ENABLED"
                            ? { ...entry, value: "0" }
                            : entry,
                        ),
                      }
                    : section,
                );
                await api.saveConfig(sections);
                await refreshConfig();
                notify(t("invite.pickupDisabled"), "success");
              })
            }
          >
            {t("invite.disablePickup")}
          </Button>
        </div>
      )}
    </div>
  );
}
