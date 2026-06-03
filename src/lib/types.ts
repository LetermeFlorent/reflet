export interface LastRun {
  at: string;
  status: string;
  copied: number;
  updated: number;
  deleted: number;
  errors: number;
}

export interface SyncPair {
  id: string;
  name: string;
  source: string;
  destination: string;
  enabled: boolean;
  intervalSecOverride?: number | null;
  notifyPc: boolean;
  notifyApp: boolean;
  ignorePatterns: string[];
  lastRun?: LastRun | null;
  status: string;
  nextRunSec?: number | null;
}

export interface NewPair {
  name: string;
  source: string;
  destination: string;
  enabled: boolean;
  intervalSecOverride?: number | null;
  notifyPc: boolean;
  notifyApp: boolean;
  ignorePatterns: string[];
}

export interface Settings {
  intervalSec: number;
  deleteBehavior: "trash" | "permanent";
  autostart: boolean;
  startMinimized: boolean;
  confirmDeletesWithDryRun: boolean;
  ignorePatterns: string[];
  verifyByContent: "off" | "blake3";
  mtimeToleranceSec: number;
  deleteSafetyThresholdPct: number;
  schedulerRunning: boolean;
  notifyPc: boolean;
  notifyApp: boolean;
  compactCards: boolean;
}

export interface AppStateDto {
  settings: Settings;
  pairs: SyncPair[];
  schedulerRunning: boolean;
  syncBusy: boolean;
}

export interface PlanItem {
  rel: string;
  isDir: boolean;
  size: number;
  reason: string;
}

export interface SyncPlan {
  toCreateDir: string[];
  toCopy: PlanItem[];
  toOverwrite: PlanItem[];
  toDelete: PlanItem[];
  totalBytes: number;
  deletePct: number;
  abortedSafety: boolean;
}

export interface LogEntry {
  at: string;
  level: string;
  pairId?: string | null;
  action: string;
  path?: string | null;
  message: string;
}
