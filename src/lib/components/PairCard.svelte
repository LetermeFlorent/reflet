<script lang="ts">
  import type { SyncPair } from "$lib/types";
  import { api } from "$lib/ipc";
  import { store } from "$lib/store.svelte";
  import { formatDate, formatInterval } from "$lib/format";
  import StatusBadge from "./StatusBadge.svelte";
  import Switch from "./Switch.svelte";

  let {
    pair,
    onEdit,
    onDryRun,
    onDelete,
  }: { pair: SyncPair; onEdit: () => void; onDryRun: () => void; onDelete: () => void } = $props();

  const isSyncing = $derived(store.progress?.pairId === pair.id);
  const pct = $derived(
    isSyncing && store.progress && store.progress.total > 0
      ? Math.min(100, Math.round((store.progress.done / store.progress.total) * 100))
      : 0,
  );
  const indeterminate = $derived(isSyncing && (!store.progress || store.progress.total === 0));

  let etaSec = $state<number | null>(null);
  let startAt = 0;

  $effect(() => {
    if (!isSyncing) {
      etaSec = null;
      startAt = 0;
      return;
    }
    if (startAt === 0) startAt = Date.now();
    const tick = () => {
      const p = store.progress;
      if (!p || p.total <= 0 || p.done <= 0) {
        etaSec = null;
        return;
      }
      const elapsed = (Date.now() - startAt) / 1000;
      if (elapsed <= 0.5) {
        etaSec = null;
        return;
      }
      const rate = p.done / elapsed;
      etaSec = Math.max(0, Math.round((p.total - p.done) / rate));
    };
    tick();
    const id = setInterval(tick, 1000);
    return () => clearInterval(id);
  });

  function formatDur(s: number): string {
    if (s < 60) return `${s} s`;
    const m = Math.floor(s / 60);
    const ss = s % 60;
    return ss ? `${m} min ${ss} s` : `${m} min`;
  }

  let nextSec = $state<number | null>(null);

  $effect(() => {
    const ns = pair.nextRunSec;
    if (ns == null) {
      nextSec = null;
      return;
    }
    const targetMs = Date.now() + ns * 1000;
    const tick = () => {
      nextSec = Math.max(0, Math.round((targetMs - Date.now()) / 1000));
    };
    tick();
    const id = setInterval(tick, 1000);
    return () => clearInterval(id);
  });

  const intervalLabel = $derived.by(() => {
    const ov = pair.intervalSecOverride;
    if (ov != null && ov > 0) return formatInterval(ov);
    const g = store.settings?.intervalSec ?? 900;
    return `${formatInterval(g)} (défaut)`;
  });

  async function toggle(enabled: boolean) {
    try {
      await api.setPairEnabled(pair.id, enabled);
    } catch (err) {
      store.toast("error", String(err));
    }
  }

  async function syncNow() {
    try {
      await api.syncNow(pair.id);
      store.toast("info", `Synchro lancée : ${pair.name}`);
    } catch (err) {
      store.toast("error", String(err));
    }
  }
</script>

