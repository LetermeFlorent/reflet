<script lang="ts">
  import type { SyncPair } from "$lib/types";
  import { store } from "$lib/store.svelte";
  import { formatInterval } from "$lib/format";
  import StatusBadge from "./StatusBadge.svelte";
  import Switch from "./Switch.svelte";

  let { pair, onToggle }: { pair: SyncPair; onToggle: (enabled: boolean) => void } = $props();

  const intervalLabel = $derived.by(() => {
    if (pair.scheduleTimes?.length > 0) return pair.scheduleTimes.join(", ");
    const ov = pair.intervalSecOverride;
    if (ov != null && ov > 0) return formatInterval(ov);
    const g = store.settings?.intervalSec ?? 900;
    return `${formatInterval(g)} (défaut)`;
  });
  const intervalTitle = $derived(
    pair.scheduleTimes?.length > 0
      ? "Planifié à des horaires spécifiques"
      : "Intervalle de synchro automatique",
  );
</script>

<div class="top">
  <div class="title">
    <h2>{pair.name}</h2>
    <StatusBadge status={pair.status} />
  </div>
  {#if pair.watchRealtime && pair.enabled}
    <span class="badge live" title="Surveillance temps réel active">● Live</span>
  {/if}
  {#if pair.backupMode}
    <span class="badge backup" title="Mode sauvegarde : copies horodatées">⧉ Sauvegarde</span>
  {/if}
  {#if pair.compression && pair.compression.method !== "off"}
    <span class="badge comp" title="Archive compressée ({pair.compression.method})">
      {pair.compression.method}{pair.compression.level > 0 ? `-${pair.compression.level}` : ""}
    </span>
  {/if}
  <span class="interval badge" title={intervalTitle}>
    <svg width="13" height="13" viewBox="0 0 16 16" fill="none">
      <circle cx="8" cy="8.5" r="5.5" stroke="currentColor" stroke-width="1.3" />
      <path d="M8 5.5V8.5L10 10" stroke="currentColor" stroke-width="1.3" stroke-linecap="round" />
      <path d="M6 1.5h4" stroke="currentColor" stroke-width="1.3" stroke-linecap="round" />
    </svg>
    {intervalLabel}
  </span>
  <div class="spacer"></div>
  <Switch checked={pair.enabled} onchange={onToggle} />
</div>

<style>
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
  .live {
    background: color-mix(in srgb, var(--green) 18%, transparent);
    color: var(--green);
  }
  .backup {
    background: color-mix(in srgb, var(--orange) 18%, transparent);
    color: var(--orange);
  }
  .comp {
    background: color-mix(in srgb, var(--accent) 18%, transparent);
    color: var(--accent);
  }
</style>
