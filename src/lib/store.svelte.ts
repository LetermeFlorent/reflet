import { api, listen, type UnlistenFn } from "./ipc";
import type { Settings, SyncPair } from "./types";

export interface Toast {
  id: number;
  kind: "info" | "error" | "success";
  message: string;
}

export interface Progress {
  pairId: string;
  done: number;
  total: number;
}

function shallowEq(a: unknown, b: unknown): boolean {
  if (a === b) return true;
  if (a == null || b == null) return false;
  if (Array.isArray(a) && Array.isArray(b)) {
    return a.length === b.length && a.every((v, i) => v === b[i]);
  }
  if (typeof a === "object" && typeof b === "object") {
    const ka = Object.keys(a as object);
    const kb = Object.keys(b as object);
    if (ka.length !== kb.length) return false;
    return ka.every(
      (k) => (a as Record<string, unknown>)[k] === (b as Record<string, unknown>)[k],
    );
  }
  return false;
}

class Store {
  settings = $state<Settings | null>(null);
  pairs = $state<SyncPair[]>([]);
  schedulerRunning = $state(true);
  syncBusy = $state(false);
  progress = $state<Progress | null>(null);
  toasts = $state<Toast[]>([]);
  loaded = $state(false);

  private toastSeq = 1;

  async refresh() {
    const s = await api.getAppState();
    this.settings = s.settings;
    this.mergePairs(s.pairs);
    this.schedulerRunning = s.schedulerRunning;
    this.syncBusy = s.syncBusy;
    this.loaded = true;
  }

  private mergePairs(incoming: SyncPair[]) {
    const byId = new Map(this.pairs.map((p) => [p.id, p]));
    const merged = incoming.map((np) => {
      const existing = byId.get(np.id);
      if (!existing) return np;
      const ex = existing as unknown as Record<string, unknown>;
      const src = np as unknown as Record<string, unknown>;
      for (const k of Object.keys(np)) {
        if (!shallowEq(ex[k], src[k])) ex[k] = src[k];
      }
      return existing;
    });
    const sameOrder =
      merged.length === this.pairs.length && merged.every((p, i) => p === this.pairs[i]);
    if (!sameOrder) this.pairs = merged;
  }

  toast(kind: Toast["kind"], message: string) {
    const id = this.toastSeq++;
    this.toasts = [...this.toasts, { id, kind, message }];
    setTimeout(() => this.dismiss(id), kind === "error" ? 7000 : 3500);
  }

  dismiss(id: number) {
    this.toasts = this.toasts.filter((t) => t.id !== id);
  }

  pairName(id?: string | null): string {
    if (!id) return "";
    return this.pairs.find((p) => p.id === id)?.name ?? id;
  }

  async initListeners(): Promise<UnlistenFn> {
    const unlisten: UnlistenFn[] = [];

    unlisten.push(await listen("state:changed", () => this.refresh()));

    unlisten.push(
      await listen<{ busy: boolean }>("sync:busy", (e) => {
        this.syncBusy = e.payload.busy;
        if (!e.payload.busy) this.progress = null;
      }),
    );

    unlisten.push(
      await listen<Progress>("sync:progress", (e) => {
        this.progress = e.payload;
      }),
    );

    unlisten.push(
      await listen<any>("sync:started", (e) => {
        this.progress = { pairId: e.payload.pairId, done: 0, total: e.payload.totalFiles };
      }),
    );

    unlisten.push(
      await listen<any>("sync:finished", (e) => {
        const p = e.payload;
        this.progress = null;
        const pair = this.pairs.find((x) => x.id === p.pairId);
        if (this.settings?.notifyApp && pair?.notifyApp) {
          if (p.status === "ok") {
            this.toast(
              "success",
              `${p.name} : ${p.copied} copiés, ${p.updated} màj, ${p.deleted} supprimés`,
            );
          } else {
            this.toast("error", `${p.name} : terminé avec erreurs`);
          }
        }
        this.refresh();
      }),
    );

    unlisten.push(
      await listen<{ pairId?: string; message: string }>("app:error", (e) => {
        this.toast("error", e.payload.message);
      }),
    );

    return () => unlisten.forEach((u) => u());
  }
}

export const store = new Store();