<div class="card pair">
  <div class="top">
    <div class="title">
      <h2>{pair.name}</h2>
      <StatusBadge status={pair.status} />
    </div>
    <span class="interval badge" title="Intervalle de synchro automatique">
      <svg width="13" height="13" viewBox="0 0 16 16" fill="none">
        <circle cx="8" cy="8.5" r="5.5" stroke="currentColor" stroke-width="1.3" />
        <path d="M8 5.5V8.5L10 10" stroke="currentColor" stroke-width="1.3" stroke-linecap="round" />
        <path d="M6 1.5h4" stroke="currentColor" stroke-width="1.3" stroke-linecap="round" />
      </svg>
      {intervalLabel}
    </span>
    <div class="spacer"></div>
    <Switch checked={pair.enabled} onchange={toggle} />
  </div>

  <div class="paths mono">
    <span class="path" title={pair.source}>{pair.source}</span>
    <span class="arrow">→</span>
    <span class="path" title={pair.destination}>{pair.destination}</span>
  </div>

  <div class="bottom">
    <div class="last muted">
      {#if isSyncing}
        <span class="sync-status">
          Synchro… {indeterminate ? "…" : pct + " %"}{#if etaSec !== null} · ~{formatDur(etaSec)} restant{/if}
        </span>
      {:else}
        {#if pair.lastRun}
          Dernière : {formatDate(pair.lastRun.at)} ·
          <span style="color:var(--green)">{pair.lastRun.copied}</span> copiés ·
          <span style="color:var(--accent)">{pair.lastRun.updated}</span> màj ·
          <span style="color:var(--red)">{pair.lastRun.deleted}</span> suppr.
          {#if pair.lastRun.errors > 0}
            · <span style="color:var(--red)">{pair.lastRun.errors} err.</span>
          {/if}
        {:else}
          Jamais synchronisé
        {/if}
        {#if nextSec !== null}
          <span class="next">· prochaine dans {formatDur(nextSec)}</span>
        {/if}
      {/if}
    </div>
    <div class="spacer"></div>
    <div class="actions">
      <button class="btn btn-sm" onclick={onDryRun}>Aperçu</button>
      <button
        class="btn btn-sm sync-btn"
        class:syncing={isSyncing}
        onclick={syncNow}
        disabled={isSyncing}
      >
        {#if isSyncing}
          <span class="fill" class:indet={indeterminate} style="width:{pct}%"></span>
        {/if}
        <span class="lbl">{isSyncing ? (indeterminate ? "…" : pct + "%") : "Synchroniser"}</span>
      </button>
      <button class="btn btn-sm btn-ghost btn-icon" title="Modifier" aria-label="Modifier" onclick={onEdit}>
        <svg width="15" height="15" viewBox="0 0 16 16" fill="none">
          <path d="M11.5 2.5l2 2L6 12l-2.5.5L4 10l7.5-7.5z" stroke="currentColor" stroke-width="1.3" stroke-linejoin="round" />
        </svg>
      </button>
      <button class="btn btn-sm btn-ghost btn-icon btn-danger" title="Supprimer" aria-label="Supprimer" onclick={onDelete}>
        <svg width="15" height="15" viewBox="0 0 16 16" fill="none">
          <path d="M3 4h10M6.5 4V2.8h3V4M5 4l.6 9h4.8L11 4" stroke="currentColor" stroke-width="1.3" stroke-linecap="round" stroke-linejoin="round" />
        </svg>
      </button>
    </div>
  </div>
</div>

<style>
  .pair {
    padding: var(--s4);
    display: flex;
    flex-direction: column;
    gap: var(--s3);
  }
  .sync-btn {
    position: relative;
    overflow: hidden;
    min-width: 112px;
  }
  .sync-btn.syncing {
    border-color: var(--accent);
    color: var(--accent);
  }
  .sync-btn .fill {
    position: absolute;
    left: 0;
    top: 0;
    bottom: 0;
    width: 0;
    background: color-mix(in srgb, var(--accent) 22%, transparent);
    transition: width 0.25s var(--ease);
  }
  .sync-btn .fill.indet {
    width: 45% !important;
    animation: indet 1.1s ease-in-out infinite;
  }
  @keyframes indet {
    0% {
      transform: translateX(-110%);
    }
    100% {
      transform: translateX(260%);
    }
  }
  .sync-btn .lbl {
    position: relative;
    z-index: 1;
  }
  .top {
    display: flex;
    align-items: center;
    gap: var(--s3);
  }
  .title {
    display: flex;
    align-items: center;
    gap: var(--s3);
    min-width: 0;
  }
  .title h2 {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .paths {
    display: flex;
    align-items: center;
    gap: var(--s2);
    color: var(--text-2);
  }
  .path {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    max-width: 42%;
  }
  .arrow {
    color: var(--accent);
    flex: 0 0 auto;
  }
  .bottom {
    display: flex;
    align-items: center;
    gap: var(--s3);
    flex-wrap: wrap;
  }
  .last {
    font-size: 12px;
  }
  .sync-status {
    color: var(--accent);
    font-weight: 600;
  }
  .next {
    margin-left: 4px;
    color: var(--text-2);
  }
  .actions {
    display: flex;
    gap: 6px;
  }
</style>
