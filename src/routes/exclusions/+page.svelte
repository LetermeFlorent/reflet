<script lang="ts">
  import { store } from "$lib/store.svelte";
  import { api } from "$lib/ipc";
  import { confirmCtl } from "$lib/confirm.svelte";
  import ExclusionAddModal from "$lib/components/ExclusionAddModal.svelte";

  const patterns = $derived(store.settings?.ignorePatterns ?? []);
  let busy = $state(false);
  let modalOpen = $state(false);
  let editIdx = $state<number | null>(null);

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

  function openAdd() {
    editIdx = null;
    modalOpen = true;
  }
  function openEdit(i: number) {
    editIdx = i;
    modalOpen = true;
  }

  function onSubmit(ps: string[]) {
    const next = patterns.slice();
    if (editIdx !== null) {
      next.splice(editIdx, 1, ...ps);
    } else {
      for (const p of ps) if (!next.includes(p)) next.push(p);
    }
    persist(next);
  }

  async function remove(i: number) {
    const ok = await confirmCtl.ask(`Supprimer l'exclusion « ${patterns[i]} » ?`, {
      title: "Supprimer l'exclusion",
      confirmLabel: "Supprimer",
      danger: true,
    });
    if (ok) persist(patterns.filter((_, j) => j !== i));
  }

  function kind(p: string): string {
    if (p.endsWith("/**")) return "Dossier";
    if (/\*\.[^/]+$/.test(p)) return "Type de fichier";
    if (p.includes("*")) return "Motif";
    return "Fichier / chemin";
  }
</script>

<div class="page-scroll">
  <header class="page-head">
    <h1>Exclusions globales</h1>
    <div class="spacer"></div>
    <button class="btn" onclick={openAdd} disabled={busy}>+ Ajouter</button>
  </header>
  <p class="muted sub">
    Motifs appliqués à toutes les paires. Les dossiers et fichiers correspondants ne sont jamais
    copiés vers la destination ni supprimés.
  </p>

  {#if patterns.length === 0}
    <p class="muted empty">Aucune exclusion globale. Clique sur « + Ajouter ».</p>
  {:else}
    <table class="grid">
      <thead>
        <tr><th class="c-type">Type</th><th>Motif</th><th class="c-act">Actions</th></tr>
      </thead>
      <tbody>
        {#each patterns as p, i (p + i)}
          <tr>
            <td class="c-type">{kind(p)}</td>
            <td><code>{p}</code></td>
            <td class="c-act">
              <button class="btn btn-sm" onclick={() => openEdit(i)} disabled={busy}>Modifier</button>
              <button class="btn btn-sm btn-danger" onclick={() => remove(i)} disabled={busy}>Supprimer</button>
            </td>
          </tr>
        {/each}
      </tbody>
    </table>
  {/if}
</div>

{#if modalOpen}
  <ExclusionAddModal
    initial={editIdx !== null ? patterns[editIdx] : ""}
    onClose={() => (modalOpen = false)}
    onSubmit={onSubmit}
  />
{/if}

<style>
  .page-head {
    display: flex;
    align-items: center;
    margin-bottom: var(--s2);
  }
  .sub {
    font-size: 12px;
    max-width: 640px;
    margin-bottom: var(--s4);
    line-height: 1.55;
  }
  .empty {
    font-size: 13px;
  }
  .grid {
    width: 100%;
    border-collapse: collapse;
    background: var(--bg-elev);
    border: 1px solid var(--hairline);
    border-radius: var(--r-card);
    overflow: hidden;
  }
  .grid th,
  .grid td {
    text-align: left;
    padding: 8px 12px;
    border-bottom: 1px solid var(--hairline);
    font-size: 13px;
  }
  .grid th {
    font-size: 12px;
    font-weight: 600;
    color: var(--text-2);
    background: var(--bg-sunken);
  }
  .grid tbody tr:last-child td {
    border-bottom: none;
  }
  .c-type {
    width: 140px;
    color: var(--text-2);
  }
  .c-act {
    width: 1%;
    white-space: nowrap;
    text-align: right;
  }
  .c-act .btn {
    margin-left: 6px;
  }
  .grid code {
    font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
    font-size: 12px;
  }
</style>
