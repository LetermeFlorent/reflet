import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";
import type { AppStateDto, LogEntry, NewPair, Settings, SyncPair, SyncPlan } from "./types";

export const api = {
  getAppState: () => invoke<AppStateDto>("get_app_state"),
  getSettings: () => invoke<Settings>("get_settings"),
  updateSettings: (settings: Settings) => invoke<void>("update_settings", { settings }),
  setSchedulerRunning: (running: boolean) =>
    invoke<void>("set_scheduler_running", { running }),

  addPair: (p: NewPair) => invoke<string>("add_pair", { new: p }),
  updatePair: (pair: SyncPair) => invoke<void>("update_pair", { pair }),
  deletePair: (id: string) => invoke<void>("delete_pair", { id }),
  setPairEnabled: (id: string, enabled: boolean) =>
    invoke<void>("set_pair_enabled", { id, enabled }),
  setPairWatchRealtime: (id: string, watch: boolean) =>
    invoke<void>("set_pair_watch_realtime", { id, watch }),
  reorderPairs: (orderedIds: string[]) =>
    invoke<void>("reorder_pairs", { orderedIds }),

  syncNow: (id: string) => invoke<void>("sync_now", { id }),
  syncAll: () => invoke<void>("sync_all"),
  dryRun: (id: string) => invoke<SyncPlan>("dry_run", { id }),

  getLogs: () => invoke<LogEntry[]>("get_logs"),
  clearLogs: () => invoke<void>("clear_logs"),

  showWindow: () => invoke<void>("show_window"),
  hideWindow: () => invoke<void>("hide_window"),
  quitApp: () => invoke<void>("quit_app"),

  pickFolder: async (): Promise<string | null> => {
    const r = await open({ directory: true, multiple: false });
    return typeof r === "string" ? r : null;
  },

  pickFiles: async (): Promise<string[]> => {
    const r = await open({ multiple: true });
    if (!r) return [];
    return Array.isArray(r) ? r : [r];
  },

  pickFolders: async (): Promise<string[]> => {
    const r = await open({ directory: true, multiple: true });
    if (!r) return [];
    return Array.isArray(r) ? r : [r];
  },
};

export { listen };
export type { UnlistenFn };
