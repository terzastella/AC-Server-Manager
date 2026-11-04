export type LogEntry = { ts: string; stream: string; message: string };

export type InviteSettings = {
  name: string;
  token: string;
  accent: string;
  banner: string;
  api_port: number;
};

export type JoinArgs = {
  ip: string;
  http_port: number;
  api_port: number;
  token: string;
  name: string;
  accent: string;
  banner: string;
};

export type InviteCar = {
  id: string;
  name: string;
  skins: string[];
};

export type InviteServerInfo = {
  name: string;
  track: string;
  max_clients: number;
  port: number;
  free_slots: number;
  running: boolean;
  cars: InviteCar[];
};

export type ServerStatus = {
  running: boolean;
  pid: number | null;
  uptime_secs: number;
  server_dir: string;
  log_file: string | null;
};

export type AppInfo = {
  server_dir: string;
  exe_path: string;
  has_server: boolean;
  version: string;
  launcher_mode: boolean;
  profile: string;
  profile_name: string;
  profiles: ServerProfile[];
};

export type ServerProfile = {
  id: string;
  name: string;
  slug: string;
  legacy: boolean;
  created: string;
};

export type ProfileIndex = {
  version: number;
  active: string;
  profiles: ServerProfile[];
};

export type SavedInvite = { link: string };

export type LauncherState = {
  launcher_mode: boolean;
  driver_name: string;
  invites: SavedInvite[];
};

export type Assists = {
  ideal_line: boolean;
  auto_blip: boolean;
  auto_clutch: boolean;
  auto_shifter: boolean;
  traction_control: number;
  abs: number;
  stability_control: number;
  damage: number;
  tyre_wear: number;
  slipstream: number;
  fuel_rate: boolean;
  tyre_blankets: boolean;
};

export type AssistsState = {
  path: string | null;
  exists: boolean;
  values: Assists;
};

export type IniEntry = { key: string; value: string };
export type IniSection = { name: string; entries: IniEntry[] };
export type IniDoc = { sections: IniSection[]; newline: string };

export type CarSlot = {
  driver_name: string;
  team: string;
  model: string;
  skin: string;
  spectator_mode: number;
  guid: string;
  ballast: number;
  restrictor: number;
};

export type CarInfo = {
  id: string;
  name: string;
  brand: string;
  skins: string[];
  used: boolean;
};

export type TrackInfo = { id: string; name: string; used: boolean };
export type WeatherInfo = { id: string; used: boolean };

export type ContentReport = {
  cars: CarInfo[];
  tracks: TrackInfo[];
  weather: WeatherInfo[];
  used_cars: string[];
  used_tracks: string[];
  missing_cars: string[];
  missing_tracks: string[];
  missing_weather: string[];
  game_dir: string | null;
};

export type CopyReport = {
  copied: string[];
  skipped: string[];
  errors: string[];
};

export type Adapter = { name: string; ip: string; kind: string };

export type PortState = { port: number; tcp_free: boolean; udp_free: boolean };

export type PresetInfo = {
  name: string;
  server_name: string;
  track: string;
  cars: string[];
};

export type BackupInfo = { name: string; path: string; modified: number };

export type LogFile = { name: string; path: string; size: number; modified: number };

export type Settings = {
  server_dir: string | null;
  game_dir: string | null;
  cm_dir: string | null;
  theme: string;
  language: string;
};

export type CmSetting = {
  key: string;
  value: string;
  group: string;
  kind: string;
  encrypted: boolean;
};

export type CmServerPreset = {
  name: string;
  path: string;
  server_name: string;
  track: string;
  cars: string[];
  max_clients: string;
  password: boolean;
  files: string[];
  has_wrapper: boolean;
  preview: string | null;
};

export type CmUserPreset = {
  category: string;
  name: string;
  path: string;
  size: number;
};

export type CmLocation = {
  cm_exe: string | null;
  data_dir: string | null;
  values_file: string | null;
  ac_root: string | null;
  version: string | null;
  running: boolean;
};

export type CmScan = {
  location: CmLocation;
  settings: CmSetting[];
  server_presets: CmServerPreset[];
  user_presets: CmUserPreset[];
  total_keys: number;
  error: string | null;
};

export type SkinEntry = {
  name: string;
  display: string;
  preview: string | null;
  team: string;
  country: string;
  number: string;
};

export type CarEntry = {
  id: string;
  name: string;
  brand: string;
  class: string;
  tags: string[];
  specs: Record<string, unknown>;
  preview: string | null;
  badge: string | null;
  skins: SkinEntry[];
  in_server: boolean;
  in_game: boolean;
  used: boolean;
  server_path: string | null;
  game_path: string | null;
};

export type TrackEntry = {
  id: string;
  name: string;
  country: string;
  city: string;
  length: string;
  tags: string[];
  preview: string | null;
  outline: string | null;
  in_server: boolean;
  in_game: boolean;
  used: boolean;
  server_path: string | null;
  game_path: string | null;
};

export type ContentLibrary = {
  cars: CarEntry[];
  tracks: TrackEntry[];
  game_dir: string | null;
};
