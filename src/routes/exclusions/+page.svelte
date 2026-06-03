<script lang="ts">
  import { store } from "$lib/store.svelte";
  import { api } from "$lib/ipc";
  import ExclusionsManager from "$lib/components/ExclusionsManager.svelte";

  const patterns = $derived(store.settings?.ignorePatterns ?? []);
  let busy = $state(false);

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
  <header class="page-head"><h1>Exclusions globales</h1></header>
  <p class="muted sub">
    Motifs appliqués à toutes les paires. Les dossiers et fichiers correspondants ne sont jamais
    copiés vers la destination ni supprimés. Chaque paire peut aussi avoir ses propres exclusions
    (dans « Modifier la paire »).
  </p>

  <ExclusionsManager {patterns} {onAdd} {onReplace} {onRemove} {busy} />
</div>

<style>
  .page-head {
    margin-bottom: var(--s2);
  }
  .sub {
    font-size: 12px;
    max-width: 640px;
    margin-bottom: var(--s4);
    line-height: 1.55;
  }
</style>
