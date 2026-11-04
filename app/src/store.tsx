import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useRef,
  useState,
} from "react";
import { listen } from "@tauri-apps/api/event";
import { api } from "./api";
import type {
  Adapter,
  AppInfo,
  CarSlot,
  ContentReport,
  IniDoc,
  LogEntry,
  ProfileIndex,
  ServerStatus,
  Settings,
} from "./types";

export type Toast = { id: number; text: string; tone: "info" | "success" | "error" };

type Store = {
  info: AppInfo | null;
  status: ServerStatus | null;
  settings: Settings | null;
  config: IniDoc | null;
  slots: CarSlot[];
  content: ContentReport | null;
  profiles: ProfileIndex | null;
  adapters: Adapter[];
  logs: LogEntry[];
  toasts: Toast[];
  busy: string | null;
  refreshAll: () => Promise<void>;
  refreshStatus: () => Promise<void>;
  refreshConfig: () => Promise<void>;
  refreshSlots: () => Promise<void>;
  refreshContent: () => Promise<void>;
  refreshAdapters: () => Promise<void>;
  refreshLogs: () => Promise<void>;
  setSettings: (settings: Settings) => void;
  switchProfile: (id: string) => Promise<void>;
  notify: (text: string, tone?: Toast["tone"]) => void;
  dismiss: (id: number) => void;
  run: <T>(label: string, task: () => Promise<T>) => Promise<T | undefined>;
};

const AppContext = createContext<Store | null>(null);

export function AppProvider({ children }: { children: React.ReactNode }) {
  const [info, setInfo] = useState<AppInfo | null>(null);
  const [status, setStatus] = useState<ServerStatus | null>(null);
  const [settings, setSettings] = useState<Settings | null>(null);
  const [config, setConfig] = useState<IniDoc | null>(null);
  const [slots, setSlots] = useState<CarSlot[]>([]);
  const [content, setContent] = useState<ContentReport | null>(null);
  const [profiles, setProfiles] = useState<ProfileIndex | null>(null);
  const [adapters, setAdapters] = useState<Adapter[]>([]);
  const [logs, setLogs] = useState<LogEntry[]>([]);
  const [toasts, setToasts] = useState<Toast[]>([]);
  const [busy, setBusy] = useState<string | null>(null);
  const toastId = useRef(0);

  const notify = useCallback((text: string, tone: Toast["tone"] = "info") => {
    const id = ++toastId.current;
    setToasts((prev) => [...prev.slice(-3), { id, text, tone }]);
    window.setTimeout(() => {
      setToasts((prev) => prev.filter((item) => item.id !== id));
    }, tone === "error" ? 9000 : 4200);
  }, []);

  const dismiss = useCallback((id: number) => {
    setToasts((prev) => prev.filter((item) => item.id !== id));
  }, []);

  const refreshStatus = useCallback(async () => {
    try {
      setStatus(await api.status());
    } catch {
      /* server busy or shutting down */
    }
  }, []);

  const refreshConfig = useCallback(async () => {
    setConfig(await api.readConfig());
  }, []);

  const refreshSlots = useCallback(async () => {
    setSlots(await api.readSlots());
  }, []);

  const refreshContent = useCallback(async () => {
    setContent(await api.scanContent());
  }, []);

  const refreshAdapters = useCallback(async () => {
    setAdapters(await api.adapters());
  }, []);

  const refreshLogs = useCallback(async () => {
    setLogs(await api.logs());
  }, []);

  const refreshAll = useCallback(async () => {
    const results = await Promise.allSettled([
      api.info(),
      api.getSettings(),
      api.status(),
      api.readConfig(),
      api.readSlots(),
      api.scanContent(),
      api.adapters(),
      api.logs(),
      api.listProfiles(),
    ]);
    const value = <T,>(index: number): T | null =>
      results[index].status === "fulfilled" ? (results[index].value as T) : null;

    const nextInfo = value<AppInfo>(0);
    if (nextInfo) setInfo(nextInfo);
    const nextSettings = value<Settings>(1);
    if (nextSettings) setSettings(nextSettings);
    const nextStatus = value<ServerStatus>(2);
    if (nextStatus) setStatus(nextStatus);
    const nextConfig = value<IniDoc>(3);
    if (nextConfig) setConfig(nextConfig);
    const nextSlots = value<CarSlot[]>(4);
    if (nextSlots) setSlots(nextSlots);
    const nextContent = value<ContentReport>(5);
    if (nextContent) setContent(nextContent);
    const nextAdapters = value<Adapter[]>(6);
    if (nextAdapters) setAdapters(nextAdapters);
    const nextLogs = value<LogEntry[]>(7);
    if (nextLogs) setLogs(nextLogs);
    const nextProfiles = value<ProfileIndex>(8);
    if (nextProfiles) setProfiles(nextProfiles);

    const firstError = results.find((item) => item.status === "rejected");
    if (firstError && firstError.status === "rejected") {
      notify(String(firstError.reason), "error");
    }
  }, [notify]);

  const switchProfile = useCallback(
    async (id: string) => {
      try {
        const next = await api.setActiveProfile(id);
        setProfiles(next);
        await refreshAll();
      } catch (error) {
        notify(String(error), "error");
      }
    },
    [notify, refreshAll],
  );

  const run = useCallback(
    async <T,>(label: string, task: () => Promise<T>): Promise<T | undefined> => {
      setBusy(label);
      try {
        return await task();
      } catch (error) {
        notify(String(error), "error");
        return undefined;
      } finally {
        setBusy(null);
      }
    },
    [notify],
  );

  useEffect(() => {
    void refreshAll();
    const timer = window.setInterval(() => {
      void refreshStatus();
    }, 5000);
    return () => window.clearInterval(timer);
  }, [refreshAll, refreshStatus]);

  useEffect(() => {
    const subscription = listen<LogEntry>("server-log", (event) => {
      setLogs((prev) => {
        const next = [...prev, event.payload];
        return next.length > 2000 ? next.slice(-2000) : next;
      });
    });
    return () => {
      void subscription.then((unlisten) => unlisten());
    };
  }, []);

  useEffect(() => {
    const subscription = listen("entry-list-changed", () => {
      void refreshSlots();
    });
    return () => {
      void subscription.then((unlisten) => unlisten());
    };
  }, [refreshSlots]);

  const store = useMemo<Store>(
    () => ({
      info,
      status,
      settings,
      config,
      slots,
      content,
      profiles,
      adapters,
      logs,
      toasts,
      busy,
      refreshAll,
      refreshStatus,
      refreshConfig,
      refreshSlots,
      refreshContent,
      refreshAdapters,
      refreshLogs,
      setSettings,
      switchProfile,
      notify,
      dismiss,
      run,
    }),
    [
      info,
      status,
      settings,
      config,
      slots,
      content,
      profiles,
      adapters,
      logs,
      toasts,
      busy,
      refreshAll,
      refreshStatus,
      refreshConfig,
      refreshSlots,
      refreshContent,
      refreshAdapters,
      refreshLogs,
      switchProfile,
      notify,
      dismiss,
      run,
    ],
  );

  return <AppContext.Provider value={store}>{children}</AppContext.Provider>;
}

export function useApp(): Store {
  const store = useContext(AppContext);
  if (!store) throw new Error("useApp fuori dal provider");
  return store;
}

export function useConfigValue(section: string, key: string): string {
  const { config } = useApp();
  const found = config?.sections
    .find((item) => item.name.toLowerCase() === section.toLowerCase())
    ?.entries.find((entry) => entry.key.toUpperCase() === key.toUpperCase());
  return found?.value ?? "";
}
