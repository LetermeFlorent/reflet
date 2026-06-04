<script lang="ts">
  import type { SyncPair } from "$lib/types";
  import { api } from "$lib/ipc";
  import { store } from "$lib/store.svelte";
  import CardHeader from "./CardHeader.svelte";
  import CardStatusLine from "./CardStatusLine.svelte";
  import SyncButton from "./SyncButton.svelte";

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
  const compact = $derived(store.settings?.compactCards ?? false);

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

<div
  class="card pair"
  class:tinted={!!pair.color}
  style={pair.color ? `--card-glow:var(--color-${pair.color}-glow)` : ""}
>
  <CardHeader {pair} onToggle={toggle} />

  <div class="paths mono">
    <span class="path" title={pair.source}>{pair.source}</span>
    <span class="arrow">→</span>
    <span class="path" title={pair.destination}>{pair.destination}</span>
  </div>

  <div class="bottom">
    <CardStatusLine {pair} {isSyncing} {pct} {indeterminate} {compact} />
    <div class="spacer"></div>
    <div class="actions">
      <button class="btn btn-sm" onclick={onDryRun}>Aperçu</button>
      <SyncButton {isSyncing} {pct} {indeterminate} onclick={syncNow} />
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
  /* Accent couleur moderne : barre verticale a gauche + leger degrade qui s'estompe. */
  .pair.tinted {
    position: relative;
    overflow: hidden;
    background:
      linear-gradient(
        to right,
        color-mix(in srgb, var(--card-glow) 14%, transparent),
        transparent 45%
      ),
      var(--bg-elev);
  }
  .pair.tinted::before {
    content: "";
    position: absolute;
    left: 0;
    top: 0;
    bottom: 0;
    width: 4px;
    background: var(--card-glow);
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
  .actions {
    display: flex;
    gap: 6px;
  }
</style>
