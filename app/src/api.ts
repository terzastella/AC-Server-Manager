import { invoke } from "@tauri-apps/api/core";
import type {
  Adapter,
  AppInfo,
  BackupInfo,
  CarSlot,
  CmScan,
  CmSetting,
  ContentLibrary,
  ContentReport,
  CopyReport,
  IniDoc,
  IniSection,
  LogEntry,
  LogFile,
  PortState,
  PresetInfo,
  ProfileIndex,
  ServerStatus,
  Settings,
  InviteSettings,
  JoinArgs,
  InviteServerInfo,
  LauncherState,
  Assists,
  AssistsState,
} from "./types";
export const api = {
  info: () => invoke<AppInfo>("app_info"),
  getSettings: () => invoke<Settings>("get_settings"),
  saveSettings: (settings: Settings) => invoke<void>("save_settings", { settings }),
  setLanguage: (language: "it" | "en") =>
    invoke<Settings>("set_language", { language }),
  setServerDir: (dir: string) => invoke<AppInfo>("set_server_dir", { dir }),

  listProfiles: () => invoke<ProfileIndex>("list_profiles"),
  createProfile: (name: string) => invoke<ProfileIndex>("create_profile", { name }),
  importProfile: (path: string, name: string) =>
    invoke<ProfileIndex>("import_profile", { path, name }),
  duplicateProfile: (id: string) => invoke<ProfileIndex>("duplicate_profile", { id }),
  renameProfile: (id: string, name: string) =>
    invoke<ProfileIndex>("rename_profile", { id, name }),
  deleteProfile: (id: string) => invoke<ProfileIndex>("delete_profile", { id }),
  setActiveProfile: (id: string) => invoke<ProfileIndex>("set_active_profile", { id }),

  status: () => invoke<ServerStatus>("server_status"),
  start: () => invoke<ServerStatus>("start_server"),
  stop: () => invoke<ServerStatus>("stop_server"),
  restart: () => invoke<ServerStatus>("restart_server"),
  stopExternal: () => invoke<number>("stop_external_server"),

  logs: () => invoke<LogEntry[]>("get_logs"),
  clearLogs: () => invoke<void>("clear_logs"),
  listLogs: () => invoke<LogFile[]>("list_logs"),
  tailLog: (file: string, lines = 400) => invoke<string>("tail_log", { file, lines }),

  readConfig: () => invoke<IniDoc>("read_config"),
  saveConfig: (sections: IniSection[]) => invoke<string>("save_config", { sections }),
  readWelcome: () => invoke<string>("read_welcome"),
  saveWelcome: (text: string) => invoke<void>("save_welcome", { text }),

  readSlots: () => invoke<CarSlot[]>("read_entry_list"),
  saveSlots: (slots: CarSlot[]) => invoke<string>("save_entry_list", { slots }),

  scanContent: () => invoke<ContentReport>("scan_content"),
  contentLibrary: () => invoke<ContentLibrary>("content_library"),
  contentImage: (path: string, width = 480) =>
    invoke<string>("content_image", { path, width }),
  copyContent: (gameDir: string) => invoke<CopyReport>("copy_content", { gameDir }),

  adapters: () => invoke<Adapter[]>("list_adapters"),
  writeShare: (ip: string, httpPort: number) =>
    invoke<string[]>("write_share_files", { ip, httpPort }),
  checkPorts: (ports: number[]) => invoke<PortState[]>("check_ports", { ports }),
  serverInfo: (port: number) => invoke<Record<string, unknown>>("server_info", { port }),
  openFirewall: () => invoke<void>("open_firewall"),

  listPresets: () => invoke<PresetInfo[]>("list_presets"),
  savePreset: (name: string) => invoke<string>("save_preset", { name }),
  loadPreset: (name: string) => invoke<void>("load_preset", { name }),
  listBackups: () => invoke<BackupInfo[]>("list_backups"),
  backupNow: () => invoke<string>("backup_now"),
  openPath: (path: string) => invoke<void>("open_path", { path }),
  openUrl: (url: string) => invoke<void>("open_url", { url }),
  cmProtocolReady: () => invoke<boolean>("cm_protocol_ready"),

  cmScan: () => invoke<CmScan>("cm_scan"),
  cmImportPreset: (source: string) =>
    invoke<string>("cm_import_server_preset", { source }),
  cmSaveSnapshot: (settings: CmSetting[], source: string | null) =>
    invoke<string>("cm_save_snapshot", { settings, source }),

  inviteSettings: () => invoke<InviteSettings>("invite_settings"),
  saveInviteSettings: (invite: InviteSettings) =>
    invoke<InviteSettings>("save_invite_settings", { invite }),
  regenerateInviteToken: () => invoke<InviteSettings>("regenerate_invite_token"),
  inviteLink: (ip: string, httpPort: number) =>
    invoke<string>("invite_link", { ip, httpPort }),
  registerInviteProtocol: () => invoke<boolean>("register_invite_protocol"),
  inviteProtocolStatus: () => invoke<boolean>("invite_protocol_status"),
  joinContext: () => invoke<JoinArgs | null>("join_context"),
  launcherLibrary: () => invoke<ContentLibrary>("launcher_library"),
  remoteServerInfo: (host: string, apiPort: number, token: string) =>
    invoke<InviteServerInfo>("remote_server_info", { host, apiPort, token }),
  remoteReserve: (
    host: string,
    apiPort: number,
    token: string,
    driver: string,
    car: string,
    skin: string,
    guid: string,
  ) => invoke<{ ok: boolean; slot?: number; error?: string }>("remote_reserve", {
    host,
    apiPort,
    token,
    driver,
    car,
    skin,
    guid,
  }),

  steamGuid: () => invoke<string | null>("steam_guid"),
  launchGame: (
    ip: string,
    port: number,
    httpPort: number,
    name: string,
    car: string,
    skin: string,
    serverName: string,
  ) =>
    invoke<string>("launch_game", {
      ip,
      port,
      httpPort,
      name,
      car,
      skin,
      serverName,
    }),

  launcherState: () => invoke<LauncherState>("launcher_state"),
  addInvite: (link: string) => invoke<LauncherState>("add_invite", { link }),
  removeInvite: (link: string) => invoke<LauncherState>("remove_invite", { link }),
  setDriverName: (name: string) => invoke<string>("set_driver_name", { name }),
  openJoinLink: (link: string) => invoke<void>("open_join_link", { link }),
  assistsState: () => invoke<AssistsState>("assists_state"),
  saveAssists: (values: Assists) => invoke<AssistsState>("save_assists", { values }),
};

export function openPath(path: string) {
  return api.openPath(path);
}
