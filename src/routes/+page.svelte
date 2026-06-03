<script lang="ts">
  import { store } from "$lib/store.svelte";
  import { api } from "$lib/ipc";
  import { confirmCtl } from "$lib/confirm.svelte";
  import type { SyncPair, SyncPlan } from "$lib/types";
  import PairCard from "$lib/components/PairCard.svelte";
  import PairEditModal from "$lib/components/PairEditModal.svelte";
  import DryRunModal from "$lib/components/DryRunModal.svelte";

  let editing = $state<SyncPair | null | undefined>(undefined);

  let dryPair = $state<SyncPair | null>(null);
  let dryPlan = $state<SyncPlan | null>(null);
  let dryLoading = $state(false);

  let searchQuery = $state("");
  let dragIndex = $state<number | null>(null);

  const filteredPairs = $derived.by(() => {
    const q = searchQuery.toLowerCase().trim();
    if (!q) return store.pairs;
    return store.pairs.filter(
      (p) =>
        p.name.toLowerCase().includes(q) ||
        p.source.toLowerCase().includes(q) ||
        p.destination.toLowerCase().includes(q),
    );
  });

  function dragStart(e: DragEvent, i: number) {
    dragIndex = i;
    if (e.dataTransfer) e.dataTransfer.effectAllowed = "move";
  }
  function dragOver(e: DragEvent, i: number) {
    e.preventDefault();
    if (dragIndex === null || dragIndex === i) return;
    const arr = [...store.pairs];
    const [moved] = arr.splice(dragIndex, 1);
    arr.splice(i, 0, moved);
    store.pairs = arr;
    dragIndex = i;
  }
  function dragEnd() {
    dragIndex = null;
    api.reorderPairs(store.pairs.map((p) => p.id)).catch(() => {});
  }

  async function confirmDelete(p: SyncPair) {
    const ok = await confirmCtl.ask(
      `Supprimer la paire « ${p.name} » ? Les dossiers source et destination ne sont pas touchés.`,
      { title: "Supprimer la paire", confirmLabel: "Supprimer", danger: true },
    );
    if (!ok) return;
    try {
      await api.deletePair(p.id);
      store.toast("info", "Paire supprimée");
    } catch (e) {
      store.toast("error", String(e));
    }
  }

  async function openDry(p: SyncPair) {
    dryPair = p;
    dryPlan = null;
    dryLoading = true;
    try {
      dryPlan = await api.dryRun(p.id);
    } catch (e) {
      store.toast("error", String(e));
      dryPair = null;
    } finally {
      dryLoading = false;
    }
  }
  function closeDry() {
    dryPair = null;
    dryPlan = null;
  }
  async function applyDry() {
    if (!dryPair) return;
    try {
      await api.syncNow(dryPair.id);
      store.toast("info", `Synchro lancée : ${dryPair.name}`);
    } catch (e) {
      store.toast("error", String(e));
    }
    closeDry();
  }

  async function toggleScheduler() {
    try {
      await api.setSchedulerRunning(!store.schedulerRunning);
    } catch (e) {
      store.toast("error", String(e));
    }
  }

  async function syncAll() {
    try {
      await api.syncAll();
      store.toast("info", "Synchronisation de toutes les paires lancée");
    } catch (e) {
      store.toast("error", String(e));
    }
  }
</script>

<div class="dash">
  <header class="page-head">
    <h1>Tableau de bord</h1>
    <div class="search-wrap">
      <input class="input search-input" bind:value={searchQuery} placeholder="Rechercher une paire…" />
    </div>
    <div class="spacer"></div>
    <button class="btn btn-sm" onclick={toggleScheduler}>
      {store.schedulerRunning ? "Mettre en pause" : "Reprendre"}
    </button>
    <button class="btn btn-sm" onclick={syncAll} disabled={store.syncBusy || store.pairs.length === 0}>
      Tout synchroniser
    </button>
    <button class="btn btn-sm" onclick={() => (editing = null)}>+ Ajouter</button>
  </header>

  <div class="dash-scroll">
    {#if !store.loaded}
      <p class="muted">Chargement…</p>
    {:else if store.pairs.length === 0}
      <div class="empty-wrap">
        <div class="card empty">
          <div class="empty-logo">◑</div>
          <h2>Aucune paire de synchronisation</h2>
          <p class="muted">Ajoute un dossier source et sa destination pour commencer.</p>
          <button class="btn" onclick={() => (editing = null)}>Ajouter une paire</button>
        </div>
      </div>
    {:else if filteredPairs.length === 0 && searchQuery}
      <div class="empty-wrap">
        <div class="card empty">
          <div class="empty-logo">◑</div>
          <h2>Aucun résultat</h2>
          <p class="muted">Aucune paire ne correspond à « {searchQuery} ».</p>
        </div>
      </div>
    {:else}
      <div class="list">
        {#each filteredPairs as p, i (p.id)}
          <div
            class="pair-drag"
            draggable="true"
            role="listitem"
            ondragstart={(e) => dragStart(e, i)}
            ondragover={(e) => dragOver(e, i)}
            ondragend={dragEnd}
          >
            <span class="drag-handle" title="Déplacer pour réordonner">⠿</span>
            <PairCard
              pair={p}
              onEdit={() => (editing = p)}
              onDryRun={() => openDry(p)}
              onDelete={() => confirmDelete(p)}
            />
          </div>
        {/each}
      </div>
    {/if}
  </div>
</div>

{#if editing !== undefined}
  <PairEditModal
    pair={editing}
    onClose={() => (editing = undefined)}
    onSaved={() => store.refresh()}
  />
{/if}

{#if dryPair}
  <DryRunModal
    plan={dryPlan}
    pairName={dryPair.name}
    loading={dryLoading}
    onClose={closeDry}
    onConfirm={applyDry}
  />
{/if}

<style>
  .dash {
    height: 100%;
    min-height: 0;
    display: flex;
    flex-direction: column;
    padding: var(--s5) var(--s6) 0;
    animation: pageIn 0.25s var(--ease);
  }
  .page-head {
    flex: 0 0 auto;
    display: flex;
    align-items: center;
    gap: var(--s2);
    margin-bottom: var(--s4);
  }
  .dash-scroll {
    flex: 1 1 auto;
    min-height: 0;
    overflow-y: auto;
    padding-bottom: var(--s5);
  }
  .list {
    display: flex;
    flex-direction: column;
    gap: var(--s3);
  }
  .empty-wrap {
    display: grid;
    place-items: center;
    min-height: 100%;
  }
  .empty {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--s4);
    padding: var(--s6) var(--s5);
    text-align: center;
    width: 100%;
    max-width: 480px;
  }
  .empty h2 {
    font-size: 18px;
  }
  .empty-logo {
    font-size: 64px;
    line-height: 1;
    color: var(--accent);
    opacity: 0.55;
  }
  .search-wrap {
    flex: 0 0 220px;
    margin: 0 var(--s3);
  }
  .search-input {
    font-size: 13px;
    padding: 6px 10px;
  }
  .pair-drag {
    display: flex;
    align-items: stretch;
    gap: 2px;
    border-radius: var(--r-card);
    cursor: default;
  }
  .pair-drag:active {
    opacity: 0.7;
  }
  .drag-handle {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 24px;
    flex: 0 0 auto;
    cursor: grab;
    color: var(--text-3);
    font-size: 18px;
    letter-spacing: -2px;
    user-select: none;
  }
  .drag-handle:hover {
    color: var(--accent);
  }
</style>
