<script lang="ts">
  import { store } from "$lib/store.svelte";
  import { api } from "$lib/ipc";
  import ExclusionsManager from "$lib/components/ExclusionsManager.svelte";

  const patterns = $derived(store.settings?.ignorePatterns ?? []);
  let busy = $state(false);
  let mgr = $state<{ openAdd: () => void } | undefined>(undefined);

  async function persist(next: string[]) {
    if (!store.settings) return;
    busy = true;
    try {
      await api.updateSettings({ ...store.settings, ignorePatterns: next });
      await store.refresh();
    } catch (e) {
      store.toast("error", String(e));
    } finally {
      busy = false;
    }
  }

  const onAdd = (ps: string[]) => {
    const next = patterns.slice();
    for (const p of ps) if (!next.includes(p)) next.push(p);
    persist(next);
  };
  const onReplace = (i: number, ps: string[]) => {
    const next = patterns.slice();
    next.splice(i, 1, ...ps);
    persist(next);
  };
  const onRemove = (i: number) => persist(patterns.filter((_, j) => j !== i));
</script>

<div class="page-scroll">
  <div class="head-row">
    <div class="head-text">
      <h1>Exclusions globales</h1>
      <p class="muted sub">
        Motifs appliqués à toutes les paires. Les dossiers et fichiers correspondants ne sont jamais
        copiés vers la destination ni supprimés. Chaque paire peut aussi avoir ses propres exclusions
        (dans « Modifier la paire »).
      </p>
    </div>
    <button class="btn" onclick={() => mgr?.openAdd()} disabled={busy}>+ Ajouter</button>
  </div>

  <ExclusionsManager bind:this={mgr} {patterns} {onAdd} {onReplace} {onRemove} {busy} showAddButton={false} />
</div>

<style>
  .head-row {
    display: flex;
    align-items: flex-end;
    gap: var(--s4);
    margin-bottom: var(--s4);
  }
  .head-text {
    flex: 1;
    min-width: 0;
  }
  .head-text h1 {
    margin-bottom: 4px;
  }
  .head-row .btn {
    flex: 0 0 auto;
  }
  .sub {
    font-size: 12px;
    max-width: 640px;
    line-height: 1.55;
    margin: 0;
  }
</style>
