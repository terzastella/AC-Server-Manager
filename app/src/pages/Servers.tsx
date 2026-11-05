import { useState } from "react";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { api } from "../api";
import { Badge, Button, Card } from "../components/ui";
import { useApp } from "../store";
import { useT } from "../i18n/core";
import { profileName } from "../profileName";

export function Servers() {
  const { t } = useT();
  const { profiles, status, run, notify, refreshAll, busy, switchProfile } = useApp();
  const running = status?.running ?? false;
  const locked = running || Boolean(busy);
  const [name, setName] = useState("");
  const [importPath, setImportPath] = useState("");
  const [importName, setImportName] = useState("");
  const [editing, setEditing] = useState<string | null>(null);
  const [draft, setDraft] = useState("");
  const [pending, setPending] = useState<string | null>(null);

  const create = () =>
    void run("profile-create", async () => {
      await api.createProfile(name.trim());
      setName("");
      await refreshAll();
      notify(t("servers.notifyCreated"), "success");
    });

  const duplicate = (id: string) =>
    void run("profile-duplicate", async () => {
      await api.duplicateProfile(id);
      await refreshAll();
      notify(t("servers.notifyDuplicated"), "success");
    });

  const saveRename = (id: string) =>
    void run("profile-rename", async () => {
      await api.renameProfile(id, draft.trim());
      setEditing(null);
      setDraft("");
      await refreshAll();
      notify(t("servers.notifyRenamed"), "success");
    });

  const remove = (id: string) =>
    void run("profile-delete", async () => {
      await api.deleteProfile(id);
      setPending(null);
      await refreshAll();
      notify(t("servers.notifyDeleted"), "success");
    });

  const browse = async () => {
    const picked = await openDialog({
      directory: true,
      multiple: false,
      title: t("servers.importPickTitle"),
    });
    if (typeof picked === "string") setImportPath(picked);
  };

  const importServer = () =>
    void run("profile-import", async () => {
      await api.importProfile(importPath.trim(), importName.trim());
      setImportPath("");
      setImportName("");
      await refreshAll();
      notify(t("servers.notifyImported"), "success");
    });

  return (
    <div className="page">
      <Card
        title={t("servers.title")}
        subtitle={t("servers.subtitle")}
      >
        {running && (
          <p className="muted">{t("servers.running")}</p>
        )}
        <div className="stack-sm">
          {(profiles?.profiles ?? []).map((profile) => (
            <div key={profile.id} className="profile-row">
              <div className="profile-row-main">
                {editing === profile.id ? (
                  <input
                    value={draft}
                    onChange={(event) => setDraft(event.target.value)}
                    autoFocus
                  />
                ) : (
                  <strong>{profileName(profile, t)}</strong>
                )}
                {profiles?.active === profile.id && <Badge tone="accent">{t("servers.active")}</Badge>}
                {profile.legacy && <Badge tone="muted">{t("servers.default")}</Badge>}
              </div>
              <div className="row gap">
                {editing === profile.id ? (
                  <>
                    <Button
                      variant="primary"
                      disabled={!draft.trim() || Boolean(busy)}
                      onClick={() => saveRename(profile.id)}
                    >
                      {t("common.save")}
                    </Button>
                    <Button
                      variant="ghost"
                      onClick={() => {
                        setEditing(null);
                        setDraft("");
                      }}
                    >
                      {t("common.cancel")}
                    </Button>
                  </>
                ) : (
                  <>
                    {profiles?.active !== profile.id && (
                      <Button
                        variant="primary"
                        disabled={locked}
                        onClick={() =>
                          void run("profile-activate", async () => {
                            await switchProfile(profile.id);
                            notify(t("servers.notifyActivated", { name: profileName(profile, t) }), "success");
                          })
                        }
                      >
                        {t("servers.activate")}
                      </Button>
                    )}
                    <Button
                      variant="ghost"
                      disabled={locked}
                      onClick={() => {
                        setEditing(profile.id);
                        setDraft(profile.name);
                      }}
                    >
                      {t("servers.rename")}
                    </Button>
                    <Button
                      variant="ghost"
                      disabled={locked}
                      onClick={() => duplicate(profile.id)}
                    >
                      {t("servers.duplicate")}
                    </Button>
                    {!profile.legacy && (
                      <Button
                        variant="danger"
                        disabled={locked || (profiles?.profiles.length ?? 0) <= 1}
                        onClick={() => setPending(profile.id)}
                      >
                        {t("common.delete")}
                      </Button>
                    )}
                  </>
                )}
              </div>
              {pending === profile.id && (
                <div className="profile-confirm">
                  <span>{t("servers.confirmDelete", { name: profileName(profile, t) })}</span>
                  <Button
                    variant="danger"
                    onClick={() => remove(profile.id)}
                  >
                    {t("servers.confirm")}
                  </Button>
                  <Button variant="ghost" onClick={() => setPending(null)}>
                    {t("common.cancel")}
                  </Button>
                </div>
              )}
            </div>
          ))}
        </div>
      </Card>

      <Card title={t("servers.importTitle")} subtitle={t("servers.importSubtitle")}>
        <div className="stack-sm">
          <div className="row gap">
            <input
              value={importPath}
              placeholder={t("servers.pathPlaceholder")}
              onChange={(event) => setImportPath(event.target.value)}
            />
            <Button variant="ghost" onClick={() => void browse()}>
              {t("servers.browse")}
            </Button>
          </div>
          <div className="row gap">
            <input
              value={importName}
              placeholder={t("servers.namePlaceholder")}
              maxLength={40}
              onChange={(event) => setImportName(event.target.value)}
            />
            <Button
              variant="primary"
              disabled={!importPath.trim() || locked}
              onClick={importServer}
            >
              {t("servers.import")}
            </Button>
          </div>
        </div>
        <p className="muted">{t("servers.importHelp")}</p>
      </Card>

      <Card title={t("servers.newTitle")} subtitle={t("servers.newSubtitle")}>
        <div className="row gap">
          <input
            value={name}
            placeholder={t("servers.newPlaceholder")}
            maxLength={40}
            onChange={(event) => setName(event.target.value)}
            onKeyDown={(event) => {
              if (event.key === "Enter" && name.trim() && !running) create();
            }}
          />
          <Button variant="primary" disabled={!name.trim() || locked} onClick={create}>
            {t("servers.create")}
          </Button>
        </div>
        <p className="muted">{t("servers.newHelp")}</p>
      </Card>
    </div>
  );
}
