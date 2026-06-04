<script lang="ts">
  import type { SyncPair } from "$lib/types";
  import { store } from "$lib/store.svelte";
  import { formatDate } from "$lib/format";

  let { pair, isSyncing, pct, indeterminate, compact }: {
    pair: SyncPair;
    isSyncing: boolean;
    pct: number;
    indeterminate: boolean;
    compact: boolean;
  } = $props();

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

  let nextSec = $state<number | null>(null);

  $effect(() => {
    void store.rev;
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

  function formatDur(s: number): string {
    if (s < 60) return `${s} s`;
    const m = Math.floor(s / 60);
    const ss = s % 60;
    return ss ? `${m} min ${ss} s` : `${m} min`;
  }
</script>

<div class="last muted">
  {#if isSyncing}
    <span class="sync-status">
      Synchro… {indeterminate ? "…" : pct + " %"}{#if etaSec !== null} · ~{formatDur(etaSec)} restant{/if}
    </span>
  {:else}
    {#if compact}
      {pair.lastRun ? (pair.lastRun.errors > 0 ? "Synchro avec erreurs" : "À jour") : "Jamais synchronisé"}
    {:else if pair.lastRun}
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
      <span class="next">· {nextSec <= 0 ? "synchro imminente" : `prochaine dans ${formatDur(nextSec)}`}</span>
    {/if}
  {/if}
</div>

<style>
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
</style>
